//! proptest strategies for fuzz ops: boundary integers, every connection role, requests on
//! every interface the test client can build.

use super::ops::{FuzzOp, FuzzRequest, ObjectRef};
use casement_core::policy::ConnectionRole;
use casement_testclient::TestClient;
use porter_core::SandboxFacts;
use proptest::prelude::*;

/// An integer argument: the boundaries more often than chance would pick them.
pub fn argument() -> impl Strategy<Value = i32> {
    prop_oneof![
        3 => Just(0),
        2 => Just(1),
        2 => Just(-1),
        2 => Just(i32::MIN),
        2 => Just(i32::MAX),
        3 => 0..64,
        3 => any::<i32>(),
    ]
}

/// A connection role: mostly the public socket.
pub fn role() -> impl Strategy<Value = ConnectionRole> {
    prop_oneof![
        6 => Just(ConnectionRole::Public),
        2 => Just(ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
            app: "org.example.Fuzz".to_owned(),
            instance: "1".to_owned(),
        })),
        1 => Just(ConnectionRole::Shell),
        1 => Just(ConnectionRole::Agent),
    ]
}

/// Which object a request names: usually a live one.
pub fn object() -> impl Strategy<Value = ObjectRef> {
    prop_oneof![
        8 => (0..32u32).prop_map(ObjectRef::Valid),
        2 => (0..8u32).prop_map(ObjectRef::Destroyed),
        1 => (0..8u32).prop_map(ObjectRef::Foreign),
        1 => Just(ObjectRef::Null),
    ]
}

/// A request on some interface the client knows; the opcode is usually the interface's own.
pub fn wire_request() -> impl Strategy<Value = FuzzRequest> {
    let interfaces = TestClient::known_interfaces();
    (
        proptest::sample::select(interfaces),
        0..4u8,
        proptest::collection::vec(argument(), 0..6),
        any::<u16>(),
    )
        .prop_map(|((interface, requests), kind, args, wild)| {
            // One opcode in four is wild: usually past the interface's own.
            let own = u16::try_from(requests).unwrap_or(u16::MAX);
            let opcode = if own == 0 || kind == 0 {
                wild % 64
            } else {
                wild % own
            };
            FuzzRequest::Wire {
                interface: interface.to_owned(),
                opcode,
                args,
            }
        })
}

/// A request, now and then repeated into a flood.
pub fn request() -> impl Strategy<Value = FuzzRequest> {
    prop_oneof![
        12 => wire_request(),
        1 => (2..2_000u16, wire_request()).prop_map(|(times, request)| FuzzRequest::Repeated {
            times,
            request: Box::new(request),
        }),
    ]
}

/// One op: mostly requests, now and then time passing.
pub fn op() -> impl Strategy<Value = FuzzOp> {
    prop_oneof![
        10 => (role(), object(), request())
            .prop_map(|(role, object, request)| FuzzOp::Request { role, object, request }),
        2 => (1..200u32).prop_map(FuzzOp::Advance),
    ]
}

/// A sequence of up to `len` ops.
pub fn ops(len: usize) -> impl Strategy<Value = Vec<FuzzOp>> {
    proptest::collection::vec(op(), 1..=len)
}
