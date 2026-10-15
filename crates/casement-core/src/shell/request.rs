//! The requests of `quire_shell_v1`, as the model takes them. The switcher, overview and show
//! desktop requests join at P3-Z with their machines.

use crate::geometry::LogicalRect;
use crate::ids::{LayerKey, WindowId};
use crate::window::MenuEntry;
use casement_protocols::vocab::ShellRole;

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
        entry: MenuEntry,
    },
}
