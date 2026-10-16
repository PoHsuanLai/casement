//! `ext_data_control` (P0-H5).

use crate::protocol::consent::{asked_world, decided, heard, kept};
use casement_core::geometry::PhysicalSize;
use casement_core::policy::ConnectionRole;
use casement_core::privilege::Verdict;
use casement_protocols::vocab::{Answer, Power};
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::selection::data_control::ExtControl;
use casement_testclient::selection::data_device::DataDevice;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::{Pattern, shm_buffer};
use casement_testkit::{ASKED_PROGRAM, ClientId, Fixture, FixtureSpec};

/// A clipboard manager is told of every new selection.
#[test]
fn a_manager_sees_selection_changes() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let owner = fixture.connect(ConnectionRole::Shell);
    let manager = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let keyboard = Keyboard::bind(fixture.client(owner)).expect("keyboard");
    let device = DataDevice::bind(fixture.client(owner), keyboard.seat(), None).expect("device");
    let manager_keyboard = Keyboard::bind(fixture.client(manager)).expect("keyboard");
    let _control =
        ExtControl::bind(fixture.client(manager), manager_keyboard.seat()).expect("manager");
    let _layer = map(&mut fixture, owner, &focused(Layer::Top, "owner"));
    device.copy(fixture.client(owner), "text/plain", 0);
    fixture.pump();
    device.copy(fixture.client(owner), "text/uri-list", 0);
    fixture.pump();
    let selections = fixture
        .client(manager)
        .transcript()
        .0
        .iter()
        .filter(|line| line.event == "selection")
        .count();
    assert!(
        selections >= 2,
        "the manager was told of both changes, saw {selections}"
    );
}
/// A layer of 400x300 at the centre of the output, at `level`, taking the keyboard exclusively.
fn focused(level: Layer, namespace: &'static str) -> LayerAsk {
    LayerAsk {
        level,
        namespace,
        anchor: Anchor::empty(),
        exclusive_zone: 0,
        keyboard: KeyboardInteractivity::Exclusive,
        size: (400, 300),
    }
}

/// Maps `ask` the way a shell does: create, wait for the configure, acknowledge it and attach
/// a buffer of the configured size.
fn map(fixture: &mut Fixture, id: ClientId, ask: &LayerAsk) -> LayerSurface {
    let layer = LayerSurface::create(fixture.client(id), ask).expect("a layer surface");
    fixture.pump();
    let configure = layer.configure().expect("the initial configure");
    assert!(layer.ack(fixture.client(id)));
    let size = PhysicalSize {
        w: configure.width.max(ask.size.0),
        h: configure.height.max(ask.size.1),
    };
    let buffer = shm_buffer(fixture.client(id), size, Pattern::Solid(0xff20_2020));
    layer.show(fixture.client(id), &buffer);
    fixture.pump();
    layer
}

// ---- Per-app consent (`Power::Clipboard`): smithay serves this protocol whole, so it holds what a
// ---- client that waits for the answer created and the gate hook settles it.

/// Binds a clipboard manager on `id` (a manager and the device of its seat) and lets the
/// compositor answer.
fn bind(fixture: &mut Fixture, id: ClientId) -> ExtControl {
    let keyboard = Keyboard::bind(fixture.client(id)).expect("keyboard");
    let control = ExtControl::bind(fixture.client(id), keyboard.seat()).expect("manager");
    fixture.pump();
    control
}

/// The shell takes the keyboard on a layer and sets the clipboard: what a manager that may
/// watch the clipboard is told of. What it returns must stay alive.
fn shell_copies(fixture: &mut Fixture) -> (Keyboard, DataDevice, LayerSurface) {
    let owner = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let keyboard = Keyboard::bind(fixture.client(owner)).expect("keyboard");
    let device = DataDevice::bind(fixture.client(owner), keyboard.seat(), None).expect("device");
    let layer = map(fixture, owner, &focused(Layer::Top, "owner"));
    device.copy(fixture.client(owner), "text/plain", 0);
    fixture.pump();
    (keyboard, device, layer)
}

/// A program the person already allowed binds its manager without a question and is told
/// the clipboard.
#[test]
fn a_program_the_person_allowed_gets_a_working_manager_without_a_question() {
    let (mut fixture, shell, client) = asked_world();
    decided(&mut fixture, Power::Clipboard, Verdict::Allow);
    let _control = bind(&mut fixture, client);
    assert!(shell.asked().is_empty(), "the book answers");
    assert!(
        heard(&mut fixture, client, "selection") > 0,
        "told the clipboard at once"
    );
    assert_eq!(heard(&mut fixture, client, "finished"), 0);
    let before = heard(&mut fixture, client, "selection");
    let _owner = shell_copies(&mut fixture);
    assert!(
        heard(&mut fixture, client, "selection") > before,
        "and every change"
    );
}

