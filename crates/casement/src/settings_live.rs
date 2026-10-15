//! The stored settings followed while casement runs: when `casement/settings.toml` changes, the
//! pointer's settings reach the devices already present and the key repeat reaches the clients
//! already bound, at once, as a change in System Settings does on the Mac. (The core loop does
//! the reaching when the model is told, `core_loop::feed`.)
//!
//! Only these parts follow the file for now. The keymap's layout is compiled when the seat
//! starts, so a changed layout still waits for a restart; the rest of the document is read at
//! start.

use crate::State;
use crate::appearance::ThreadSpawner;
use crate::cli::ConfigHome;
use crate::config::{self, CasementDoc};
use casement_core::Event;
use casement_core::settings::{CasementSettings, KeyboardSettings};
use ds_core::spawner::Spawner;
use smithay::reexports::calloop::channel::{self, Event as ChannelEvent};

/// `current` with the parts that follow the file taken from `doc`: the pointer's settings and
/// the key repeat. Everything else, the keymap's bindings and layout among it, stays.
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
        keyboard: KeyboardSettings {
            repeat_delay: read.keyboard.repeat_delay,
            repeat_rate: read.keyboard.repeat_rate,
            ..current.keyboard.clone()
        },
        ..current.clone()
    }
}

/// Stamps the file's version (a file written before the file had one, once), then follows the
/// file. A file that cannot be watched is logged and the settings read at start are kept.
pub fn attach(state: &mut State, home: &ConfigHome) -> Result<(), crate::StartError> {
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
    let next = followed(state.settings(), doc);
    if *state.settings() == next {
        return;
    }
    crate::core_loop::feed(state, vec![Event::SettingsChanged(Box::new(next))]);
}
