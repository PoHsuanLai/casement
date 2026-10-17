//! The composition root's state (FROZEN: lanes never edit it; a change is an interface ask).

use crate::cli::{DirectTty, StartConfig, StartMode, TtyMode};
use crate::config::{self, CasementDoc, ConfigError};
use crate::seams::Seams;
use crate::time::ClockSource;
use crate::xwayland::XwaylandState;
use casement_backend::tty::{RenderMode, TtySeams};
use casement_backend::{
    BackendChoice, BackendError, BackendKind, DmabufMode, HeadlessBackend, HeadlessConfig,
    TtyBackend, WinitBackend,
};
use casement_core::frame::RepeatGate;
use casement_core::geometry::LogicalPoint;
use casement_core::gpu::GpuChoice;
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use casement_core::{IdMint, Model, OutputKey, OutputName, WindowId};
use casement_layout::{Arrangement, LayoutKnobs, RustScene};
use casement_motion::MotionBook;
use casement_render::{ChromeLayer, CursorDraw, CursorShape, TextureBook};
use casement_wayland::{
    ClientMint, GlobalFilter, WaylandFault, WaylandStates, alpha_modifier, background_effect,
    capture_source, color_management, commit_timing, compositor, content_type, cursor_shape,
    data_device, dmabuf, ext_data_control, fifo, foreign_toplevel_list, fractional_scale, gamma,
    idle_inhibit, idle_notify, image_capture, input_method, layer_shell, output, output_management,
    output_power, pointer_constraints, pointer_gestures, presentation, primary_selection, quire,
    quire_window, relative_pointer, seat, security_context, session_lock, shm, shortcuts_inhibit,
    single_pixel, subcompositor, system_bell, tablet, text_input, viewporter, virtual_keyboard,
    wlr_data_control, wlr_toplevel, workspace, xdg_activation, xdg_decoration, xdg_dialog,
    xdg_foreign, xdg_shell, xdg_toplevel_icon, xdg_toplevel_tag,
};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_server::{Display, DisplayHandle};
use std::collections::BTreeMap;

/// How often a fault that repeats at every frame is logged again.
const FAULT_LOG_EVERY: std::time::Duration = std::time::Duration::from_secs(30);

/// Everything casement holds, as typed sub-states.
#[derive(Debug)]
pub struct State {
    /// The model, the layout and motion.
    pub core: CoreState,
    /// Every protocol's sub-state.
    pub wl: WaylandStates<State>,
    /// The backend.
    pub backend: BackendKind,
    /// What the renderer keeps between frames.
    pub render: RenderState,
    /// The agent edge (fields from P2-Z and P4-Z).
    pub agent: AgentState,
    /// The D-Bus servers.
    pub ipc: IpcState,
    /// The session clock (the only clock, `time/`).
    pub clock: ClockSource,
    /// What the timing lines (`timing`) remember between the moments.
    pub timing: crate::timing::Timing,
    /// The injected file, bus and process seams (A1, A4, A5).
    pub seams: Seams,
    /// X11 clients through xwayland-satellite (P1-X1 starts it; off until then).
    pub xwayland: XwaylandState,
    /// Whether this run is the crash guard's `--safe` run.
    pub start_mode: StartMode,
    /// The process behind each public connection, for force quit.
    pub processes: crate::clients::Processes,
    /// Who holds each live connection, for the connection caps.
    pub roster: crate::clients::Roster,
    /// The looks at the programs behind new public connections, on threads of their own.
    pub probes: crate::clients::Probes,
    /// The display, for creating globals, inserting clients and sending events. (Not in the
    /// plan's sketch; the handlers need it: FINDINGS.)
    pub display: DisplayHandle,
    /// The event loop, for inserting sources (security-context listeners, timers).
    pub event_loop: LoopHandle<'static, State>,
    /// A37: whether a buffer attached before the first ack is an error (`testing` only).
    #[cfg(feature = "testing")]
    pub ack_policy: crate::relax::AckPolicy,
}

