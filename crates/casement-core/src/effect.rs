//! The closed set of outputs of [`crate::Model::step`]: `casement::exec` is the one place
//! they are carried out.

use crate::app::AppKey;
use crate::cause::Cause;
use crate::event::ResizeEdges;
use crate::focus::PressRoute;
use crate::geometry::LogicalSize;
use crate::ids::{ClientKey, InputSerial, LayerKey, OutputKey, WindowId};
use crate::lock::LockReply;
use crate::output::OutputConfig;
use crate::time::{Due, TimerKey};
use casement_protocols::vocab::{Attention, Emphasis};

/// Something the model wants done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Send a toplevel a configure.
    Configure {
        /// The window.
        window: WindowId,
        /// What to send.
        configure: ConfigureSpec,
    },
    /// Ask a toplevel to close.
    SendClose {
        /// The window.
        window: WindowId,
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
    /// A timer fired for something that is gone.
    StaleTimer(TimerKey),
    /// An event named a window the model does not know.
    UnknownWindow(WindowId),
}
