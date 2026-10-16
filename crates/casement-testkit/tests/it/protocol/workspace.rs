//! `workspace` (P1-M2): Spaces as ext-workspace. The workspace `id` is the Space's minted id,
//! the same after a restart on the same Spaces file; activate, create and remove take effect at
//! `commit`; the announcement's transcript is a golden.

use crate::protocol::consent::{ScriptedShell, kept, program};
use crate::protocol::wlr_toplevel::flow;
use crate::protocol::xdg_shell::{check_golden, golden};
use casement_core::Event;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::{UnixSeconds, Verdict};
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::manage::workspace::{Shown, WorkspaceWatch};
use casement_testkit::fakes::MemorySpaceStore;
use casement_testkit::{ASKED_PROGRAM, ClientId, Fixture, FixtureSpec};

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

/// A desktop with the scripted shell, and a Space list bound by a program that is asked.
fn asked_watch() -> (Fixture, ScriptedShell, WorkspaceWatch) {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let shell = ScriptedShell::start(&mut fixture);
    let client = fixture.connect_via_ask();
    fixture.pump();
    let mut watch = WorkspaceWatch::start(fixture.client(client).globals()).expect("the list");
    settle(&mut fixture, &mut watch);
    (fixture, shell, watch)
}

/// Seeing and managing the Spaces is the windows power: a manager bound by a program that is
/// asked is told nothing while the question is open, the shell is asked once, and on Allow the
/// groups and workspaces are announced.
#[test]
fn a_workspace_list_bound_while_asked_hears_nothing_until_allowed() {
    let (mut fixture, shell, mut watch) = asked_watch();
    assert_eq!(shell.asked().len(), 1, "the shell is asked once");
    assert_eq!(shell.asked()[0].power, Power::Windows);
    assert!(watch.groups().is_empty() && watch.workspaces().is_empty());
    assert_eq!(watch.dones(), 0);
    assert!(!watch.finished());

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.groups().len(), 1);
    assert_eq!(watch.workspaces().len(), 1);
    assert!(watch.dones() >= 1);
    assert!(!watch.finished());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Windows),
        Some(Verdict::Allow)
    );
}

/// Don't Allow: the manager is `finished` and announces nothing.
#[test]
fn a_refused_workspace_list_is_finished() {
    let (mut fixture, shell, mut watch) = asked_watch();
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    settle(&mut fixture, &mut watch);
    assert!(watch.finished());
    assert!(watch.groups().is_empty() && watch.workspaces().is_empty());
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Windows),
        Some(Verdict::Deny)
    );
}

/// Taking the power back finishes the manager in use, and what it asks for after that changes
/// nothing.
#[test]
fn a_power_taken_back_finishes_the_workspace_list_and_commits_change_nothing() {
    let (mut fixture, shell, mut watch) = asked_watch();
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    settle(&mut fixture, &mut watch);
    assert_eq!(watch.workspaces().len(), 1);
    assert!(!watch.finished());

    fixture.feed_events(vec![Event::PrivilegeRevoked {
        subject: program(ASKED_PROGRAM),
        power: Power::Windows,
        at: UnixSeconds(2),
    }]);
    settle(&mut fixture, &mut watch);
    assert!(watch.finished(), "the manager in use is finished");

    let output = fixture
        .model()
        .spaces()
        .iter()
        .next()
        .expect("a Space")
        .output
        .clone();
    watch.create_workspace(0, "Two").expect("request");
    watch.commit().expect("commit");
    settle(&mut fixture, &mut watch);
    assert_eq!(
        fixture.model().spaces().row(&output).len(),
        1,
        "a commit after the power was taken back creates nothing"
    );
}
