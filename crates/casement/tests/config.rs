//! The settings document (P0-F1-output): defaults when absent, one conversion into the model's
//! settings, and the schema detent reads (`casement --write-schema`).

use casement::cli::ConfigHome;
use casement::config::{CasementDoc, load, to_core, write_schema};
use casement_core::settings::CasementSettings;

#[test]
#[ignore = "P0-F1-output"]
fn a_missing_document_gives_the_defaults() {
    let home = tempfile::tempdir().expect("temp dir");
    let doc = load(&ConfigHome::At(home.path().to_path_buf())).expect("load");
    assert_eq!(doc, CasementDoc::default());
    assert_eq!(to_core(&doc), CasementSettings::defaults());
}

#[test]
#[ignore = "P0-F1-output"]
fn the_schema_is_written_and_reads_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_schema(dir.path()).expect("schema");
    let text = std::fs::read_to_string(dir.path().join("casement.settings.toml")).expect("written");
    let schema = ds_settings::schema::Schema::from_toml(&text).expect("parses");
    assert!(!schema.key.is_empty(), "every settings key has a row");
}
