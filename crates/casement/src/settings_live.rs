//! The stored settings followed while casement runs: when `casement/settings.toml` changes, the
//! pointer's settings reach the devices already present and the key repeat reaches the clients
//! already bound, at once, as a change in System Settings does on the Mac. (The core loop does
//! the reaching when the model is told, `core_loop::feed`.)
//!
//! The night light follows as well (its mode, times, warmth, fade and the displays it leaves
//! alone, which sill writes when it turns Night Shift on): the model re-drives every output.
//!
//! The keyboard layout follows too: a changed `keyboard.xkb_*` is compiled and put on the seat's
//! keyboard (every bound client gets the new keymap, as an input-source change does on the Mac).
//! A layout that does not compile keeps the old keymap, is reported once, and the model keeps
//! the names it runs with. The rest of the document is read at start.

use crate::State;
use crate::appearance::ThreadSpawner;
use crate::config::{self, CasementDoc};
use casement_core::Event;
use casement_core::settings::{CasementSettings, KeyboardSettings, XkbNames};
use casement_wayland::seat_keymap::{self, KeymapStep};
use ds_core::spawner::Spawner;
use smithay::reexports::calloop::channel::{self, Event as ChannelEvent};
use std::path::Path;

/// `current` with the parts that follow the file taken from `doc`: the pointer's settings, the
/// key repeat, the keyboard layout and the night light. Everything else, the keymap's bindings
/// among it, stays.
///
/// ```
/// use casement::config::CasementDoc;
/// use casement::settings_live::followed;
/// use casement_core::settings::CasementSettings;
///
/// let current = CasementSettings::defaults();
/// assert_eq!(followed(&current, &CasementDoc::default()), current);
/// ```
pub fn followed(current: &CasementSettings, doc: &CasementDoc) -> CasementSettings {
    let read = config::to_core(doc);
    CasementSettings {
        pointer: read.pointer,
        gestures: read.gestures,
        night_light: read.night_light,
        keyboard: KeyboardSettings {
            repeat_delay: read.keyboard.repeat_delay,
            repeat_rate: read.keyboard.repeat_rate,
            layout: read.keyboard.layout,
        },
        ..current.clone()
    }
}

/// Stamps the file's version (a file written before the file had one, once), then follows the
/// file. A file that cannot be watched is logged and the settings read at start are kept.
pub fn attach(state: &mut State, home: Option<&Path>) -> Result<(), crate::StartError> {
    if let Err(error) = config::stamp_version(home) {
        tracing::warn!(%error, "the settings file could not be marked with its version");
    }
    let (sender, receiver) = channel::channel::<CasementDoc>();
    let mut watch = config::watch(home, &ThreadSpawner);
    ThreadSpawner.spawn(Box::pin(async move {
        while let Some(loaded) = watch.changed().await {
            for line in loaded.diagnostics(<CasementDoc as ds_settings::SettingsDoc>::FILE) {
                tracing::warn!("{line}");
            }
            if sender.send(loaded.value).is_err() {
                return;
            }
        }
    }));
    state
        .event_loop
        .clone()
        .insert_source(receiver, |event, (), state: &mut State| {
            if let ChannelEvent::Msg(doc) = event {
                apply(state, &doc);
            }
        })
        .map(|_| ())
        .map_err(|error| crate::StartError::Loop(error.to_string()))
}

/// Puts the layout `doc` names on the seat's keyboard when it is not the one it has, and gives
/// the names the keyboard runs with afterwards: the new ones, or the old ones when the new did
/// not compile (reported once per set of names).
fn switch_keymap(state: &mut State, wanted: &XkbNames) -> XkbNames {
    if state.wl.seat.keymap.plan(wanted) == KeymapStep::Compile {
        let keyboard = state.wl.seat.keyboard.clone();
        match seat_keymap::switch_layout(&keyboard, state, wanted) {
            Ok(()) => state.wl.seat.keymap.switched(wanted.clone()),
            Err(fault) => {
                if state.wl.seat.keymap.refuse(wanted.clone()) {
                    tracing::warn!(%fault, "the keyboard layout in the settings did not compile; the old one stays");
                }
            }
        }
    }
    state.wl.seat.keymap.running().clone()
}

/// Tells the model the settings `doc` gives, when they differ from the ones it has.
///
/// ```no_run
/// use casement::State;
/// use casement::config::CasementDoc;
///
/// fn changed(state: &mut State, doc: &CasementDoc) {
///     casement::settings_live::apply(state, doc);
/// }
/// ```
pub fn apply(state: &mut State, doc: &CasementDoc) {
    let mut next = followed(state.settings(), doc);
    next.keyboard.layout = switch_keymap(state, &next.keyboard.layout);
    if *state.settings() == next {
        return;
    }
    crate::core_loop::feed(state, vec![Event::SettingsChanged(Box::new(next))]);
}
