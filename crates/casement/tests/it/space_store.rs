//! The Spaces file store (A1, P1-M2). Ignored until the lane fills it.

use casement::space_store::{FileSpaceStore, SpaceStore, StateDirs};

#[test]
#[ignore = "P1-M2"]
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
#[ignore = "P1-M2"]
fn the_file_is_named_spaces_toml() {
    let dirs = StateDirs::at(std::path::PathBuf::from("/state/casement"));
    assert!(dirs.spaces_file().ends_with("spaces.toml"));
}
