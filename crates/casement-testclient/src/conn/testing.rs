//! Wire-building helpers for the tests of this module: a compositor played by hand.

use super::wire;
use std::io::Read;
use std::os::unix::net::UnixStream;

/// One message: header and `payload`.
pub(super) fn message(object: u32, opcode: u16, payload: &[u8]) -> Vec<u8> {
    let size = (wire::HEADER + payload.len()) as u32;
    let mut bytes = Vec::new();
    bytes.extend(object.to_ne_bytes());
    bytes.extend(((size << 16) | u32::from(opcode)).to_ne_bytes());
    bytes.extend(payload);
    bytes
}

/// A string argument: length with its NUL, the bytes, padding.
pub(super) fn string(text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(((text.len() + 1) as u32).to_ne_bytes());
    bytes.extend(text.as_bytes());
    bytes.push(0);
    while !bytes.len().is_multiple_of(4) {
        bytes.push(0);
    }
    bytes
}

/// A `wl_registry.global` event.
pub(super) fn global(registry: u32, name: u32, interface: &str, version: u32) -> Vec<u8> {
    let mut payload = name.to_ne_bytes().to_vec();
    payload.extend(string(interface));
    payload.extend(version.to_ne_bytes());
    message(registry, 0, &payload)
}

/// Everything waiting on `from`, without blocking.
pub(super) fn read_all(mut from: &UnixStream) -> Vec<u8> {
    let mut got = Vec::new();
    let mut buffer = [0u8; 4096];
    while let Ok(n) = from.read(&mut buffer) {
        if n == 0 {
            break;
        }
        got.extend_from_slice(&buffer[..n]);
    }
    got
}

/// The (object, opcode) of each whole message in `bytes`.
pub(super) fn heads(bytes: &[u8]) -> Vec<(u32, u16)> {
    let mut at = 0;
    let mut found = Vec::new();
    while let Some((frame, took)) = wire::next_frame(&bytes[at..]) {
        found.push((frame.object, frame.opcode));
        at += took;
    }
    found
}
