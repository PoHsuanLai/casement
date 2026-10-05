//! The composition root's state (FROZEN: lanes never edit it; a change is an interface ask).

use crate::cli::StartConfig;
use crate::config::{self, ConfigError};
use crate::time::ClockSource;
use casement_backend::{
    BackendChoice, BackendError, BackendKind, DmabufMode, HeadlessBackend, HeadlessConfig,
    WinitBackend,
};
use casement_core::geometry::LogicalPoint;
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use casement_core::{IdMint, Model, OutputKey};
use casement_layout::{Arrangement, LayoutKnobs, RustScene};
use casement_motion::MotionBook;
use casement_render::{ChromeLayer, CursorDraw, CursorShape, TextureBook};
use casement_wayland::{
    ClientMint, GlobalFilter, WaylandFault, WaylandStates, alpha_modifier, background_effect,
    commit_timing, compositor, content_type, cursor_shape, data_device, dmabuf, ext_data_control,
    fifo, foreign_toplevel_list, fractional_scale, idle_inhibit, idle_notify, input_method,
    layer_shell, output, pointer_constraints, pointer_gestures, presentation, primary_selection,
    quire_window, relative_pointer, seat, security_context, session_lock, shm, shortcuts_inhibit,
    single_pixel, subcompositor, system_bell, tablet, text_input, viewporter, virtual_keyboard,
    wlr_data_control, xdg_activation, xdg_decoration, xdg_dialog, xdg_foreign, xdg_shell,
    xdg_toplevel_icon, xdg_toplevel_tag,
};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::wayland_server::{Display, DisplayHandle};
use std::collections::BTreeMap;

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
    /// The display, for creating globals, inserting clients and sending events. (Not in the
    /// plan's sketch; the handlers need it: FINDINGS.)
    pub display: DisplayHandle,
    /// The event loop, for inserting sources (security-context listeners, timers).
    pub event_loop: LoopHandle<'static, State>,
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

/// The agent edge's state (P2-Z and P4-Z give it fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentState;

/// The D-Bus servers' handles (owned by `casement-ipc`, so their lanes change them there).
#[derive(Debug)]
pub struct IpcState {
    /// The handles.
    pub handles: casement_ipc::IpcHandles,
}

/// Why casement could not start.
#[derive(Debug, thiserror::Error)]
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
    /// casement as `config` describes it: the settings document read, every P0 global created
    /// behind the I7 filter, the backend started, the clock attached to `event_loop`, an empty
    /// model at the clock's now. Outputs are plugged afterwards through the core loop, the
    /// same path hotplug takes.
    pub fn new(
        config: StartConfig,
        display: &mut Display<State>,
        event_loop: &LoopHandle<'static, State>,
    ) -> Result<State, StartError> {
        let dh = display.handle();
        let settings = config::to_core(&config::load(&config.config_home)?);
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
                BackendKind::Winit(WinitBackend::start(first)?)
            }
            BackendChoice::Tty => {
                return Err(StartError::NotYet("the tty backend arrives at P1-Z"));
            }
        };
        let clock = ClockSource::start(config.clock, event_loop)?;
        let entropy = config
            .entropy
            .bytes()
            .map_err(|error| StartError::Entropy(error.to_string()))?;
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
        };
        Ok(State {
            core,
            wl,
            backend,
            render,
            agent: AgentState,
            ipc: IpcState {
                handles: casement_ipc::IpcHandles::none(),
            },
            clock,
            display: dh,
            event_loop: event_loop.clone(),
        })
    }

    /// The settings the model runs with.
    pub fn settings(&self) -> &CasementSettings {
        self.core.model.settings()
    }
}
