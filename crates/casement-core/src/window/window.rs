//! One window's record.

use super::{SizeLimits, WindowHints, WindowOrigin, WindowState, WindowTitle};
use crate::app::AppKey;
use crate::ids::{ClientKey, OutputKey, WindowId};
use casement_protocols::vocab::{Emphasis, Modality, SensitivityLevel, WindowRole};
use porter_core::{ClaimedId, SpaceId};

/// One toplevel window the model knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    /// Its id.
    pub id: WindowId,
    /// The client that owns it.
    pub client: ClientKey,
    /// The app it belongs to.
    pub app: AppKey,
    /// Whether it is a Wayland client's own window or an X11 program's.
    pub origin: WindowOrigin,
    /// What kind of window it is.
    pub role: WindowRole,
    /// What it blocks.
    pub modality: Modality,
    /// Which Space it is on.
    pub place: SpacePlace,
    /// The output a window on every Space sits on (A28): none for a window on one Space (its
    /// Space says), and none while no output is plugged.
    pub home: Option<OutputKey>,
    /// Its place in the stacking order, across the whole model.
    pub stack: StackRank,
    /// Floating, tiled, filled, full screen or minimised.
    pub state: WindowState,
    /// The window it belongs to, if any (a sheet's document, a dialog's owner).
    pub parent: Parent,
    /// Key, main or inactive.
    pub emphasis: Emphasis,
    /// What capture and the agent may see of it.
    pub sensitivity: SensitivityLevel,
    /// Its size limits.
    pub limits: SizeLimits,
    /// Its title (app-authored text).
    pub title: WindowTitle,
    /// The `xdg_toplevel.app_id` it declared.
    pub claimed_id: ClaimedId,
    /// What its client asked of the compositor (`quire_window_v1`).
    pub hints: WindowHints,
}

/// The window a window belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parent {
    /// None: a top-level document or panel.
    None,
    /// This window.
    Window(WindowId),
}

/// Which Space a window is on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpacePlace {
    /// This one.
    On(SpaceId),
    /// Every Space of its output.
    AllSpaces,
}

/// A window's place in the stacking order: the larger, the nearer the top. Distinct for every
/// window in the model, so the order within any Space is total (A28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StackRank(pub u64);