/// The model and what lays it out and animates it.
#[derive(Debug)]
pub struct CoreState {
    /// The model.
    pub model: Model,
    /// The layout.
    pub layout: RustScene,
    /// The last arrangement.
    pub arrangement: Arrangement,
    /// The motion legs in flight.
    pub motion: MotionBook,
    /// Layout's tunables, from the settings.
    pub knobs: LayoutKnobs,
    /// Window, layer and lease ids, minted by the edge that sees them (not in the sketch:
    /// FINDINGS).
    pub mint: IdMint,
}

/// What the renderer keeps between frames.
#[derive(Debug)]
pub struct RenderState {
    /// Which surface tree draws each window, layer and lock surface.
    pub textures: TextureBook,
    /// One damage tracker per output.
    pub trackers: BTreeMap<OutputKey, OutputDamageTracker>,
    /// The cursor and lock covers.
    pub chrome: ChromeLayer,
    /// Whether the pointer has had its first position.
    pub pointer: PointerStart,
    /// The surfaces told which outputs they are on.
    pub presence: crate::core_loop::Presence,
    /// Frame failures, so one that repeats every vblank is logged once per window.
    pub frame_faults: RepeatGate<OutputName>,
    /// The server-side frame part the pointer is on, if any.
    pub(crate) frame_over: Option<crate::core_loop::FrameOver>,
    /// The frame of each framed window as it last was, to draw once the window has closed.
    pub(crate) closing: BTreeMap<WindowId, crate::core_loop::Kept>,
    /// The last image of each window about to close, to fade with its frame.
    pub(crate) bodies: crate::core_loop::Bodies,
    /// The pointer and a Split View divider: over its strip, or dragging it.
    pub(crate) divider: crate::core_loop::DividerPointer,
    /// A window move that may tile: the zone its pointer arms.
    pub(crate) tile_drag: casement_core::window::TileDrag,
    /// The last titlebar press, for the double click.
    pub clicks: casement_core::window::ClickTracker,
    /// The traffic light held down, if any (a light acts on release).
    pub light_arm: casement_core::window::LightArm,
    /// Hover intent on the green light: when it rests long enough the window menu shows.
    pub light_dwell: casement_core::window::Dwell,
    /// The person's light or dark scheme, which the server-side frame is drawn in.
    pub scheme: casement_layout::Scheme,
    /// Whether the person reduced transparency (quire's `appearance.transparency`): every blur is
    /// drawn as its mostly opaque stand-in.
    pub transparency: casement_layout::Transparency,
    /// The window titles' rasteriser (`casement-chrome`), made when the first server-side frame
    /// needs a title so a session without one never scans the fonts.
    pub titles: Option<casement_chrome::TitleRaster>,
    /// The window Mission Control's pointer is on, if any.
    pub(crate) overview_hover: Option<casement_core::WindowId>,
    /// The press on a slot of Mission Control: pending, or dragging a window to a Space.
    pub(crate) overview_drag: casement_core::overview_drag::Drag,
    /// The rasteriser of the title under that window (no margins), made when first needed.
    pub labels: Option<casement_chrome::TitleRaster>,
    /// The app icons under the windows in Mission Control, found when it opens.
    pub icons: crate::icons::IconBook,
}

/// Whether the pointer has been put where it starts: the centre of the primary output, once an
/// output is plugged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerStart {
    /// No output has been plugged yet: the pointer is nowhere.
    Unplaced,
    /// It has been placed; from here only input moves it.
    Placed,
}

/// The agent edge's state (P2-Z: who was handed a privileged connection; P4-Z adds the seat,
/// the leases and the kill chord).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentState {
    /// The app each privileged client was handed its connection as (`connect::answer`; the
    /// caller proved it on the bus), so a later phase can say who holds the shell's and the agent
    /// tier's connections.
    pub handed: BTreeMap<casement_core::ClientKey, casement_agent::AppKey>,
    /// The remote EIS connections that are open, one device slot each (`connect::answer_remote`),
    /// so `EndEis` can end a portal session's sockets.
    pub remote: crate::remote_sessions::RemoteSessions,
}

/// The D-Bus servers' handles (owned by `casement-ipc`, so their lanes change them there).
#[derive(Debug)]
pub struct IpcState {
    /// The handles.
    pub handles: casement_ipc::IpcHandles,
    /// The Privacy list Settings reads, when `org.quire.Compositor1` is served: the model's
    /// answers are published to it (`privacy::publish`).
    pub board: Option<casement_ipc::privacy::PrivacyBoard>,
}

