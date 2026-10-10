//! The Spaces file store (A1, P1-M2): an atomic write, a missing file, the file's name and the
//! typed helpers over a store.

use casement::space_store::{
    EphemeralSpaceStore, FileSpaceStore, SpaceStore, StateDirs, load_spaces, save_spaces,
};
use casement_core::space::SpaceFile;

#[test]
fn a_write_is_atomic_and_reads_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let dirs = StateDirs::at(dir.path().join("casement"));
    let mut store = FileSpaceStore::new(&dirs);
    assert_eq!(store.read().expect("read"), None, "no file yet");
    store.write("spaces = []\n").expect("write");
    assert_eq!(
        store.read().expect("read").as_deref(),
        Some("spaces = []\n")
    );
    let leftovers: Vec<_> = std::fs::read_dir(dirs.dir())
        .expect("the state directory exists after a write")
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(leftovers.len(), 1, "no temporary file is left behind");
}

#[test]
fn a_second_write_replaces_the_first() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = FileSpaceStore::new(&StateDirs::at(dir.path().to_path_buf()));
    store.write("first").expect("write");
    store.write("second").expect("write");
    assert_eq!(store.read().expect("read").as_deref(), Some("second"));
}

#[test]
fn the_file_is_named_spaces_toml() {
    let dirs = StateDirs::at(std::path::PathBuf::from("/state/casement"));
    assert!(dirs.spaces_file().ends_with("spaces.toml"));
}

#[test]
fn spaces_saved_through_a_store_load_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = FileSpaceStore::new(&StateDirs::at(dir.path().join("state")));
    assert_eq!(
        load_spaces(&store).expect("load"),
        SpaceFile::empty(),
        "a missing file is no Spaces"
    );
    let file = SpaceFile::import_sill_json(
        r#"{"by_id": {"s-00000000000000000000000000000001": {"index": 0}}}"#,
    )
    .expect("import");
    save_spaces(&mut store, &file).expect("save");
    assert_eq!(load_spaces(&store).expect("load"), file);
}

#[test]
fn a_file_that_does_not_parse_is_a_typed_error_and_an_ephemeral_store_keeps_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = FileSpaceStore::new(&StateDirs::at(dir.path().to_path_buf()));
    store.write("spaces = 3").expect("write");
    assert!(load_spaces(&store).is_err());
    let mut nowhere = EphemeralSpaceStore;
    nowhere.write("anything").expect("write");
    assert_eq!(nowhere.read().expect("read"), None);
}
