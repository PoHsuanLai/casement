//! Who holds the keyboard: the lock, else a layer surface asking for it exclusively, else
//! the key window.

use super::Model;
use crate::effect::KeyboardTarget;
use crate::layer::{KeyboardInteractivity, LayerLevel};
use crate::lock::{LockState, LockSurfaces};

impl Model {
    /// Where keystrokes go now. While the session is locked that is the lock surface of the
    /// first output the locker covers, or nothing before it has covered one; windows and
    /// layers do not hear the keyboard until it is unlocked.
    pub fn keyboard_target(&self) -> KeyboardTarget {
        match &self.lock {
            LockState::Unlocked => self
                .exclusive_layer()
                .or_else(|| self.focus.key.map(KeyboardTarget::Window))
                .unwrap_or(KeyboardTarget::Nothing),
            LockState::Locked {
                surfaces: LockSurfaces::Covering(outputs),
            } => outputs
                .first()
                .cloned()
                .map_or(KeyboardTarget::Nothing, KeyboardTarget::LockSurface),
            LockState::Locking { .. }
            | LockState::Locked {
                surfaces: LockSurfaces::Pending,
            }
            | LockState::LockedWithoutSurface => KeyboardTarget::Nothing,
        }
    }

    /// The layer surface that holds the keyboard exclusively: among those above the windows
    /// that ask for it, the highest level, the newest.
    fn exclusive_layer(&self) -> Option<KeyboardTarget> {
        self.layers
            .iter()
            .filter(|(_, spec)| spec.keyboard == KeyboardInteractivity::Exclusive)
            .filter(|(_, spec)| matches!(spec.level, LayerLevel::Top | LayerLevel::Overlay))
            .max_by_key(|(key, spec)| (spec.level, **key))
            .map(|(key, _)| KeyboardTarget::Layer(*key))
    }
}
