//! The closed set of inputs to [`crate::Model::step`]. Each delta freeze extends it.

use crate::app::{AppKey, MenuSource};
use crate::desktop::DesktopRequest;
use crate::focus::TokenCheck;
use crate::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use crate::ids::{ClientKey, InputSerial, LayerKey, OutputKey, WindowId};
use crate::input::InputEvent;
use crate::layer::LayerSpec;
use crate::lock::LockEvent;
use crate::output::{GammaRamp, MinuteOfDay, OutputChangeSet, OutputConfig, OutputFacts};
use crate::policy::ConnectionRole;
use crate::privilege::{Book, ClientStanding, Floor, Subject, UnixSeconds};
use crate::settings::CasementSettings;
use crate::shell::ShellRequest;
use crate::space::{SpaceFile, WorkspaceRequest};
use crate::time::TimerKey;
use crate::window::{
    A11yRoot, FirstMouse, FrameHint, Parent, SizeLimits, WindowOrigin, WindowTitle,
    WlrToplevelRequest,
};
use casement_protocols::vocab::{
    Answer, AskId, Edited, Modality, PlaceTarget, Power, Toggle, WindowRole,
};
use porter_core::{ClaimedId, PeerIdentity};

/// Something that happened, as the edge (a protocol handler, the backend, the clock) reports
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// A client connected.
    ClientConnected {
        /// The client.
        client: ClientKey,
        /// Who its process is.
        peer: PeerIdentity,
        /// Which socket it came through.
        role: ConnectionRole,
    },
    /// A client disconnected.
    ClientGone {
        /// The client.
        client: ClientKey,
    },
    /// A toplevel mapped (its first commit with a buffer after the initial configure).
    ToplevelMapped(MappedToplevel),
    /// A toplevel unmapped or was destroyed.
    ToplevelUnmapped {
        /// The window.
        window: WindowId,
    },
    /// A toplevel committed a new size.
    ToplevelCommitted {
        /// The window.
        window: WindowId,
        /// Its committed size.
        size: LogicalSize,
    },
    /// A toplevel asked for something.
    ToplevelRequest {
        /// The window.
        window: WindowId,
        /// What it asked.
        request: ToplevelRequest,
    },
    /// A toplevel's metadata changed.
    ToplevelMeta {
        /// The window.
        window: WindowId,
        /// What changed.
        meta: ToplevelMeta,
    },
    /// An interactive move or resize (the person dragging) put a window at this rectangle.
    WindowDragged {
        /// The window.
        window: WindowId,
        /// Where it now is, in global logical coordinates.
        rect: LogicalRect,
    },
    /// The person let go of the window an interactive move or resize was dragging: a picture in
    /// picture window snaps to the nearest corner now.
    WindowDragEnded {
        /// The window.
        window: WindowId,
    },
    /// A window was put by hand with its top-left at `rect`'s, wherever that is, even partly
    /// outside the usable area: the wlcs shim's `position_window_absolute`. No protocol sends
    /// it. Layout takes the rectangle as given (the size is still held to the window's limits
    /// and the area) until the person drags the window or it unmaps.
    WindowPlaced {
        /// The window.
        window: WindowId,
        /// Where it goes, in global logical coordinates.
        rect: LogicalRect,
    },
    /// A window's `quire_window_v1` hint changed (C50).
    WindowHint {
        /// The window.
        window: WindowId,
        /// What changed.
        hint: WindowHintChange,
    },
    /// A mapped toplevel's dialog hint changed (`xdg_dialog` `set_modal` / `unset_modal` after
    /// the map; a hint set before it travels in [`MappedToplevel::modality`]).
    ToplevelModality {
        /// The window.
        window: WindowId,
        /// What it now blocks.
        modality: Modality,
    },
    /// A layer surface mapped.
    LayerMapped {
        /// The surface.
        layer: LayerKey,
        /// What it asked for.
        spec: LayerSpec,
    },
    /// A layer surface changed what it asks for.
    LayerChanged {
        /// The surface.
        layer: LayerKey,
        /// What it now asks for.
        spec: LayerSpec,
    },
    /// A layer surface was put by hand with its top-left at `at` (global logical pixels), where
    /// its anchors would not have put it: the wlcs shim's `position_window_absolute`. No protocol
    /// sends it. A pin lasts until the surface unmaps.
    LayerPlaced {
        /// The surface.
        layer: LayerKey,
        /// Where its top-left goes.
        at: LogicalPoint,
    },
    /// A layer surface unmapped.
    LayerUnmapped {
        /// The surface.
        layer: LayerKey,
    },
    /// The session lock moved.
    Lock(LockEvent),
    /// Input arrived.
    Input(InputEvent),
    /// An output was plugged.
    OutputPlugged(OutputFacts),
    /// An output was unplugged.
    OutputUnplugged(OutputKey),
    /// An output's configuration was applied.
    OutputConfigured {
        /// The output.
        output: OutputKey,
        /// Its configuration.
        config: OutputConfig,
    },
    /// A timer fired.
    TimerFired(TimerKey),
    /// A client answered a ping.
    Pong {
        /// The client.
        client: ClientKey,
    },
    /// A client holds (`On`) or no longer holds (`Off`) a `quire_window_manager_v1` bound at
    /// version 2 or later, so it can be sent `quit` when its app is asked to quit.
    QuitListener {
        /// The client.
        client: ClientKey,
        /// Whether it listens.
        toggle: Toggle,
    },
    /// The settings changed.
    SettingsChanged(Box<CasementSettings>),
    /// A client asked to change an output (`zwlr_output_manager_v1`, `zwlr_output_power_v1`);
    /// the model checks it against the display's facts and answers with `ApplyOutput` or a
    /// note (P1-M1).
    OutputRequested {
        /// The output.
        output: OutputKey,
        /// What it asked to change.
        change: OutputChangeSet,
    },
    /// A client took control of an output's gamma, or gave it back with `Identity`
    /// (`zwlr_gamma_control_v1`; P1-M1).
    GammaSet {
        /// The output.
        output: OutputKey,
        /// The ramp.
        ramp: GammaRamp,
    },
    /// The local time of day is now this minute (night light; read by the loop's clock, a new
    /// minute at a time).
    NightLightTick(MinuteOfDay),
    /// A window-list client asked something of a window (`zwlr_foreign_toplevel_handle_v1`;
    /// P1-M2).
    WlrToplevel {
        /// The window.
        window: WindowId,
        /// What it asked.
        request: WlrToplevelRequest,
    },
    /// A workspace client asked something of the Spaces (`ext_workspace_manager_v1`; P1-M2).
    Workspace(WorkspaceRequest),
    /// The Spaces file was read at start: the model adopts the stored ids, so a Space is the
    /// same Space after a restart (P1-M2).
    SpacesLoaded(Box<SpaceFile>),
    /// The shell managed the desktop (`quire_desktop_manager_v1`; P2-Z). The wire layer has
    /// refused these on an agent connection, so every one is the person's.
    Desktop(DesktopRequest),
    /// The shell told the compositor something (`quire_shell_v1`; P2-Z).
    Shell(ShellRequest),
    /// The composition root says who a public client's program is, once it has looked at the
    /// connection. Until it does, a public client is unproven and can only be asked for
    /// Allow Once. A standing for a connection that is not public is ignored.
    ClientStanding {
        /// The client.
        client: ClientKey,
        /// Who its program is.
        standing: ClientStanding,
    },
    /// A client used a power that needs the person's consent, for the first time or after a
    /// refusal. The model answers with [`Effect::Resolve`](crate::Effect::Resolve), at once or
    /// after the person has answered.
    PrivilegeUse {
        /// The client.
        client: ClientKey,
        /// The power.
        power: Power,
    },
    /// The person answered a question on the shell's screen (`permission_answer`). `at` is the
    /// wall-clock second the root read; it only stamps the book.
    PermissionAnswered {
        /// The question.
        ask: AskId,
        /// What the person said.
        answer: Answer,
        /// When.
        at: UnixSeconds,
    },
    /// Settings switched a program's power on.
    PrivilegeGranted {
        /// The program or app.
        subject: Subject,
        /// The power.
        power: Power,
        /// When.
        at: UnixSeconds,
    },
    /// Settings switched a program's power off: every connected client of it loses the power
    /// at once.
    PrivilegeRevoked {
        /// The program or app.
        subject: Subject,
        /// The power.
        power: Power,
        /// When.
        at: UnixSeconds,
    },
    /// Settings forgot a program's answer ("Ask Again"): it is asked at its next use, and a
    /// connected client of it loses the power now.
    PrivilegeForgotten {
        /// The program or app.
        subject: Subject,
        /// The power.
        power: Power,
    },
    /// The shell can be asked a question: it bound `quire_shell_v1` at the version that
    /// carries `permission_ask`.
    ShellReady,
    /// The shell cannot be asked any more (its connection or its protocol object ended).
    /// Questions that were on its screen wait for the next shell.
    ShellGone,
    /// The person's book was read at start.
    BookLoaded(Box<Book>),
    /// The administrator's floor was read at start, or read again.
    FloorLoaded(Box<Floor>),
}

