//! The requests of `quire_shell_v1`, as the model takes them. The switcher, overview and show
//! desktop requests join at P3-Z with their machines.

use crate::geometry::LogicalRect;
use crate::ids::{LayerKey, WindowId};
use casement_protocols::vocab::{
    FullScreenBar, Motion, OverviewMode, ShellRole, Toggle, WindowEntry,
};
use ds_core::vocab::Fraction;

/// One request from the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShellRequest {
    /// A layer surface of the shell is the bar, the dock, the switcher and so on
    /// (`set_role`). The wire layer has already refused a surface that is not a layer surface
    /// of the shell's client and a role another live surface holds.
    Role {
        /// The surface.
        layer: LayerKey,
        /// Its role.
        role: ShellRole,
    },
    /// Where `window` minimises to (`set_minimise_target`): `local`, in the coordinates of the
    /// `dock` layer surface, which the model turns into global logical pixels.
    MinimiseTarget {
        /// The dock surface the rectangle is local to.
        dock: LayerKey,
        /// The window.
        window: WindowId,
        /// The rectangle, local to `dock`.
        local: LogicalRect,
    },
    /// Forget `window`'s minimise target (`clear_minimise_target`).
    ClearMinimiseTarget {
        /// The window.
        window: WindowId,
    },
    /// The person picked `entry` in the window menu the shell drew for `window`
    /// ([`ShellNote::WindowMenu`](super::ShellNote::WindowMenu)). The model carries it out only
    /// when the entry is enabled for the window's state now.
    WindowMenuPick {
        /// The window.
        window: WindowId,
        /// The entry.
        entry: WindowEntry,
    },
    /// The person picked `window` for the other half of the Split View being chosen
    /// ([`ShellNote::SplitChoose`](super::ShellNote::SplitChoose)). A window that is not among
    /// the candidates now, or no choosing, changes nothing.
    SplitPick {
        /// The window picked.
        window: WindowId,
    },
    /// The person pressed Escape or clicked elsewhere: the first window stays alone, full screen.
    SplitCancel,
    /// The person moved the divider of the Split View `window` is in: the leading window now
    /// takes `share` of the width (thousandths; the model keeps it between 200 and 800, and
    /// layout holds each half to its window's minimum width).
    SplitDivide {
        /// Either window of the pair.
        window: WindowId,
        /// The leading window's share.
        share: Fraction,
    },
    /// The shell opened or closed an overview (`overview`: its key or gesture). Only Mission
    /// Control is built; the others change nothing.
    Overview {
        /// Which overview.
        mode: OverviewMode,
        /// Open (on) or close (off).
        toggle: Toggle,
    },
    /// The shell chose what the bar does in full screen (`set_full_screen_bar`, version 7). The
    /// full-screen windows already shown are laid out and configured again.
    FullScreenBar(FullScreenBar),
    /// The shell said how much motion the person allows (`set_motion`, version 7).
    Motion(Motion),
}
