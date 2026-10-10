//! The Space ids file's codec (P1-M2). Ignored until the lane fills it.

use crate::support::space_id;
use casement_core::OutputKey;
use casement_core::space::{SpaceFile, SpaceIndex, StoredShown, StoredSpace};

fn file() -> SpaceFile {
    let output = serde_json::from_str::<OutputKey>("\"connector:DP-1\"").expect("output key");
    let one = space_id("s-00000000000000000000000000000001");
    let two = space_id("s-00000000000000000000000000000002");
    SpaceFile::new(
        vec![
            StoredSpace::new(one.clone(), output.clone(), SpaceIndex(0)),
            StoredSpace::new(two, output.clone(), SpaceIndex(1)),
        ],
        vec![StoredShown::new(output, one)],
    )
}

#[test]
#[ignore = "P1-M2"]
fn the_file_round_trips_through_its_text() {
    let file = file();
    let text = file.render().expect("render");
    assert_eq!(SpaceFile::parse(&text).expect("parse"), file);
    assert_eq!(
        file.render().expect("render again"),
        text,
        "the same bytes every time"
    );
}

#[test]
#[ignore = "P1-M2"]
fn an_empty_text_is_an_empty_file() {
    assert_eq!(SpaceFile::parse("").expect("parse"), SpaceFile::empty());
}

#[test]
#[ignore = "P1-M2"]
fn sills_interim_json_is_imported_once() {
    let json = r#"{"by_id": {"s-00000000000000000000000000000001": {"index": 0}}}"#;
    let file = SpaceFile::import_sill_json(json).expect("import");
    assert_eq!(file.spaces.len(), 1);
}

#[test]
#[ignore = "P1-M2"]
fn bad_text_is_a_typed_error() {
    assert!(SpaceFile::parse("spaces = 3").is_err());
    assert!(SpaceFile::import_sill_json("{").is_err());
}