/// Why casement could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StartError {
    /// The backend refused.
    #[error("backend: {0}")]
    Backend(#[from] BackendError),
    /// A protocol could not start (the keymap).
    #[error("wayland: {0}")]
    Wayland(#[from] WaylandFault),
    /// The settings document could not be read.
    #[error("config: {0}")]
    Config(#[from] ConfigError),
    /// The event loop refused a source.
    #[error("event loop: {0}")]
    Loop(String),
    /// What was asked arrives in a later phase.
    #[error("not yet: {0}")]
    NotYet(&'static str),
    /// The system's entropy could not be read.
    #[error("entropy: {0}")]
    Entropy(String),
}

impl State {
    /// casement as `config` describes it, with the real seams for its state home: the settings
    /// document read, every global created behind the I7 filter, the backend started, the clock
    /// attached to `event_loop`, an empty model at the clock's now. Outputs are plugged
    /// afterwards through the core loop, the same path hotplug takes.
    pub fn new(
        config: StartConfig,
        display: &mut Display<State>,
        event_loop: &LoopHandle<'static, State>,
    ) -> Result<State, StartError> {
        let seams = Seams::real(&config.state_home, &config.system_dir);
        State::with_seams(config, seams, display, event_loop)
    }

    /// The same with these seams (the test kit injects its fakes here).
    pub fn with_seams(
        config: StartConfig,
        mut seams: Seams,
        display: &mut Display<State>,
        event_loop: &LoopHandle<'static, State>,
    ) -> Result<State, StartError> {
        let dh = display.handle();
        // `--safe` ignores the settings file altogether: a start that keeps failing must not
        // depend on what a person or another program wrote there.
        let doc = match config.mode {
            StartMode::Safe => CasementDoc::default(),
            StartMode::Normal => config::load(&config.config_home)?,
        };
        let settings = safe_settings(config.mode, config::to_core(&doc));
        let filter = GlobalFilter::policy();
        let primary = primary_selection::PrimarySelectionPart::new::<State>(&dh, filter);
        let mut wl = WaylandStates {
            clients: ClientMint::start(),
            compositor: compositor::CompositorPart::new::<State>(&dh, filter),
            subcompositor: subcompositor::SubcompositorPart::new(),
            shm: shm::ShmPart::new::<State>(&dh, filter),
            output: output::OutputPart::new::<State>(&dh, filter),
            viewporter: viewporter::ViewporterPart::new::<State>(&dh, filter),
            single_pixel: single_pixel::SinglePixelPart::new::<State>(&dh, filter),
            alpha_modifier: alpha_modifier::AlphaModifierPart::new::<State>(&dh, filter),
            presentation: presentation::PresentationPart::new::<State>(&dh, filter),
            fractional_scale: fractional_scale::FractionalScalePart::new::<State>(&dh, filter),
            fifo: fifo::FifoPart::new::<State>(&dh, filter),
            commit_timing: commit_timing::CommitTimingPart::new::<State>(&dh, filter),
            color_management: color_management::ColorPart::new::<State>(&dh, filter),
            content_type: content_type::ContentTypePart::new::<State>(&dh, filter),
            xdg_shell: xdg_shell::XdgShellPart::new::<State>(&dh, filter),
            xdg_decoration: xdg_decoration::XdgDecorationPart::new::<State>(&dh, filter),
            xdg_activation: xdg_activation::XdgActivationPart::new::<State>(&dh, filter),
            xdg_dialog: xdg_dialog::XdgDialogPart::new::<State>(&dh, filter),
            xdg_toplevel_icon: xdg_toplevel_icon::XdgToplevelIconPart::new::<State>(&dh, filter),
            xdg_toplevel_tag: xdg_toplevel_tag::XdgToplevelTagPart::new::<State>(&dh, filter),
            xdg_foreign: xdg_foreign::XdgForeignPart::new::<State>(&dh, filter),
            system_bell: system_bell::SystemBellPart::new::<State>(&dh, filter),
            quire_window: quire_window::QuireWindowPart::new::<State>(&dh, filter),
            layer_shell: layer_shell::LayerShellPart::new::<State>(&dh, filter),
            session_lock: session_lock::SessionLockPart::new::<State>(&dh, filter),
            seat: seat::SeatPart::new(&dh, filter, &settings.keyboard)?,
            cursor_shape: cursor_shape::CursorShapePart::new::<State>(&dh, filter),
            pointer_constraints: pointer_constraints::PointerConstraintsPart::new::<State>(
                &dh, filter,
            ),
            relative_pointer: relative_pointer::RelativePointerPart::new::<State>(&dh, filter),
            pointer_gestures: pointer_gestures::PointerGesturesPart::new::<State>(&dh, filter),
            shortcuts_inhibit: shortcuts_inhibit::ShortcutsInhibitPart::new::<State>(&dh, filter),
            tablet: tablet::TabletPart::new::<State>(&dh, filter),
            data_device: data_device::DataDevicePart::new::<State>(&dh, filter),
            ext_data_control: ext_data_control::ExtDataControlPart::new::<State>(
                &dh, filter, &primary,
            ),
            wlr_data_control: wlr_data_control::WlrDataControlPart::new::<State>(
                &dh, filter, &primary,
            ),
            primary_selection: primary,
            dmabuf: dmabuf::DmabufPart::new(filter),
            text_input: text_input::TextInputPart::new::<State>(&dh, filter),
            input_method: input_method::InputMethodPart::new::<State>(&dh, filter),
            virtual_keyboard: virtual_keyboard::VirtualKeyboardPart::new::<State>(&dh, filter),
            idle_notify: idle_notify::IdleNotifyPart::new::<State>(&dh, filter),
            idle_inhibit: idle_inhibit::IdleInhibitPart::new::<State>(&dh, filter),
            foreign_toplevel_list: foreign_toplevel_list::ForeignToplevelListPart::new::<State>(
                &dh, filter,
            ),
            security_context: security_context::SecurityContextPart::new::<State>(&dh, filter),
            background_effect: background_effect::BackgroundEffectPart::new::<State>(&dh, filter),
            // P1-M1 advertises the first three; P1-M2 and P1-C1 fill the rest (their `new` creates no global yet).
            output_management: output_management::OutputManagementPart::new::<State>(&dh, filter),
            output_power: output_power::OutputPowerPart::new::<State>(&dh, filter),
            gamma: gamma::GammaPart::new::<State>(&dh, filter),
            wlr_toplevel: wlr_toplevel::WlrToplevelPart::new::<State>(&dh, filter),
            workspace: workspace::WorkspacePart::new::<State>(&dh, filter),
            image_capture: image_capture::ImageCapturePart::new::<State>(&dh, filter),
            // P2-Z declares these three; P2-Q1, P2-Q2 and P2-Q3 create their globals.
            capture_source: capture_source::ToplevelCapturePart::new::<State>(&dh, filter),
            screencopy: casement_wayland::screencopy::ScreencopyPart::new::<State>(&dh, filter),
            quire_desktop: quire::desktop::QuireDesktopPart::new::<State>(&dh, filter),
            quire_shell: quire::shell::QuireShellPart::new::<State>(&dh, filter),
        };
        if config.dmabuf == DmabufMode::Soft {
            dmabuf::apply_offer::<State>(&mut wl.dmabuf, &dh, &dmabuf::DmabufOffer::soft())?;
        }
        let backend = match config.backend {
            BackendChoice::Headless => {
                BackendKind::Headless(HeadlessBackend::start(HeadlessConfig {
                    outputs: config.outputs.clone(),
                    renderer: config.renderer,
                    dmabuf: config.dmabuf,
                    cost: config.cost.clone(),
                })?)
            }
            BackendChoice::Winit => {
                let first = config
                    .outputs
                    .first()
                    .cloned()
                    .ok_or(StartError::NotYet("winit needs an output spec"))?;
                BackendKind::Winit(Box::new(WinitBackend::start(first)?))
            }
            BackendChoice::Tty => {
                let seams = tty_seams(
                    seams.tty.take(),
                    event_loop,
                    &settings.gpu.render,
                    &config.tty,
                    render_mode_of(config.mode),
                )?;
                let mut tty = TtyBackend::start(seams)?;
                offer_dmabuf(&mut wl.dmabuf, &dh, &mut tty)?;
                BackendKind::Tty(Box::new(tty))
            }
        };
        let clock = ClockSource::start(config.clock, event_loop)?;
        let timing = crate::timing::Timing::new(crate::timing::Stamp(clock.monotonic_epoch_ns()));
        let entropy = config
            .entropy
            .bytes()
            .map_err(|error| StartError::Entropy(error.to_string()))?;
        wl.quire_shell.wall = Box::new(seams.wall);
        let model = Model::new(
            settings.clone(),
            clock.now(),
            SpaceSeed::from_entropy(entropy),
        );
        let core = CoreState {
            model,
            layout: RustScene,
            arrangement: Arrangement::default(),
            motion: MotionBook::empty(),
            knobs: LayoutKnobs::of(&settings),
            mint: IdMint::start(),
        };
        let render = RenderState {
            textures: TextureBook::empty(),
            trackers: BTreeMap::new(),
            chrome: ChromeLayer {
                cursor: CursorDraw::Named(CursorShape::Default),
                cursor_at: LogicalPoint { x: 0, y: 0 },
                covers: Vec::new(),
            },
            pointer: PointerStart::Unplaced,
            presence: crate::core_loop::Presence::default(),
            frame_faults: RepeatGate::new(FAULT_LOG_EVERY),
            frame_over: None,
            closing: BTreeMap::new(),
            bodies: crate::core_loop::Bodies::default(),
            divider: crate::core_loop::DividerPointer::default(),
            tile_drag: casement_core::window::TileDrag::default(),
            clicks: casement_core::window::ClickTracker::default(),
            light_arm: casement_core::window::LightArm::default(),
            light_dwell: casement_core::window::Dwell::default(),
            scheme: casement_layout::Scheme::default(),
            transparency: match config.mode {
                StartMode::Safe => casement_layout::Transparency::Reduced,
                StartMode::Normal => casement_layout::Transparency::default(),
            },
            titles: None,
            overview_hover: None,
            overview_drag: casement_core::overview_drag::Drag::Idle,
            labels: None,
            icons: crate::icons::IconBook::new(config.icons.clone()),
        };
        Ok(State {
            core,
            wl,
            backend,
            render,
            agent: AgentState::default(),
            ipc: IpcState {
                handles: casement_ipc::IpcHandles::none(),
                board: None,
            },
            clock,
            timing,
            seams,
            xwayland: XwaylandState::Off,
            start_mode: config.mode,
            processes: crate::clients::Processes::default(),
            roster: crate::clients::Roster::default(),
            probes: crate::clients::Probes::default(),
            display: dh,
            event_loop: event_loop.clone(),
            #[cfg(feature = "testing")]
            ack_policy: crate::relax::AckPolicy::default(),
        })
    }

    /// The settings the model runs with.
    pub fn settings(&self) -> &CasementSettings {
        self.core.model.settings()
    }
}

/// `settings` as a run in `mode` uses them: unchanged, or for `--safe` with the cheapest minimise
/// animation (the settings are the defaults already, so nothing a person chose is in them).
fn safe_settings(mode: StartMode, settings: CasementSettings) -> CasementSettings {
    match mode {
        StartMode::Normal => settings,
        StartMode::Safe => CasementSettings {
            windows: casement_core::settings::WindowSettings {
                minimise_effect: casement_core::settings::MinimiseEffect::Scale,
                ..settings.windows
            },
            ..settings
        },
    }
}

/// The dma-buf offer of a seat: the render plan's feedback (a scanout tranche, the render
/// GPU's tranche, an import tranche for each other GPU that scans out) when the GPUs and their
/// formats are known; else what `--dmabuf` asked for (soft in the jail, nothing by default).
fn offer_dmabuf(
    part: &mut dmabuf::DmabufPart,
    display: &DisplayHandle,
    tty: &mut TtyBackend,
) -> Result<(), StartError> {
    match tty.dmabuf_feedback() {
        Ok(Some(feedback)) => {
            let offer = dmabuf::DmabufOffer::from_feedback(&feedback);
            dmabuf::apply_offer::<State>(part, display, &offer)?;
        }
        Ok(None) => {}
        Err(error) => tracing::warn!(%error, "dma-buf feedback not offered; clients use shm"),
    }
    Ok(())
}

/// How a run in `mode` draws on a seat: `--safe` survives a broken GPU driver, so it draws every
/// output with the CPU and opens no GPU at all (CASB-07); a normal run uses the GPU.
fn render_mode_of(mode: StartMode) -> RenderMode {
    match mode {
        StartMode::Normal => RenderMode::Gpu,
        StartMode::Safe => RenderMode::Software,
    }
}

/// The seams the tty backend starts over: the injected ones (a test), else the machine's own.
fn tty_seams(
    injected: Option<TtySeams>,
    handle: &LoopHandle<'static, State>,
    render: &GpuChoice,
    mode: &TtyMode,
    drawing: RenderMode,
) -> Result<TtySeams, StartError> {
    match (injected, mode) {
        (Some(seams), _) => Ok(seams),
        (None, TtyMode::Direct(direct)) => direct_tty_seams(handle, direct, drawing),
        (None, _) => real_tty_seams(handle, render, drawing),
    }
}

/// The seams of the vkms test mode (`--drm-node`): one node, no seat, no input.
#[cfg(feature = "tty")]
fn direct_tty_seams(
    handle: &LoopHandle<'static, State>,
    direct: &DirectTty,
    drawing: RenderMode,
) -> Result<TtySeams, StartError> {
    let writeback = match direct.writeback_raw {
        Some(_) => casement_backend::tty::WritebackMode::Enabled,
        None => casement_backend::tty::WritebackMode::Disabled,
    };
    Ok(casement_backend::tty::direct_seams(
        &direct.node,
        writeback,
        drawing,
        handle,
    )?)
}

/// A build without the real adapters cannot run on a node either.
#[cfg(not(feature = "tty"))]
fn direct_tty_seams(
    handle: &LoopHandle<'static, State>,
    direct: &DirectTty,
    drawing: RenderMode,
) -> Result<TtySeams, StartError> {
    let _ = (handle, direct, drawing);
    Err(StartError::NotYet(
        "this build has no tty backend (feature `tty`)",
    ))
}

/// The machine's seat, devices and GPUs; `render` is the `gpu.render` setting.
#[cfg(feature = "tty")]
fn real_tty_seams(
    handle: &LoopHandle<'static, State>,
    render: &GpuChoice,
    drawing: RenderMode,
) -> Result<TtySeams, StartError> {
    let root = casement_backend::gpu::SysRoot::host();
    Ok(casement_backend::tty::real_seams(
        root,
        render.clone(),
        drawing,
        handle,
    )?)
}

/// A build without the real adapters cannot run on a seat.
#[cfg(not(feature = "tty"))]
fn real_tty_seams(
    handle: &LoopHandle<'static, State>,
    render: &GpuChoice,
    drawing: RenderMode,
) -> Result<TtySeams, StartError> {
    let _ = (handle, render, drawing);
    Err(StartError::NotYet(
        "this build has no tty backend (feature `tty`)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::settings::MinimiseEffect;

    #[test]
    fn a_safe_run_draws_with_the_cpu_and_a_normal_run_with_the_gpu() {
        assert_eq!(render_mode_of(StartMode::Safe), RenderMode::Software);
        assert_eq!(render_mode_of(StartMode::Normal), RenderMode::Gpu);
    }

    #[test]
    fn a_safe_run_minimises_the_plainest_way_and_a_normal_run_keeps_the_settings() {
        let normal = CasementSettings::defaults();
        assert_eq!(normal.windows.minimise_effect, MinimiseEffect::Genie);
        let kept = safe_settings(StartMode::Normal, normal.clone());
        assert_eq!(kept, normal);
        let safe = safe_settings(StartMode::Safe, normal.clone());
        assert_eq!(safe.windows.minimise_effect, MinimiseEffect::Scale);
        assert_eq!(safe.pointer, normal.pointer, "nothing else is touched");
    }
}
