//! casement-testkit's integration tests: the fixture, the globals golden, adopt, the protocol wire tests and the renderer scenes.
//!
//! One integration binary: every former `tests/*.rs` is a module here, and `support` is shared.

mod adopt;
mod bindings;
mod fixture;
mod globals;
mod input_convert;
mod logind;
mod protocol;
mod scene;
mod space_sync;
mod topology;
mod tty;
mod writeback;
mod xwayland;

/// Test files that run in their own process and stay their own binary (a comment in each says why).
const STANDALONE: &[&str] = &["fuzz", "relax"];

/// Fails when a test file or module directory is not included above, or a `tests/*.rs` file is
/// not a listed standalone binary (the guard upstream Blitz uses: a file that is not `mod`-ed in
/// would silently never run).
#[test]
fn every_test_file_is_included() {
    let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let main = std::fs::read_to_string(tests.join("it/main.rs")).expect("main.rs");
    let stem =
        |path: &std::path::Path| path.file_stem().and_then(|s| s.to_str()).map(str::to_owned);
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(tests.join("it")).expect("tests/it") {
        let path = entry.expect("entry").path();
        let is_module = path.is_dir() || path.extension().is_some_and(|e| e == "rs");
        let name = stem(&path).expect("a name");
        if is_module && name != "main" && !main.contains(&format!("mod {name};")) {
            missing.push(format!("tests/it/{name}"));
        }
    }
    for entry in std::fs::read_dir(&tests).expect("tests") {
        let path = entry.expect("entry").path();
        let name = stem(&path).expect("a name");
        let is_binary =
            path.extension().is_some_and(|e| e == "rs") || path.join("main.rs").exists();
        if is_binary && name != "it" && !STANDALONE.contains(&name.as_str()) {
            missing.push(format!(
                "tests/{name} (a separate binary: include it in tests/it or list it in STANDALONE)"
            ));
        }
    }
    assert!(missing.is_empty(), "not included: {missing:?}");
}
