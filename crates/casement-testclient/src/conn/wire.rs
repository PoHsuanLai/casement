//! The Wayland wire format, just enough to read a message off a byte stream: split complete
//! messages, decode their arguments by signature, and print them the way a transcript does.
//! Nothing here sends; the tap in `tap.rs` only watches.

use wayland_backend::protocol::{ArgumentType, MessageDesc};

/// A message header: 8 bytes, the object id and `size << 16 | opcode`.
pub(super) const HEADER: usize = 8;

/// One complete message cut from a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Frame<'a> {
    /// The object it is for.
    pub object: u32,
    /// Its opcode.
    pub opcode: u16,
    /// Everything after the header.
    pub payload: &'a [u8],
}

/// The first complete message in `bytes` and how many bytes it took, or `None` when `bytes`
/// holds only part of one. A size below the header's is malformed: it reads as an 8-byte
/// message so the scan always moves on.
pub(super) fn next_frame(bytes: &[u8]) -> Option<(Frame<'_>, usize)> {
    let header = bytes.get(..HEADER)?;
    let object = word(&header[0..4]);
    let second = word(&header[4..8]);
    let size = ((second >> 16) as usize).max(HEADER);
    let payload = bytes.get(HEADER..size)?;
    Some((
        Frame {
            object,
            opcode: (second & 0xffff) as u16,
            payload,
        },
        size,
    ))
}

fn word(four: &[u8]) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(four);
    u32::from_ne_bytes(bytes)
}

/// One decoded argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Arg {
    Int(i32),
    Uint(u32),
    /// 24.8 fixed point, as sent.
    Fixed(i32),
    /// `None` is the null string.
    Str(Option<String>),
    /// 0 is null.
    Object(u32),
    /// A new object. Its interface is the message's declared one, or for `wl_registry.bind`
    /// (no declared interface; the signature spells out name, interface, version, id) the
    /// string just before it.
    NewId(u32),
    Array(Vec<u8>),
    /// The descriptor travels beside the stream.
    Fd,
}

/// Why a payload does not fit its signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Malformed;

/// Decodes `payload` by `desc.signature`.
pub(super) fn decode(desc: &MessageDesc, payload: &[u8]) -> Result<Vec<Arg>, Malformed> {
    let mut cursor = Cursor { rest: payload };
    let mut args = Vec::with_capacity(desc.signature.len());
    for kind in desc.signature {
        args.push(match kind {
            ArgumentType::Int => Arg::Int(cursor.word()? as i32),
            ArgumentType::Uint => Arg::Uint(cursor.word()?),
            ArgumentType::Fixed => Arg::Fixed(cursor.word()? as i32),
            ArgumentType::Str(_) => Arg::Str(cursor.string()?),
            ArgumentType::Object(_) => Arg::Object(cursor.word()?),
            ArgumentType::NewId => Arg::NewId(cursor.word()?),
            ArgumentType::Array => Arg::Array(cursor.padded()?.to_vec()),
            ArgumentType::Fd => Arg::Fd,
        });
    }
    Ok(args)
}

struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        let (head, tail) = self.rest.split_at_checked(n).ok_or(Malformed)?;
        self.rest = tail;
        Ok(head)
    }

    fn word(&mut self) -> Result<u32, Malformed> {
        Ok(word(self.take(4)?))
    }

    /// A length-prefixed run, padded to a word; the run without its padding.
    fn padded(&mut self) -> Result<&'a [u8], Malformed> {
        let len = self.word()? as usize;
        let run = self.take(len.div_ceil(4).checked_mul(4).ok_or(Malformed)?)?;
        run.get(..len).ok_or(Malformed)
    }

    /// A string: length (with its NUL), bytes, padding; length 0 is the null string.
    fn string(&mut self) -> Result<Option<String>, Malformed> {
        let len = self.word()? as usize;
        if len == 0 {
            return Ok(None);
        }
        let run = self.take(len.div_ceil(4).checked_mul(4).ok_or(Malformed)?)?;
        let text = run.get(..len - 1).ok_or(Malformed)?;
        Ok(Some(String::from_utf8_lossy(text).into_owned()))
    }
}

