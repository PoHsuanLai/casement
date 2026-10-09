//! Typed fuzzing (P0-T-client, 11 §3.5): the box at the merge tier (each test is a fixed seed,
//! a bounded number of sequences, so the box is a count and not a clock: B8 keeps clocks out
//! of tests) and a bigger one at the phase tier; every sequence keeps the five oracles
//! (`casement_testkit::fuzz`). A failing sequence is shrunk and persisted under
//! `tests/proptest-regressions/`, and replays first from then on.

// Its own binary: the most expensive tests, tiered by `GATE_TIER`, with a `file!()`-keyed
// persistence directory.
use casement_core::policy::ConnectionRole;
use casement_testkit::fuzz::{FuzzOp, FuzzRequest, ObjectRef, run_ops, strategy};
use casement_testkit::{Fixture, FixtureSpec};
use proptest::test_runner::{
    Config, FileFailurePersistence, RngAlgorithm, TestCaseError, TestRng, TestRunner,
};

/// Ops in a sequence at most.
const SEQUENCE: usize = 24;
/// Sequences per seed at the merge tier; the phase tier takes five times as many. Measured in the
/// jail: a case costs about 0.15 s, so 24 cases take about 3.5 s of a test's 10 s. A fuzzed `wl_registry.bind` with a nonsense
/// version is a core protocol error (`wl_display.invalid_object`), which oracle 2 documents.
const MERGE_CASES: u32 = 24;

/// How many sequences each seed's test runs: `CASEMENT_FUZZ_CASES` when set, else by tier.
fn cases() -> u32 {
    let fixed = std::env::var("CASEMENT_FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok());
    let phase = std::env::var("GATE_TIER").is_ok_and(|tier| tier == "phase");
    fixed.unwrap_or(if phase { MERGE_CASES * 5 } else { MERGE_CASES })
}

/// One seed's share of the box.
fn run_seed(seed: u8) {
    let config = Config {
        cases: cases(),
        source_file: Some(file!()),
        failure_persistence: Some(Box::new(FileFailurePersistence::WithSource(
            "proptest-regressions",
        ))),
        ..Config::default()
    };
    let mut runner = TestRunner::new_with_rng(
        config,
        TestRng::from_seed(RngAlgorithm::ChaCha, &[seed; 32]),
    );
    let outcome = runner.run(&strategy::ops(SEQUENCE), |ops| {
        let mut fixture = Fixture::start(FixtureSpec::desktop())
            .map_err(|error| TestCaseError::fail(format!("the fixture did not start: {error}")))?;
        run_ops(&mut fixture, &ops).map_err(TestCaseError::fail)
    });
    if let Err(failure) = outcome {
        panic!("seed {seed}: {failure}");
    }
}

#[test]
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

/// The box, in eight fixed-seed shares so each stays inside a test's 10 s.
macro_rules! the_typed_fuzz_box {
    ($($name:ident: $seed:literal),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                run_seed($seed);
            }
        )*
    };
}

the_typed_fuzz_box! {
    the_typed_fuzz_box_keeps_the_five_oracles_seed_1: 1,
    the_typed_fuzz_box_keeps_the_five_oracles_seed_2: 2,
    the_typed_fuzz_box_keeps_the_five_oracles_seed_3: 3,
    the_typed_fuzz_box_keeps_the_five_oracles_seed_4: 4,
    the_typed_fuzz_box_keeps_the_five_oracles_seed_5: 5,
    the_typed_fuzz_box_keeps_the_five_oracles_seed_6: 6,
    the_typed_fuzz_box_keeps_the_five_oracles_seed_7: 7,
    the_typed_fuzz_box_keeps_the_five_oracles_seed_8: 8,
}
