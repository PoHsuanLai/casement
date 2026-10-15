//! Key bindings: chords and what they do (C30: the settings document carries the defaults).

use super::{Chord, Origin};
use crate::window::SplitSide;
use casement_protocols::vocab::PlaceTarget;
use serde::{Deserialize, Serialize};

/// What a binding does.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BindingAction {
    /// Call a D-Bus method (sill's launcher, the screenshot tool).
    Call(DbusCall),
    /// Start a program.
    Spawn(Argv),
    /// A desktop command casement carries out itself.
    Desktop(DesktopCommand),
}

/// A D-Bus method call on the session bus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DbusCall {
    /// The bus name.
    pub service: String,
    /// The object path.
    pub path: String,
    /// The interface.
    pub interface: String,
    /// The method.
    pub method: String,
    /// String arguments, in order.
    pub args: Vec<String>,
}

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Argv(pub Vec<String>);

/// A command casement carries out itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DesktopCommand {
    /// Close the key window.
    CloseKey,
    /// Minimise the key window.
    MinimiseKey,
    /// Hide the active app.
    HideActive,
    /// Hide every app but the active one.
    HideOthers,
    /// Quit the active app.
    QuitActive,
    /// Show the force-quit panel.
    ForceQuitPanel,
    /// Show another Space.
    SwitchSpace(SpaceStep),
    /// Place the key window.
    Place(PlaceTarget),
    /// Open Mission Control, or close it when it is open.
    MissionControl,
    /// Tile the key window to one side of the screen in a Split View (Window, Tile Window to
    /// Left / Right of Screen). Bound to no chord by default, as on the Mac.
    SplitTile(SplitSide),
}

/// Which Space to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SpaceStep {
    /// The one to the left.
    Previous,
    /// The one to the right.
    Next,
    /// The n-th, counting from 1.
    Number(u8),
}

/// One binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Binding {
    /// The chord.
    pub chord: Chord,
    /// What it does.
    pub action: BindingAction,
}

/// Every binding, in the order the settings list them (the first match wins).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bindings(pub Vec<Binding>);

/// What a chord resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// This binding's action.
    Action(BindingAction),
    /// Nothing: the key goes to the focused client.
    PassThrough,
}

/// What `chord` from `origin` resolves to. Agent and remote origins never fire a binding, and
/// neither does a feed (I1, I8: only the person's hands, a virtual keyboard they use, or the
/// tests, drive the desktop's own shortcuts).
pub fn resolve(bindings: &Bindings, chord: &Chord, origin: &Origin) -> Resolved {
    if !origin.fires_bindings() {
        return Resolved::PassThrough;
    }
    bindings
        .0
        .iter()
        .find(|binding| binding.chord == *chord)
        .map_or(Resolved::PassThrough, |binding| {
            Resolved::Action(binding.action.clone())
        })
}