/// `Arg` as a transcript prints it. `name_of` says what interface an object id is (so
/// `wl_surface#7`); ids it does not know print as `#7`.
pub(super) fn show(arg: &Arg, declared: Option<&str>, name_of: &dyn Fn(u32) -> String) -> String {
    match arg {
        Arg::Int(v) => v.to_string(),
        Arg::Uint(v) => v.to_string(),
        Arg::Fixed(v) => format!("{:.3}", f64::from(*v) / 256.0),
        Arg::Str(None) | Arg::Object(0) => "nil".to_owned(),
        Arg::Str(Some(text)) => format!("{text:?}"),
        Arg::Object(id) => name_of(*id),
        Arg::NewId(id) => format!("new {}#{id}", declared.unwrap_or("")),
        Arg::Array(bytes) if bytes.len().is_multiple_of(4) => {
            let words: Vec<String> = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from_ne_bytes(*c).to_string())
                .collect();
            format!("[{}]", words.join(", "))
        }
        Arg::Array(bytes) => format!("bytes({})", bytes.len()),
        Arg::Fd => "fd".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::testing::{message, string};
    use wayland_client::Proxy;
    use wayland_client::protocol::{wl_display, wl_registry};

    #[test]
    fn a_partial_message_waits_and_a_complete_one_is_cut() {
        let whole = message(1, 1, &7u32.to_ne_bytes());
        assert_eq!(next_frame(&whole[..5]), None);
        assert_eq!(next_frame(&whole[..11]), None);
        let (frame, took) = next_frame(&whole).expect("complete");
        assert_eq!((frame.object, frame.opcode, took), (1, 1, 12));
        assert_eq!(frame.payload, 7u32.to_ne_bytes());
    }

    #[test]
    fn a_registry_global_event_decodes_its_string() {
        let mut payload = 3u32.to_ne_bytes().to_vec();
        payload.extend(string("wl_compositor"));
        payload.extend(6u32.to_ne_bytes());
        let desc = &wl_registry::WlRegistry::interface().events[0];
        let args = decode(desc, &payload).expect("decodes");
        assert_eq!(
            args,
            vec![
                Arg::Uint(3),
                Arg::Str(Some("wl_compositor".to_owned())),
                Arg::Uint(6)
            ]
        );
    }

    #[test]
    fn a_bind_reads_name_interface_version_and_id() {
        let mut payload = 3u32.to_ne_bytes().to_vec();
        payload.extend(string("wl_shm"));
        payload.extend(1u32.to_ne_bytes());
        payload.extend(9u32.to_ne_bytes());
        let desc = &wl_registry::WlRegistry::interface().requests[0];
        let args = decode(desc, &payload).expect("decodes");
        assert_eq!(
            args,
            vec![
                Arg::Uint(3),
                Arg::Str(Some("wl_shm".to_owned())),
                Arg::Uint(1),
                Arg::NewId(9)
            ]
        );
    }

    #[test]
    fn a_short_payload_is_malformed_not_a_panic() {
        let desc = &wl_display::WlDisplay::interface().events[0];
        assert_eq!(decode(desc, &[1, 0]), Err(Malformed));
        assert_eq!(decode(desc, &[]), Err(Malformed));
    }

    #[test]
    fn a_string_that_claims_more_than_it_has_is_malformed() {
        let payload = 4000u32.to_ne_bytes();
        let desc = &wl_registry::WlRegistry::interface().requests[0];
        assert_eq!(decode(desc, &payload), Err(Malformed));
    }

    #[test]
    fn arguments_print_as_a_transcript_line() {
        let name = |id: u32| format!("wl_surface#{id}");
        assert_eq!(show(&Arg::Fixed(384), None, &name), "1.500");
        assert_eq!(show(&Arg::Object(0), None, &name), "nil");
        assert_eq!(show(&Arg::Object(7), None, &name), "wl_surface#7");
        assert_eq!(show(&Arg::Array(vec![1, 0, 0, 0]), None, &name), "[1]");
        assert_eq!(show(&Arg::Array(Vec::new()), None, &name), "[]");
        assert_eq!(
            show(&Arg::Str(Some("a\"b".to_owned())), None, &name),
            "\"a\\\"b\""
        );
    }
}