/// A toplevel that just mapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedToplevel {
    /// Its id.
    pub window: WindowId,
    /// Its client.
    pub client: ClientKey,
    /// Its app.
    pub app: AppKey,
    /// Whether its client is a Wayland program or the X11 satellite.
    pub origin: WindowOrigin,
    /// The app id it declared.
    pub claimed_id: ClaimedId,
    /// Its role.
    pub role: WindowRole,
    /// What it blocks.
    pub modality: Modality,
    /// Its parent.
    pub parent: Parent,
    /// Its size limits.
    pub limits: SizeLimits,
    /// The size of its first buffer.
    pub first_size: LogicalSize,
    /// The xdg-activation token it mapped with.
    pub activation: TokenCheck,
}

/// What a toplevel asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToplevelRequest {
    /// An interactive move, answering this press.
    Move {
        /// The press's serial.
        serial: InputSerial,
    },
    /// An interactive resize from these edges, answering this press.
    Resize {
        /// The press's serial.
        serial: InputSerial,
        /// The edges.
        edges: ResizeEdges,
    },
    /// Maximise (zoom) on or off.
    Maximise(Toggle),
    /// Minimise.
    Minimise,
    /// Full screen on or off, optionally on one output.
    FullScreen {
        /// On or off.
        toggle: Toggle,
        /// The output it asked for.
        output: Option<OutputKey>,
    },
    /// Activation with a token.
    Activate(TokenCheck),
    /// Show the window menu here.
    WindowMenu {
        /// Where, in global logical coordinates.
        at: LogicalPoint,
    },
    /// Placement (`quire_window_v1.request_placement`), answering this press.
    Place {
        /// Where.
        target: PlaceTarget,
        /// The press's serial.
        serial: InputSerial,
    },
}

/// The edges an interactive resize moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResizeEdges {
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
    /// The top-left corner.
    TopLeft,
    /// The top-right corner.
    TopRight,
    /// The bottom-left corner.
    BottomLeft,
    /// The bottom-right corner.
    BottomRight,
}

/// A toplevel's metadata change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToplevelMeta {
    /// Its title.
    Title(WindowTitle),
    /// Its declared app id.
    ClaimedId(ClaimedId),
    /// Its parent.
    Parent(Parent),
    /// Its size limits.
    Limits(SizeLimits),
}

/// A `quire_window_v1` hint change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowHintChange {
    /// Its first-mouse regions.
    FirstMouse(FirstMouse),
    /// Its frame.
    Frame(FrameHint),
    /// Its role.
    Role(WindowRole),
    /// Its accessibility root.
    A11y(A11yRoot),
    /// Whether its document is edited.
    Edited(Edited),
    /// Where its global menu lives (the KDE appmenu protocol, `gtk_shell1`).
    Menu(MenuSource),
}
