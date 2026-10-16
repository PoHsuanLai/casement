//! Who holds the keyboard: the lock, else a layer surface asking for it exclusively, else an
//! on-demand layer surface that has it (it mapped above the windows with nothing exclusive holding the keyboard, or was pressed), else the
//! key window.

use super::Model;
use crate::effect::KeyboardTarget;
use crate::layer::{KeyboardInteractivity, LayerLevel};
use crate::lock::{LockState, LockSurfaces};

impl Model {
    /// Where keystrokes go now. While the session is locked that is the lock surface of the
    /// output the person is working on when the locker covers it (the key window's output, else
    /// the first lit one), else of the first output it covers, or nothing before it has
    /// covered one; windows and layers do not hear the keyboard until it is unlocked.
    pub fn keyboard_target(&self) -> KeyboardTarget {
        match &self.lock {
            LockState::Unlocked => self
                .exclusive_layer()
                .or_else(|| self.demanding_layer())
                .or_else(|| self.focus.key.map(KeyboardTarget::Window))
                .unwrap_or(KeyboardTarget::Nothing),
            LockState::Locked {
                surfaces: LockSurfaces::Covering(outputs),
            } => self
                .working_output()
                .filter(|working| outputs.contains(working))
                .or_else(|| outputs.first().cloned())
                .map_or(KeyboardTarget::Nothing, KeyboardTarget::LockSurface),
            LockState::Locking { .. }
            | LockState::Locked {
                surfaces: LockSurfaces::Pending,
            }
            | LockState::LockedWithoutSurface => KeyboardTarget::Nothing,
        }
    }

    /// The on-demand layer surface that holds the keyboard, if it is still mapped and still
    /// asks to take it on demand.
    fn demanding_layer(&self) -> Option<KeyboardTarget> {
        self.demand
            .filter(|key| {
                self.layers
                    .get(key)
                    .is_some_and(|spec| spec.keyboard == KeyboardInteractivity::OnDemand)
            })
            .map(KeyboardTarget::Layer)
    }

    /// The layer surface that holds the keyboard exclusively: among those above the windows
    /// that ask for it, the highest level, the newest.
    pub(super) fn exclusive_layer(&self) -> Option<KeyboardTarget> {
        self.layers
            .iter()
            .filter(|(_, spec)| spec.keyboard == KeyboardInteractivity::Exclusive)
            .filter(|(_, spec)| matches!(spec.level, LayerLevel::Top | LayerLevel::Overlay))
            .max_by_key(|(key, spec)| (spec.level, **key))
            .map(|(key, _)| KeyboardTarget::Layer(*key))
    }
}