/// A program the person already refused gets a finished device and a cancelled source, and
/// is never told the clipboard, without being asked again.
#[test]
fn a_program_the_person_refused_never_gets_the_clipboard() {
    let (mut fixture, shell, client) = asked_world();
    decided(&mut fixture, Power::Clipboard, Verdict::Deny);
    let control = bind(&mut fixture, client);
    let _source = control.source(fixture.client(client));
    fixture.pump();
    assert!(shell.asked().is_empty(), "the book answers");
    assert_eq!(heard(&mut fixture, client, "finished"), 1);
    assert_eq!(heard(&mut fixture, client, "cancelled"), 1);
    let _owner = shell_copies(&mut fixture);
    assert_eq!(heard(&mut fixture, client, "selection"), 0);
    assert_eq!(heard(&mut fixture, client, "data_offer"), 0);
}

/// Asked, then allowed: the manager hears nothing while the question is open, and once the
/// person allows it is told the clipboard and every change after.
#[test]
fn a_manager_made_while_asked_is_released_when_allowed() {
    let (mut fixture, shell, client) = asked_world();
    let control = bind(&mut fixture, client);
    let _source = control.source(fixture.client(client));
    fixture.pump();
    assert_eq!(
        shell.asked().len(),
        1,
        "one question for the device and the source"
    );
    assert_eq!(shell.asked()[0].power, Power::Clipboard);
    let _owner = shell_copies(&mut fixture);
    assert_eq!(
        heard(&mut fixture, client, "selection"),
        0,
        "nothing while it waits"
    );

    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert!(
        heard(&mut fixture, client, "selection") > 0,
        "told the clipboard now"
    );
    assert_eq!(heard(&mut fixture, client, "finished"), 0);
    assert_eq!(heard(&mut fixture, client, "cancelled"), 0);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Clipboard),
        Some(Verdict::Allow)
    );
}

/// Asked, then refused: the device is finished, the source cancelled, and the clipboard never
/// reaches them.
#[test]
fn a_manager_made_while_asked_is_finished_when_refused() {
    let (mut fixture, shell, client) = asked_world();
    let control = bind(&mut fixture, client);
    let _source = control.source(fixture.client(client));
    fixture.pump();
    shell.answer(&mut fixture, shell.only_ask(), Answer::Deny);
    assert_eq!(heard(&mut fixture, client, "finished"), 1);
    assert_eq!(heard(&mut fixture, client, "cancelled"), 1);
    let _owner = shell_copies(&mut fixture);
    assert_eq!(heard(&mut fixture, client, "selection"), 0);
    assert_eq!(
        kept(&fixture, ASKED_PROGRAM, Power::Clipboard),
        Some(Verdict::Deny)
    );
}

/// A power the person takes back ends the manager the program already had: its device is
/// finished and the clipboard no longer reaches it (smithay's `DataControlState::revoke`).
#[test]
fn a_manager_the_person_switches_off_is_finished() {
    let (mut fixture, _shell, client) = asked_world();
    decided(&mut fixture, Power::Clipboard, Verdict::Allow);
    let _control = bind(&mut fixture, client);
    assert_eq!(heard(&mut fixture, client, "finished"), 0);
    decided(&mut fixture, Power::Clipboard, Verdict::Deny);
    fixture.pump();
    assert_eq!(heard(&mut fixture, client, "finished"), 1);
    let before = heard(&mut fixture, client, "selection");
    let _owner = shell_copies(&mut fixture);
    assert_eq!(
        heard(&mut fixture, client, "selection"),
        before,
        "the clipboard no longer reaches it"
    );
}

/// A client that hangs up while its manager waits is forgotten, and an answer that comes
/// after it is gone changes nothing for anyone.
#[test]
fn a_client_that_leaves_while_its_manager_waits_is_forgotten() {
    let (mut fixture, shell, client) = asked_world();
    let _control = bind(&mut fixture, client);
    assert_eq!(fixture.held_connections(), 1);
    fixture.disconnect(client);
    assert_eq!(fixture.held_connections(), 0);
    shell.answer(&mut fixture, shell.only_ask(), Answer::Allow);
    assert!(fixture.client(shell.id).protocol_error().is_none());
    let _owner = shell_copies(&mut fixture);
}
