//! The closed set of outputs of [`crate::Model::step`]: `casement::exec` is the one place
//! they are carried out.

use crate::app::{AppKey, Termination};
use crate::cause::Cause;
use crate::desktop::DesktopNote;
use crate::event::ResizeEdges;
use crate::focus::PressRoute;
use crate::geometry::LogicalSize;
use crate::ids::{ClientKey, InputSerial, LayerKey, OutputKey, WindowId};
use crate::input::{Argv, DbusCall};
use crate::lock::LockReply;
use crate::output::{GammaRamp, OutputConfig};
use crate::shell::ShellNote;
use crate::time::{Due, TimerKey};
use casement_protocols::vocab::{Attention, Emphasis};

/// Something the model wants done.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Effect {
    /// Send a toplevel a configure.
    Configure {
        /// The window.
        window: WindowId,
        /// What to send.
        configure: ConfigureSpec,
    },
    /// List a window on `ext_foreign_toplevel_list_v1`, or send the title and app id it now has.
    ListWindow(WindowId),
    /// Withdraw a window from the foreign toplevel list.
    UnlistWindow(WindowId),
    /// Ask a toplevel to close.
    SendClose {
        /// The window.
        window: WindowId,
    },
    /// Tell a client the person asked its app to quit (`quire_window_manager_v1.quit`, sent in
    /// place of closing the app's windows to a client that listens for it).
    SendQuit {
        /// The client.
        client: ClientKey,
    },
    /// Move keyboard focus.
    Keyboard(KeyboardTarget),
    /// Raise these windows, bottom first.
    Raise(Vec<WindowId>),
    /// Route the pending press.
    Route(PressRoute),
    /// Start an interactive grab.
    StartGrab(GrabSpec),
    /// These windows' emphasis changed.
    Emphasis(Vec<(WindowId, Emphasis)>),
    /// Ping a client.
    Ping {
        /// The client.
        client: ClientKey,
    },
    /// Ring the bell for an app.
    Bell {
        /// The app.
        app: AppKey,
    },
    /// An app's attention changed.
    Attention {
        /// The app.
        app: AppKey,
        /// The level.
        level: Attention,
    },
    /// Arm a timer.
    ArmTimer(Due),
    /// Cancel a timer.
    CancelTimer(TimerKey),
    /// Lay the scene out again, for this reason.
    Relayout(Cause),
    /// Ask for frames on an output.
    RequestFrames(OutputKey),
    /// Send a layer surface a configure.
    ConfigureLayer {
        /// The surface.
        layer: LayerKey,
        /// Its size.
        size: LogicalSize,
    },
    /// Carry out a lock reply.
    Lock(LockReply),
    /// Apply an output configuration.
    ApplyOutput {
        /// The output.
        output: OutputKey,
        /// The configuration.
        config: OutputConfig,
    },
    /// Disconnect a client with a protocol error.
    Disconnect {
        /// The client.
        client: ClientKey,
        /// Why.
        fault: ProtocolFault,
    },
    /// A diagnostic.
    Note(CoreNote),
    /// A binding calls a method on the session bus (sill's launcher, the screenshot tool):
    /// carried out through the injected `ShellCalls` seam (P1-I1).
    CallBinding(DbusCall),
    /// A binding starts a program, through the injected `Spawner` seam (P1-I1).
    Spawn(Argv),
    /// Drive this output with this gamma ramp (`zwlr_gamma_control_v1`, night light; P1-M1).
    SetGamma(OutputKey, GammaRamp),
    /// Write the Spaces file: the Space ids changed (P1-M2).
    PersistSpaces,
    /// Send `ext_workspace` clients the Spaces as they now are (P1-M2).
    PublishWorkspaces,
    /// Send `zwlr_foreign_toplevel_management` clients this window's title, app id, state and
    /// outputs as they now are (P1-M2).
    PublishToplevel(WindowId),
    /// Send `quire_desktop_v1` clients this object as it now is (P2-Z; the model emits it with
    /// the lane P2-Q1).
    Desktop(DesktopNote),
    /// Record or send this to the shell (`quire_shell_v1`; P2-Z).
    Shell(ShellNote),
    /// Stop an app's processes with a signal (force quit; P3-QU).
    Terminate(Termination),
}

/// A toplevel configure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigureSpec {
    /// The size to ask for.
    pub size: ConfigureSize,
    /// The states to send.
    pub states: ToplevelStates,
    /// `xdg_toplevel.configure_bounds`: the largest useful size.
    pub bounds: LogicalSize,
}

/// The size a configure asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigureSize {
    /// The client chooses (`0x0`).
    Keep,
    /// This size.
    Exact(LogicalSize),
}

/// The `xdg_toplevel` states a configure carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelStates {
    /// `activated`.
    pub activation: Activation,
    /// `maximized`.
    pub maximised: Held,
    /// `fullscreen`.
    pub full_screen: Held,
    /// `tiled_*`.
    pub tiled: TiledEdges,
    /// `resizing`.
    pub resizing: Held,
}

/// Whether a toplevel is drawn active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Activation {
    /// Active.
    Active,
    /// Inactive.
    Inactive,
}

/// Whether a toplevel state holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Held {
    /// It holds.
    Yes,
    /// It does not.
    No,
}

/// Which edges of a toplevel touch a neighbour or the usable area's edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TiledEdges {
    /// `tiled_top`.
    pub top: Held,
    /// `tiled_bottom`.
    pub bottom: Held,
    /// `tiled_left`.
    pub left: Held,
    /// `tiled_right`.
    pub right: Held,
}

/// Who gets the keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyboardTarget {
    /// A window.
    Window(WindowId),
    /// A layer surface.
    Layer(LayerKey),
    /// The lock surface on an output.
    LockSurface(OutputKey),
    /// Nothing.
    Nothing,
}

/// An interactive grab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabSpec {
    /// A move.
    Move {
        /// The window.
        window: WindowId,
        /// The press it answers.
        serial: InputSerial,
    },
    /// A resize.
    Resize {
        /// The window.
        window: WindowId,
        /// The press it answers.
        serial: InputSerial,
        /// The edges it moves.
        edges: ResizeEdges,
    },
}

/// Why a client is disconnected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtocolFault {
    /// A serial that answers no press.
    BadSerial,
    /// A role the surface cannot take.
    BadRole,
    /// A request its connection may not make.
    Privilege,
}

/// A diagnostic the edge logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreNote {
    /// A window's activation was refused (A7).
    RefusedActivation(WindowId),
    /// A window that was not key asked to go full screen and was refused (CASC-07).
    RefusedFullScreen(WindowId),
    /// A timer fired for something that is gone.
    StaleTimer(TimerKey),
    /// An event named a window the model does not know.
    UnknownWindow(WindowId),
    /// A client's request to change an output was refused (P1-M1).
    RefusedOutput(crate::output::OutputRefusal),
    /// A desktop request the model does not carry out yet (P2-Z).
    Deferred(Deferred),
}

/// A desktop request that waits for the machine that carries it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Deferred {
    /// Reopen an app with no windows: the launch intent (P4).
    Reopen,
}
