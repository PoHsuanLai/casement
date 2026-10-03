//! Building a request from integers: the fuzzer's way to say "send this opcode on that object
//! with boundary-ish arguments" without a typed helper. Each argument of the request's
//! signature takes the next integer (cycling) and turns it into a value of that type, chosen to
//! be awkward: null or 4 KB or non-UTF-8 strings, ids of live, destroyed and unknown objects,
//! descriptors of the wrong kind.

use super::objects::{Life, Objects};
use crate::globals::Global;
use std::os::fd::OwnedFd;
use wayland_backend::protocol::{ArgumentType, MessageDesc};

/// A request body and the descriptors that travel with it.
#[derive(Debug)]
pub(super) struct Built {
    pub payload: Vec<u8>,
    pub fds: Vec<OwnedFd>,
}

/// What a request may name.
pub(super) struct Context<'a> {
    pub objects: &'a Objects,
    pub globals: &'a [Global],
    /// The next id this client has not used; each new object takes one.
    pub next_id: &'a mut u32,
}

/// The name the scanner gives an object argument with no declared interface.
const ANONYMOUS: &str = "<anonymous>";
/// The longest string the generator makes: the protocol's own message limit is 4096 bytes.
const LONG: usize = 4096;
/// Descriptor memfds are at most this big.
const MEMFD_MAX: i32 = 65_536;

/// Builds the body of request `desc` from `ints`; with no description (an opcode the interface
/// does not have) the integers go out as raw words, which the compositor must refuse.
pub(super) fn build(desc: Option<&MessageDesc>, ints: &[i32], cx: &mut Context<'_>) -> Built {
    let mut cursor = Cursor { ints, at: 0 };
    let mut payload = Vec::new();
    let mut fds = Vec::new();
    let Some(desc) = desc else {
        for _ in 0..ints.len().min(16) {
            payload.extend(cursor.next().to_ne_bytes());
        }
        return Built { payload, fds };
    };
    // A request that binds spells out name, interface, version and id: the global is chosen once
    // and the three agree (the version may be off on purpose).
    let binds = desc.child_interface.is_none()
        && desc
            .signature
            .iter()
            .any(|kind| matches!(kind, ArgumentType::NewId));
    let chosen = if binds && !cx.globals.is_empty() {
        let at = cursor.next().unsigned_abs() as usize % cx.globals.len();
        cx.globals.get(at)
    } else {
        None
    };
    let mut uints = 0;
    let mut object_args = 0;
    for kind in desc.signature {
        match kind {
            ArgumentType::Int | ArgumentType::Fixed => {
                payload.extend(cursor.next().to_ne_bytes());
            }
            ArgumentType::Uint => {
                let value = match (chosen, uints) {
                    (Some(global), 0) => global.name,
                    (Some(global), _) => version_for(global, &mut cursor),
                    (None, _) => cursor.next() as u32,
                };
                uints += 1;
                payload.extend(value.to_ne_bytes());
            }
            ArgumentType::Str(_) => match chosen {
                Some(global) => bytes_string(&mut payload, global.interface.as_bytes()),
                None => string(&mut payload, cursor.next()),
            },
            ArgumentType::Object(_) => {
                let wanted = desc
                    .arg_interfaces
                    .get(object_args)
                    .map(|i| i.name)
                    .filter(|name| *name != ANONYMOUS);
                object_args += 1;
                payload.extend(pick_object(cx.objects, wanted, &mut cursor).to_ne_bytes());
            }
            ArgumentType::NewId => payload.extend(fresh(cx.next_id).to_ne_bytes()),
            ArgumentType::Array => array(&mut payload, cursor.next()),
            ArgumentType::Fd => {
                if let Some(fd) = descriptor(cursor.next()) {
                    fds.push(fd);
                }
            }
        }
    }
    Built { payload, fds }
}

struct Cursor<'a> {
    ints: &'a [i32],
    at: usize,
}

impl Cursor<'_> {
    fn next(&mut self) -> i32 {
        let value = self
            .ints
            .get(self.at % self.ints.len().max(1))
            .copied()
            .unwrap_or(0);
        self.at += 1;
        value
    }
}

fn fresh(next_id: &mut u32) -> u32 {
    let id = *next_id;
    *next_id = next_id.wrapping_add(1);
    id
}

fn version_for(global: &Global, cursor: &mut Cursor<'_>) -> u32 {
    match cursor.next().rem_euclid(3) {
        0 => global.version,
        1 => 1,
        _ => cursor.next() as u32,
    }
}

/// A string argument of a kind chosen by `seed`.
fn string(payload: &mut Vec<u8>, seed: i32) {
    match seed.rem_euclid(5) {
        0 => payload.extend(0u32.to_ne_bytes()),
        1 => bytes_string(payload, b""),
        2 => bytes_string(payload, &vec![b'a'; LONG]),
        3 => bytes_string(payload, &[0xff, 0xfe, 0x80]),
        _ => bytes_string(payload, b"x"),
    }
}

