//! `seat` (P0-H4).

use casement_core::geometry::{LogicalPoint, PhysicalSize, SubPoint, SubPx};
use casement_core::input::{AxisFrame, AxisSource, DeviceClass, KeyCode};
use casement_core::policy::ConnectionRole;
use casement_core::settings::RepeatRate;
use casement_protocols::vocab::PressEdge;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::layer::{
    Anchor, KeyboardInteractivity, Layer, LayerAsk, LayerSurface,
};
use casement_testclient::{Pattern, TestClient, shm_buffer};
use casement_testkit::{ClientId, Fixture, FixtureSpec, TestOrigin};
use std::time::Duration;

/// A layer that spans the whole output at `level` and takes no keyboard.
fn cover(level: Layer, namespace: &'static str) -> LayerAsk {
    LayerAsk {
        level,
        namespace,
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        exclusive_zone: -1,
        keyboard: KeyboardInteractivity::None,
        size: (0, 0),
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

/// The events of `object_prefix` objects in `client`'s transcript, as `(object, event, args)`.
fn events_of(client: &TestClient, object_prefix: &str) -> Vec<(String, String, Vec<String>)> {
    client
        .transcript()
        .0
        .iter()
        .filter(|line| line.object.starts_with(object_prefix))
        .map(|line| (line.object.clone(), line.event.clone(), line.args.clone()))
        .collect()
}

/// The names of the events of `object_prefix` objects, in order.
fn names_of(client: &TestClient, object_prefix: &str) -> Vec<String> {
    events_of(client, object_prefix)
        .into_iter()
        .map(|(_, event, _)| event)
        .collect()
}

/// Whether `wanted` appears in `names` in this order (other events may lie between).
fn in_order(names: &[String], wanted: &[&str]) -> bool {
    let mut rest = names.iter();
    wanted
        .iter()
        .all(|name| rest.by_ref().any(|seen| seen == name))
}

const KEY_LEFTSHIFT: KeyCode = KeyCode(42);
const KEY_A: KeyCode = KeyCode(30);

/// A wheel detent arrives as axis_source(wheel), axis_value120(120), axis, frame (SP1 B1).
#[test]
fn axis_source_and_value120_are_sent() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 400, y: 300 });
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _layer = map(&mut fixture, id, &cover(Layer::Top, "scroll"));
    // One wheel detent downwards: 120 in value120, 15 logical pixels.
    fixture.input().axis(
        mouse,
        AxisFrame {
            source: AxisSource::Wheel,
            v120: (0, 120),
            px: SubPoint {
                x: SubPx(0),
                y: SubPx(15 * 120),
            },
            stopped: false,
        },
    );
    let names = names_of(fixture.client(id), "wl_pointer#");
    assert!(
        in_order(&names, &["axis_source", "axis_value120", "axis", "frame"]),
        "a wheel detent arrives as source, value120, axis, frame: {names:?}"
    );
    let value120 = events_of(fixture.client(id), "wl_pointer#")
        .into_iter()
        .find(|(_, event, _)| event == "axis_value120")
        .expect("axis_value120");
    assert_eq!(value120.2.last().map(String::as_str), Some("120"));
}

/// Repeat_info carries the keyboard settings' rate and delay.
#[test]
fn repeat_info_comes_from_settings() {
    let mut spec = FixtureSpec::desktop();
    spec.settings.keyboard.repeat_rate = RepeatRate(25);
    spec.settings.keyboard.repeat_delay = Duration::from_millis(600);
    let mut fixture = Fixture::start(spec).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    fixture.pump();
    let repeat = events_of(fixture.client(id), "wl_keyboard#")
        .into_iter()
        .find(|(_, event, _)| event == "repeat_info")
        .expect("repeat_info");
    assert_eq!(repeat.2, vec!["25".to_owned(), "600".to_owned()]);
}

/// A changed layout in the settings file reaches a bound keyboard as a new keymap; the same file
/// again, an unrelated change and a layout that does not compile send none.
#[test]
fn a_layout_change_in_the_settings_file_sends_a_new_keymap() {
    use casement::config::{CasementDoc, KeyboardSection, PointerSection};
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    fixture.pump();
    let keymaps = |fixture: &mut Fixture| {
        names_of(fixture.client(id), "wl_keyboard#")
            .into_iter()
            .filter(|name| name == "keymap")
            .count()
    };
    assert_eq!(keymaps(&mut fixture), 1, "the first keymap, at bind");
    let layout = |name: &str| CasementDoc {
        keyboard: KeyboardSection {
            xkb_layout: name.to_owned(),
            ..KeyboardSection::default()
        },
        ..CasementDoc::default()
    };

    fixture.settings_file_changed(&layout("de"));
    fixture.pump();
    assert_eq!(keymaps(&mut fixture), 2, "a new layout is a new keymap");

    fixture.settings_file_changed(&layout("de"));
    fixture.pump();
    assert_eq!(keymaps(&mut fixture), 2, "the same file again sends none");

    let unrelated = CasementDoc {
        pointer: PointerSection::default(),
        ..layout("de")
    };
    fixture.settings_file_changed(&unrelated);
    fixture.pump();
    assert_eq!(keymaps(&mut fixture), 2, "an unrelated section sends none");

    fixture.settings_file_changed(&layout("no_such_layout_zz"));
    fixture.pump();
    assert_eq!(
        keymaps(&mut fixture),
        2,
        "a layout that does not compile keeps the keymap"
    );
    assert_eq!(
        fixture.model().settings().keyboard.layout.layout,
        "de",
        "and the model keeps the names the keyboard runs with"
    );
}

