//! `workspace` (P1-M2): Spaces as ext-workspace. The workspace `id` is the Space's minted id,
//! the same after a restart on the same Spaces file; activate, create and remove take effect at
//! `commit`; the announcement's transcript is a golden.

use crate::protocol::wlr_toplevel::flow;
use crate::protocol::xdg_shell::{check_golden, golden};
use casement_core::policy::ConnectionRole;
use casement_testclient::manage::workspace::{Shown, WorkspaceWatch};
use casement_testkit::fakes::MemorySpaceStore;
use casement_testkit::{ClientId, Fixture, FixtureSpec};

/// A fixture on `spec`, one public client and a bound workspace list, all read.
fn watched(spec: FixtureSpec) -> (Fixture, ClientId, WorkspaceWatch) {
    let mut fixture = Fixture::start(spec).expect("fixture");
    let client = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let mut watch = WorkspaceWatch::start(fixture.client(client).globals()).expect("the list");
    settle(&mut fixture, &mut watch);
    (fixture, client, watch)
}

/// Pumps the compositor and the workspace client until both are quiet.
fn settle(fixture: &mut Fixture, watch: &mut WorkspaceWatch) {
    for _ in 0..2 {
        fixture.pump();
        watch.pump().expect("pump");
    }
}

/// The ids the workspace client was told, in announcement order.
fn ids(watch: &WorkspaceWatch) -> Vec<String> {
    watch
        .workspaces()
        .into_iter()
        .map(|workspace| workspace.id.expect("every Space has an id"))
        .collect()
}

/// The workspace `id` is the minted `SpaceId`.
#[test]
fn the_workspace_id_is_the_minted_space_id() {
    let (fixture, _client, watch) = watched(FixtureSpec::desktop());
    let minted: Vec<String> = fixture
        .model()
        .spaces()
        .iter()
        .map(|space| space.id.as_str().to_owned())
        .collect();
    assert_eq!(minted.len(), 1, "one desktop on the one output");
    assert_eq!(ids(&watch), minted);
    let groups = watch.groups();
    assert_eq!(groups.len(), 1, "a group per output");
    assert_eq!(groups[0].outputs, 1);
    assert_eq!(groups[0].capabilities, 1, "workspaces can be created");
    let workspaces = watch.workspaces();
    assert_eq!(workspaces[0].group, Some(0));
    assert_eq!(workspaces[0].shown, Shown::Yes, "the output shows it");
    assert_eq!(workspaces[0].name, "Desktop 1");
    assert_eq!(workspaces[0].coordinates, [0]);
    assert!(watch.dones() >= 1);
}

/// The id is the same after a restart: a second compositor started on the same store finds the
/// stored Spaces (`spaces.toml`). The store is edited between the runs to name an id the
/// fixture's entropy would never mint, so the id can only have come from the file.
#[test]
fn the_workspace_id_survives_a_restart() {
    let store = MemorySpaceStore::empty();
    let (first, _client, watch) = watched(FixtureSpec::desktop().with_space_store(store.clone()));
    let minted = ids(&watch);
    let text = store.stored().expect("the Spaces file was written");
    assert!(text.contains(&minted[0]), "the file names the id: {text}");
    drop(first);

    let (_again, _client, same) = watched(FixtureSpec::desktop().with_space_store(store));
    assert_eq!(
        ids(&same),
        minted,
        "a restart on the same file keeps the id"
    );

    let chosen = "s-00000000000000000000000000abcdef";
    let edited = MemorySpaceStore::holding(&text.replace(&minted[0], chosen));
    let (_third, _client, chosen_watch) = watched(FixtureSpec::desktop().with_space_store(edited));
    assert_eq!(ids(&chosen_watch), [chosen], "the id comes from the file");
}

/// activate, create_workspace and remove reach the model as `Event::Workspace`, at `commit`.
#[test]
fn requests_reach_the_model() {
    let (mut fixture, _client, mut watch) = watched(FixtureSpec::desktop());
    let output = fixture
        .model()
        .spaces()
        .iter()
        .next()
        .expect("a Space")
        .output
        .clone();
    let row = |fixture: &Fixture| fixture.model().spaces().row(&output).len();

    watch.create_workspace(0, "Two").expect("request");
    settle(&mut fixture, &mut watch);
    assert_eq!(row(&fixture), 1, "nothing happens before the commit");
    watch.commit().expect("commit");
    settle(&mut fixture, &mut watch);
    assert_eq!(row(&fixture), 2, "created");
    assert_eq!(watch.workspaces().len(), 2, "and announced");
    assert_eq!(watch.workspaces()[1].group, Some(0));

    let second = ids(&watch)[1].clone();
    watch.activate(1).expect("request");
    settle(&mut fixture, &mut watch);
    let shown = |fixture: &Fixture| {
        fixture
            .model()
            .spaces()
            .shown_on(&output)
            .map(|space| space.id.as_str().to_owned())
    };
    assert_ne!(
        shown(&fixture).as_deref(),
        Some(second.as_str()),
        "not before the commit"
    );
    watch.commit().expect("commit");
    settle(&mut fixture, &mut watch);
    assert_eq!(
        shown(&fixture).as_deref(),
        Some(second.as_str()),
        "activated"
    );
    let seen = watch.workspaces();
    assert_eq!(seen[0].shown, Shown::No);
    assert_eq!(seen[1].shown, Shown::Yes);

    watch.remove(1).expect("request");
    watch.commit().expect("commit");
    settle(&mut fixture, &mut watch);
    assert_eq!(row(&fixture), 1, "removed");
    assert_eq!(watch.workspaces().len(), 1);
    assert_eq!(
        watch.workspaces()[0].shown,
        Shown::Yes,
        "the neighbour is shown again"
    );
}

/// The announcement of one group and workspace: the transcript matches
/// tests/golden/transcripts/workspace-publish.txt.
#[test]
fn the_announcement_transcript_matches_its_golden() {
    let (mut fixture, client, _watch) = watched(FixtureSpec::desktop());
    let text = flow(&mut fixture, client, &["ext_workspace_"], &["id"]);
    check_golden(&golden("workspace-publish.txt"), &text).expect("the announcement flow");
}