/// A length-prefixed, NUL-terminated, padded string.
fn bytes_string(payload: &mut Vec<u8>, text: &[u8]) {
    payload.extend(((text.len() + 1) as u32).to_ne_bytes());
    payload.extend(text);
    payload.push(0);
    while !payload.len().is_multiple_of(4) {
        payload.push(0);
    }
}

fn array(payload: &mut Vec<u8>, seed: i32) {
    let body: Vec<u8> = match seed.rem_euclid(3) {
        0 => Vec::new(),
        1 => seed.to_ne_bytes().to_vec(),
        _ => vec![0; 1024],
    };
    payload.extend((body.len() as u32).to_ne_bytes());
    payload.extend(&body);
    while !payload.len().is_multiple_of(4) {
        payload.push(0);
    }
}

/// An object id argument: null, a live object of the wanted interface, any live object, a
/// destroyed one, or whatever the integer says.
fn pick_object(objects: &Objects, wanted: Option<&str>, cursor: &mut Cursor<'_>) -> u32 {
    let live = objects.listed(Life::Live);
    let seed = cursor.next();
    let nth = |ids: Vec<u32>, n: i32| {
        ids.get(n.unsigned_abs() as usize % ids.len().max(1))
            .copied()
    };
    match seed.rem_euclid(5) {
        0 => 0,
        1 => nth(
            live.iter()
                .filter(|(_, name)| Some(name.as_str()) == wanted)
                .map(|(id, _)| *id)
                .collect(),
            cursor.next(),
        )
        .unwrap_or(0),
        2 => nth(live.iter().map(|(id, _)| *id).collect(), cursor.next()).unwrap_or(0),
        3 => nth(
            objects
                .listed(Life::Dead)
                .iter()
                .map(|(id, _)| *id)
                .collect(),
            cursor.next(),
        )
        .unwrap_or(9999),
        _ => cursor.next() as u32,
    }
}

/// A descriptor of a kind chosen by `seed`, or none (a message that claims a descriptor it
/// does not carry).
fn descriptor(seed: i32) -> Option<OwnedFd> {
    use rustix::fs::{MemfdFlags, ftruncate, memfd_create};
    match seed.rem_euclid(5) {
        0 => None,
        1 => rustix::pipe::pipe().ok().map(|(read, _write)| read),
        2 => std::os::unix::net::UnixStream::pair()
            .ok()
            .map(|(a, _b)| OwnedFd::from(a)),
        3 => {
            let fd = memfd_create("casement-fuzz", MemfdFlags::CLOEXEC).ok()?;
            ftruncate(&fd, u64::from((seed % MEMFD_MAX).unsigned_abs())).ok()?;
            Some(fd)
        }
        _ => std::fs::File::open("/dev/null").ok().map(OwnedFd::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::catalogue;
    use crate::conn::wire;

    fn cx<'a>(objects: &'a Objects, globals: &'a [Global], next: &'a mut u32) -> Context<'a> {
        Context {
            objects,
            globals,
            next_id: next,
        }
    }

    #[test]
    fn a_bind_names_a_real_global_and_takes_a_fresh_id() {
        let registry = catalogue::lookup("wl_registry").expect("registry");
        let globals = [Global {
            name: 7,
            interface: "wl_compositor".to_owned(),
            version: 5,
        }];
        let objects = Objects::default();
        let mut next = 10;
        let built = build(
            Some(&registry.requests[0]),
            &[0, 0],
            &mut cx(&objects, &globals, &mut next),
        );
        let args = wire::decode(&registry.requests[0], &built.payload).expect("well formed");
        assert_eq!(
            args,
            vec![
                wire::Arg::Uint(7),
                wire::Arg::Str(Some("wl_compositor".to_owned())),
                wire::Arg::Uint(5),
                wire::Arg::NewId(10)
            ]
        );
        assert_eq!(next, 11);
    }

    #[test]
    fn every_request_of_every_known_interface_builds_a_body_its_signature_reads() {
        let mut checked = 0;
        for interface in catalogue::all().values() {
            for desc in interface.requests {
                for seed in [0, 1, 2, 3, 4, i32::MIN, i32::MAX, -1] {
                    let objects = Objects::default();
                    let mut next = 2;
                    let built = build(Some(desc), &[seed], &mut cx(&objects, &[], &mut next));
                    assert!(
                        wire::decode(desc, &built.payload).is_ok(),
                        "{}.{} seed {seed}",
                        interface.name,
                        desc.name
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000, "{checked} bodies");
    }

    #[test]
    fn a_request_the_interface_does_not_have_goes_out_as_raw_words() {
        let objects = Objects::default();
        let mut next = 2;
        let built = build(None, &[1, 2, 3], &mut cx(&objects, &[], &mut next));
        assert_eq!(built.payload.len(), 12);
    }
}