/// Enter, then the modifiers it carries, then each key; a modifier key's own `modifiers` follow
/// its `key`.
///
/// The order is smithay's, and it is the right one: `wl_keyboard.modifiers` says the modifier
/// state "has changed, and it should update its local state", the protocol fixes only that
/// `modifiers` follows `enter`, and a `key` carries the raw evdev code, which a client reads
/// with the state it held when the key arrived. So Shift's press is `key`, then `modifiers`
/// (the state the following keys are read with), and Shift's release is `key`, then the
/// `modifiers` that clear it; the keys between need none.
#[test]
fn modifiers_ordering_transcript() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let keys = fixture.input().plug(
        DeviceClass::Keyboard,
        TestOrigin::Physical(DeviceClass::Keyboard),
    );
    // A launcher holds the keyboard exclusively only when it is the shell's (CASC-02).
    let id = fixture.connect(ConnectionRole::Shell);
    fixture.pump();
    let _keyboard = Keyboard::bind(fixture.client(id)).expect("a keyboard");
    let _launcher = LayerSurface::create(
        fixture.client(id),
        &LayerAsk {
            level: Layer::Overlay,
            namespace: "launcher",
            anchor: Anchor::empty(),
            exclusive_zone: 0,
            keyboard: KeyboardInteractivity::Exclusive,
            size: (400, 300),
        },
    )
    .expect("layer");
    fixture.pump();
    fixture.input().key(keys, KEY_LEFTSHIFT, PressEdge::Press);
    fixture.input().key(keys, KEY_A, PressEdge::Press);
    fixture.input().key(keys, KEY_A, PressEdge::Release);
    fixture.input().key(keys, KEY_LEFTSHIFT, PressEdge::Release);
    let names: Vec<String> = names_of(fixture.client(id), "wl_keyboard#")
        .into_iter()
        .filter(|name| name != "keymap" && name != "repeat_info")
        .collect();
    // enter and its modifiers; shift down (key, then the modifiers it set); a down and up; shift up.
    assert_eq!(
        names,
        [
            "enter",
            "modifiers",
            "key",
            "modifiers",
            "key",
            "key",
            "key",
            "modifiers"
        ]
    );
}

/// The surface now under a resting pointer receives enter at once (A16, SP1 A1).
#[test]
fn pointer_focus_is_recomputed_when_the_surface_under_it_unmaps() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let _pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let _below = map(&mut fixture, id, &cover(Layer::Background, "below"));
    fixture
        .input()
        .pointer_to(mouse, LogicalPoint { x: 400, y: 300 });
    let above = map(&mut fixture, id, &cover(Layer::Top, "above"));
    let entered = names_of(fixture.client(id), "wl_pointer#")
        .iter()
        .filter(|name| *name == "enter")
        .count();
    assert_eq!(
        entered, 2,
        "the pointer entered the surface that mapped over it"
    );
    // The surface under the resting pointer goes away; the one beneath it is entered at once.
    above.hide(fixture.client(id));
    fixture.pump();
    let names = names_of(fixture.client(id), "wl_pointer#");
    let last_enter = names.iter().rposition(|name| name == "enter");
    let last_leave = names.iter().rposition(|name| name == "leave");
    assert!(
        last_leave < last_enter && names.iter().filter(|name| *name == "enter").count() == 3,
        "leave then enter on the unmap, with no motion: {names:?}"
    );
}

/// A press the model swallowed (it only activated the window) takes its release with it: the
/// client sees no `button` event for the first click, and both for the second.
#[test]
fn a_swallowed_press_takes_its_release_with_it() {
    use crate::protocol::xdg_shell::{
        boot, click, map_toplevel, pointer as plug_pointer, settle, windows,
    };
    use casement_testclient::surface::toplevel::{WmBase, bind_pointer};
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let _pointer = bind_pointer(fixture.client(id));
    let _top = map_toplevel(&mut fixture, id, &base, (400, 300));
    settle(&mut fixture);
    let window = windows(&fixture)[0];
    let device = plug_pointer(&mut fixture);
    let buttons = |fixture: &mut Fixture| {
        fixture
            .client(id)
            .transcript()
            .0
            .iter()
            .filter(|line| line.object.starts_with("wl_pointer#") && line.event == "button")
            .count()
    };
    click(&mut fixture, device, window);
    assert_eq!(buttons(&mut fixture), 0, "the first click only activated");
    click(&mut fixture, device, window);
    assert_eq!(buttons(&mut fixture), 2, "the second click is the client's");
}
