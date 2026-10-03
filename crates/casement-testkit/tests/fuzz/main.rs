//! Typed fuzzing (P0-T-client, 11 §3.5): a 60 s box at the merge tier, 300 s at the phase
//! tier; every sequence keeps the five oracles (`casement_testkit::fuzz`).

use casement_core::policy::ConnectionRole;
use casement_testkit::fuzz::{FuzzOp, FuzzRequest, ObjectRef, run_ops};
use casement_testkit::{Fixture, FixtureSpec};

#[test]
#[ignore = "P0-T-client"]
fn a_request_on_a_destroyed_object_disconnects_only_its_client() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let ops = [
        FuzzOp::Request {
            role: ConnectionRole::Public,
            object: ObjectRef::Destroyed(0),
            request: FuzzRequest::Wire {
                interface: "wl_surface".to_owned(),
                opcode: 6,
                args: Vec::new(),
            },
        },
        FuzzOp::Advance(16),
    ];
    run_ops(&mut fixture, &ops).expect("the oracles hold");
}

#[test]
#[ignore = "P0-T-client"]
fn the_typed_fuzz_box_keeps_the_five_oracles() {
    todo!(
        "P0-T-client: proptest over FuzzOp sequences for the tier's box (60 s merge, 300 s phase)"
    )
}
