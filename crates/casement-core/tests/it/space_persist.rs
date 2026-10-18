//! The Space ids file's codec and the model's adoption of it (P1-M2), and what a workspace
//! client may ask of the Spaces.

use crate::support::driver::{Op, fresh, last, plug, run, run_from};
use crate::support::{headless, ms, space_id};
use casement_core::OutputKey;
use casement_core::settings::CasementSettings;
use casement_core::space::{
    Space, SpaceBook, SpaceContent, SpaceFile, SpaceIndex, SpaceSeed, StoredShown, StoredSpace,
    WorkspaceRequest,
};
use casement_core::{Effect, Event, Model};
use porter_core::SpaceId;

fn output_key(text: &str) -> OutputKey {
    serde_json::from_str::<OutputKey>(&format!("\"{text}\"")).expect("output key")
}

fn file() -> SpaceFile {
    let output = output_key("connector:DP-1");
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
fn the_text_is_in_id_order_whatever_the_order_given() {
    let mut reversed = file();
    reversed.spaces.reverse();
    assert_eq!(
        reversed.render().expect("render"),
        file().render().expect("render")
    );
}

#[test]
fn an_empty_text_is_an_empty_file() {
    assert_eq!(SpaceFile::parse("").expect("parse"), SpaceFile::empty());
}

#[test]
fn sills_interim_json_is_imported_once() {
    let json = r#"{"by_id": {"s-00000000000000000000000000000001": {"index": 0}}}"#;
    let file = SpaceFile::import_sill_json(json).expect("import");
    assert_eq!(file.spaces.len(), 1);
    assert_eq!(
        file.spaces[0].output, None,
        "a record without an output has no home"
    );
}

#[test]
fn an_imported_record_keeps_its_output_and_index() {
    let json = r#"{"by_id": {"s-a": {"output": "connector:DP-2", "index": 3}, "s-b": {}}}"#;
    let file = SpaceFile::import_sill_json(json).expect("import");
    let want = vec![
        StoredSpace::new(space_id("s-a"), output_key("connector:DP-2"), SpaceIndex(3)),
        StoredSpace::homeless(space_id("s-b"), SpaceIndex(1)),
    ];
    assert_eq!(file.spaces, want);
}

/// CASC-18: a duplicate id cannot overwrite the Space under it, and a file of far more Spaces
/// than casement writes is cut, instead of making start-up slow.
#[test]
fn a_damaged_file_is_trimmed_to_what_casement_would_have_written() {
    let output = output_key("connector:DP-1");
    let two = |id: &str, position: u32| {
        format!("[[spaces]]\nid = \"{id}\"\noutput = \"connector:DP-1\"\nposition = {position}\n")
    };
    let one = "s-00000000000000000000000000000001";
    let text = format!(
        "{}{}[[shown]]\noutput = \"connector:DP-1\"\nspace = \"{one}\"\n\
         [[shown]]\noutput = \"connector:DP-1\"\nspace = \"s-other\"\n",
        two(one, 0),
        two(one, 5),
    );
    let read = SpaceFile::parse_counting(&text).expect("parse");
    assert_eq!(
        read.dropped, 2,
        "the second Space of the id and the second shown entry"
    );
    assert_eq!(
        read.value.spaces,
        vec![StoredSpace::new(
            space_id(one),
            output.clone(),
            SpaceIndex(0)
        )],
        "the first entry of an id wins"
    );
    assert_eq!(
        read.value.shown,
        vec![StoredShown::new(output, space_id(one))]
    );

    let many: String = (0..1000).map(|n| two(&format!("s-{n:032}"), n)).collect();
    let capped = SpaceFile::parse(&many).expect("parse");
    assert_eq!(capped.spaces.len(), casement_core::space::MAX_STORED_SPACES);
}

#[test]
fn bad_text_is_a_typed_error() {
    assert!(SpaceFile::parse("spaces = 3").is_err());
    assert!(SpaceFile::import_sill_json("{").is_err());
    assert!(SpaceFile::import_sill_json(r#"{"by_id": {"NOT AN ID": {}}}"#).is_err());
}

#[test]
fn only_desktops_are_stored() {
    let output = output_key("connector:DP-1");
    let space = |id: &str, content: SpaceContent, position: u32| Space {
        id: space_id(id),
        output: output.clone(),
        content,
        position: SpaceIndex(position),
    };
    let book = SpaceBook::empty()
        .with_space(space("s-a", SpaceContent::Desktop, 0))
        .with_space(space(
            "s-b",
            SpaceContent::FullScreen(casement_core::WindowId(1)),
            1,
        ))
        .with_shown(output.clone(), space_id("s-b"));
    let stored = SpaceFile::of_book(&book);
    assert_eq!(
        stored.spaces,
        vec![StoredSpace::new(space_id("s-a"), output, SpaceIndex(0))]
    );
    assert!(
        stored.shown.is_empty(),
        "a full-screen Space is not remembered as shown"
    );
}

/// The id of the Space `output` shows.
fn shown_id(model: &Model, output: &OutputKey) -> SpaceId {
    model
        .spaces()
        .shown_on(output)
        .expect("a shown Space")
        .id
        .clone()
}

fn model_with_seed(byte: u8) -> Model {
    Model::new(
        CasementSettings::defaults(),
        ms(0),
        SpaceSeed::from_entropy([byte; 16]),
    )
}

#[test]
fn a_restart_keeps_the_space_id_whether_the_file_or_the_output_comes_first() {
    let key = headless("HEADLESS-1").key;
    let first = run(vec![plug("HEADLESS-1")]);
    let stored = first.model.space_file();
    let id = shown_id(&first.model, &key);
    assert_eq!(stored.spaces.len(), 1);

    let loaded = Op::Event(Event::SpacesLoaded(Box::new(stored)));
    let file_first = run_from(model_with_seed(1), vec![loaded.clone(), plug("HEADLESS-1")]);
    assert_eq!(shown_id(&file_first.model, &key), id, "file, then output");
    let output_first = run_from(model_with_seed(2), vec![plug("HEADLESS-1"), loaded.clone()]);
    assert_eq!(shown_id(&output_first.model, &key), id, "output, then file");
    let again = run_from(output_first.model, vec![loaded]);
    assert_eq!(
        shown_id(&again.model, &key),
        id,
        "loading twice changes nothing"
    );
    assert_eq!(again.model.spaces().row(&key).len(), 1);
}

#[test]
fn a_stored_space_waits_for_its_output_and_stays_in_the_file() {
    let waiting = StoredSpace::new(
        space_id("s-00000000000000000000000000000009"),
        output_key("connector:DP-9"),
        SpaceIndex(0),
    );
    let stored = SpaceFile::new(vec![waiting.clone()], Vec::new());
    let run = run_from(
        fresh(),
        vec![
            Op::Event(Event::SpacesLoaded(Box::new(stored))),
            plug("HEADLESS-1"),
        ],
    );
    assert_eq!(
        run.model.spaces().iter().count(),
        1,
        "the waiting Space is not in the book"
    );
    assert!(
        run.model.space_file().spaces.contains(&waiting),
        "and the file still names it"
    );
}

#[test]
fn a_changed_space_asks_to_be_stored_and_published() {
    let key = headless("HEADLESS-1").key;
    let base = run(vec![plug("HEADLESS-1")]);
    let made = run_from(
        base.model,
        vec![Op::Event(Event::Workspace(WorkspaceRequest::Create {
            output: key.clone(),
            name: "two".to_owned(),
        }))],
    );
    assert_eq!(made.model.spaces().row(&key).len(), 2);
    assert!(last(&made).contains(&Effect::PersistSpaces));
    assert!(last(&made).contains(&Effect::PublishWorkspaces));
}

#[test]
fn activate_remove_and_assign_move_the_spaces() {
    let one = headless("HEADLESS-1").key;
    let two = headless("HEADLESS-2").key;
    let request = |request| Op::Event(Event::Workspace(request));
    let made = run(vec![
        plug("HEADLESS-1"),
        plug("HEADLESS-2"),
        request(WorkspaceRequest::Create {
            output: one.clone(),
            name: String::new(),
        }),
    ]);
    let row: Vec<SpaceId> = made
        .model
        .spaces()
        .row(&one)
        .iter()
        .map(|s| s.id.clone())
        .collect();
    assert_eq!(row.len(), 2);
    let first = shown_id(&made.model, &one);
    let other = row
        .iter()
        .find(|id| **id != first)
        .expect("the new Space")
        .clone();

    let activated = run_from(
        made.model,
        vec![request(WorkspaceRequest::Activate(other.clone()))],
    );
    assert_eq!(shown_id(&activated.model, &one), other);

    let removed = run_from(
        activated.model,
        vec![request(WorkspaceRequest::Remove(other.clone()))],
    );
    assert!(removed.model.spaces().get(&other).is_none());
    assert_eq!(
        shown_id(&removed.model, &one),
        first,
        "the neighbour is shown again"
    );

    let alone = run_from(
        removed.model,
        vec![request(WorkspaceRequest::Remove(first.clone()))],
    );
    assert!(
        alone.model.spaces().get(&first).is_some(),
        "an output keeps its only desktop"
    );

    let moved = run_from(
        alone.model,
        vec![request(WorkspaceRequest::Create {
            output: one.clone(),
            name: String::new(),
        })],
    );
    let spare = moved
        .model
        .spaces()
        .row(&one)
        .iter()
        .map(|s| s.id.clone())
        .find(|id| *id != first)
        .expect("a spare desktop");
    let assigned = run_from(
        moved.model,
        vec![request(WorkspaceRequest::Assign {
            space: spare.clone(),
            output: two.clone(),
        })],
    );
    assert_eq!(
        assigned.model.spaces().get(&spare).expect("space").output,
        two
    );
    assert_eq!(assigned.model.spaces().row(&one).len(), 1);
}

#[test]
fn a_stored_minimise_rectangle_is_where_the_window_minimises_to() {
    use crate::support::driver::map;
    use crate::support::{proven, rect};
    use casement_core::focus::TokenCheck;
    use casement_core::window::WlrToplevelRequest;
    use casement_core::{Cause, MinimiseTarget, WindowId};
    use casement_protocols::vocab::Toggle;

    let mail = proven("org.quire.Mail");
    let wlr = |request| {
        Op::Event(Event::WlrToplevel {
            window: WindowId(1),
            request,
        })
    };
    let tile = rect(500, 900, 64, 48);
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        wlr(WlrToplevelRequest::MinimiseRect(tile)),
        wlr(WlrToplevelRequest::Minimise(Toggle::On)),
    ]);
    let target = MinimiseTarget {
        output: headless("HEADLESS-1").key,
        rect: tile,
    };
    assert!(
        last(&run).contains(&Effect::Relayout(Cause::Minimise(target))),
        "the genie goes to the tile the client named"
    );
}
