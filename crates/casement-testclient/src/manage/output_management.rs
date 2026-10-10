//! `zwlr_output_manager_v1` as a client (P1-M1): the heads and modes the compositor announced,
//! and wlr-randr-style configurations (enable or disable heads, set a mode, position, transform
//! or scale, then apply or test) with the answer each got.
//!
//! Self-contained like `foreign_toplevel_list`: it binds the manager on a side queue and reads
//! its events there.

use crate::misc::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, event_created_child};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::{self, ZwlrOutputConfigurationHeadV1},
    zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
    zwlr_output_head_v1::{self, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};

/// One mode of a head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeInfo {
    /// Width in hardware units.
    pub width: i32,
    /// Height in hardware units.
    pub height: i32,
    /// Refresh rate in millihertz.
    pub refresh: i32,
    /// Whether the display prefers it.
    pub preferred: Preference,
}

/// Whether a mode is the display's preferred one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preference {
    /// The compositor sent `preferred`.
    Preferred,
    /// It did not.
    Other,
}

/// Whether a head is lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lit {
    /// `enabled` 1.
    On,
    /// `enabled` 0 (or never said).
    Off,
}

/// One head as the compositor announced it.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadInfo {
    /// The connector name.
    pub name: String,
    /// The description.
    pub description: String,
    /// The EDID make (version 2 and later).
    pub make: Option<String>,
    /// The EDID model.
    pub model: Option<String>,
    /// Whether the head is lit.
    pub lit: Lit,
    /// Every mode it offers.
    pub modes: Vec<ModeInfo>,
    /// The mode in use.
    pub current: Option<ModeInfo>,
    /// Its place in the global space.
    pub position: (i32, i32),
    /// Its transform.
    pub transform: Transform,
    /// Its scale.
    pub scale: f64,
}

/// The mode a head is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeChoice {
    /// One of the head's announced modes (`set_mode`), by size and refresh.
    Offered {
        /// Width.
        width: i32,
        /// Height.
        height: i32,
        /// Refresh in millihertz.
        refresh: i32,
    },
    /// A size and rate that may be none of its modes (`set_custom_mode`).
    Custom {
        /// Width.
        width: i32,
        /// Height.
        height: i32,
        /// Refresh in millihertz, or 0.
        refresh: i32,
    },
}

/// What a lit head is asked for; a part left `None` is not sent.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Properties {
    /// The mode.
    pub mode: Option<ModeChoice>,
    /// The position.
    pub position: Option<(i32, i32)>,
    /// The transform.
    pub transform: Option<Transform>,
    /// The scale.
    pub scale: Option<f64>,
}

/// What a configuration does to one head.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Setting {
    /// `disable_head`.
    Disable,
    /// `enable_head`, with these properties.
    Enable(Properties),
}

/// One head's line of a configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct HeadPlan {
    /// The head's connector name.
    pub name: String,
    /// What to do with it.
    pub setting: Setting,
}

/// `apply` or `test`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    /// `apply`.
    Apply,
    /// `test`.
    Test,
}

/// How the compositor answered a configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// `succeeded`.
    Succeeded,
    /// `failed`.
    Failed,
    /// `cancelled`.
    Cancelled,
}

/// Which configuration an answer is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConfigId(pub u32);

/// A mode object and what it said.
#[derive(Debug)]
struct ModeEntry {
    proxy: ZwlrOutputModeV1,
    info: ModeInfo,
    gone: bool,
}

/// A head object and what it said.
#[derive(Debug)]
struct HeadEntry {
    proxy: ZwlrOutputHeadV1,
    info: HeadInfo,
    modes: Vec<ZwlrOutputModeV1>,
    current: Option<ZwlrOutputModeV1>,
    gone: bool,
}

/// What the side queue dispatches into.
#[derive(Debug, Default)]
struct Roster {
    heads: Vec<HeadEntry>,
    modes: Vec<ModeEntry>,
    serial: u32,
    dones: u32,
    finished: bool,
    answers: Vec<(ConfigId, Answer)>,
}

/// A bound `zwlr_output_manager_v1`.
#[derive(Debug)]
pub struct OutputManager {
    connection: Connection,
    queue: EventQueue<Roster>,
    roster: Roster,
    manager: ZwlrOutputManagerV1,
    configurations: Vec<ZwlrOutputConfigurationV1>,
}

impl OutputManager {
    /// Binds the manager; the compositor announces its heads at the fixture's next pump.
    pub fn start(globals: &GlobalList) -> Result<OutputManager, ClientError> {
        let (connection, queue) = side_queue::<Roster>(globals)?;
        let qh = queue.handle();
        let manager = globals
            .bind::<ZwlrOutputManagerV1, _, _>(&qh, 1..=4, ())
            .map_err(|_| ClientError::MissingGlobal("zwlr_output_manager_v1".to_owned()))?;
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(OutputManager {
            connection,
            queue,
            roster: Roster::default(),
            manager,
            configurations: Vec::new(),
        })
    }

    /// Reads what arrived.
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut self.roster)
    }

    /// The heads announced and not finished, in the order they were announced.
    pub fn heads(&self) -> Vec<HeadInfo> {
        self.roster
            .heads
            .iter()
            .filter(|entry| !entry.gone)
            .map(|entry| self.roster.info_of(entry))
            .collect()
    }

    /// How many heads the compositor has finished.
    pub fn finished_heads(&self) -> usize {
        self.roster.heads.iter().filter(|entry| entry.gone).count()
    }

    /// The serial of the last `done`.
    pub fn serial(&self) -> u32 {
        self.roster.serial
    }

    /// How many `done` events arrived.
    pub fn dones(&self) -> u32 {
        self.roster.dones
    }

    /// Whether the compositor sent `finished` on the manager.
    pub fn finished(&self) -> bool {
        self.roster.finished
    }

    /// Asks the compositor to stop (`stop`); it answers `finished`.
    pub fn stop(&mut self) -> Result<(), ClientError> {
        self.manager.stop();
        self.flush()
    }

    /// Builds a configuration of `plans` at `serial` (the last `done`'s when none is given) and
    /// sends `apply` or `test`. The answer arrives at a later pump ([`OutputManager::answer`]).
    pub fn configure(
        &mut self,
        serial: Option<u32>,
        plans: &[HeadPlan],
        intent: Intent,
    ) -> Result<ConfigId, ClientError> {
        let serial = serial.unwrap_or(self.roster.serial);
        let id = ConfigId(u32::try_from(self.configurations.len()).unwrap_or(u32::MAX));
        let qh = self.queue.handle();
        let configuration = self.manager.create_configuration(serial, &qh, id);
        for plan in plans {
            let entry = self
                .roster
                .heads
                .iter()
                .find(|entry| !entry.gone && entry.info.name == plan.name)
                .ok_or_else(|| ClientError::Dispatch(format!("no head {}", plan.name)))?;
            match plan.setting {
                Setting::Disable => configuration.disable_head(&entry.proxy),
                Setting::Enable(properties) => {
                    let head = configuration.enable_head(&entry.proxy, &qh, ());
                    self.roster.send_properties(entry, &head, properties)?;
                }
            }
        }
        match intent {
            Intent::Apply => configuration.apply(),
            Intent::Test => configuration.test(),
        }
        self.configurations.push(configuration);
        self.flush()?;
        Ok(id)
    }

    /// Builds a configuration that enables `name` twice, which the protocol forbids.
    pub fn enable_twice(&mut self, name: &str) -> Result<(), ClientError> {
        let qh = self.queue.handle();
        let configuration =
            self.manager
                .create_configuration(self.roster.serial, &qh, ConfigId(u32::MAX));
        let entry = self
            .roster
            .heads
            .iter()
            .find(|entry| !entry.gone && entry.info.name == name)
            .ok_or_else(|| ClientError::Dispatch(format!("no head {name}")))?;
        configuration.enable_head(&entry.proxy, &qh, ());
        configuration.enable_head(&entry.proxy, &qh, ());
        self.configurations.push(configuration);
        self.flush()
    }

    /// How the compositor answered configuration `id`, once it has.
    pub fn answer(&self, id: ConfigId) -> Option<Answer> {
        self.roster
            .answers
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, answer)| *answer)
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Roster {
    /// A head with its modes filled in.
    fn info_of(&self, entry: &HeadEntry) -> HeadInfo {
        let known = |proxy: &ZwlrOutputModeV1| {
            self.modes
                .iter()
                .find(|mode| mode.proxy == *proxy && !mode.gone)
                .map(|mode| mode.info)
        };
        HeadInfo {
            modes: entry.modes.iter().filter_map(known).collect(),
            current: entry.current.as_ref().and_then(known),
            ..entry.info.clone()
        }
    }

    /// Sends the properties of a lit head.
    fn send_properties(
        &self,
        entry: &HeadEntry,
        head: &ZwlrOutputConfigurationHeadV1,
        properties: Properties,
    ) -> Result<(), ClientError> {
        match properties.mode {
            Some(ModeChoice::Offered {
                width,
                height,
                refresh,
            }) => {
                let wanted = ModeInfo {
                    width,
                    height,
                    refresh,
                    preferred: Preference::Other,
                };
                let proxy = entry
                    .modes
                    .iter()
                    .find(|proxy| {
                        self.modes.iter().any(|mode| {
                            mode.proxy == **proxy
                                && (mode.info.width, mode.info.height, mode.info.refresh)
                                    == (wanted.width, wanted.height, wanted.refresh)
                        })
                    })
                    .ok_or_else(|| {
                        ClientError::Dispatch("the head offers no such mode".to_owned())
                    })?;
                head.set_mode(proxy);
            }
            Some(ModeChoice::Custom {
                width,
                height,
                refresh,
            }) => head.set_custom_mode(width, height, refresh),
            None => {}
        }
        if let Some((x, y)) = properties.position {
            head.set_position(x, y);
        }
        if let Some(transform) = properties.transform {
            head.set_transform(transform);
        }
        if let Some(scale) = properties.scale {
            head.set_scale(scale);
        }
        Ok(())
    }

    fn head_mut(&mut self, proxy: &ZwlrOutputHeadV1) -> Option<&mut HeadEntry> {
        self.heads.iter_mut().find(|entry| entry.proxy == *proxy)
    }

    fn mode_mut(&mut self, proxy: &ZwlrOutputModeV1) -> Option<&mut ModeEntry> {
        self.modes.iter_mut().find(|entry| entry.proxy == *proxy)
    }
}

impl Dispatch<ZwlrOutputManagerV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        _proxy: &ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => state.heads.push(HeadEntry {
                proxy: head,
                info: HeadInfo {
                    name: String::new(),
                    description: String::new(),
                    make: None,
                    model: None,
                    lit: Lit::Off,
                    modes: Vec::new(),
                    current: None,
                    position: (0, 0),
                    transform: Transform::Normal,
                    scale: 1.0,
                },
                modes: Vec::new(),
                current: None,
                gone: false,
            }),
            zwlr_output_manager_v1::Event::Done { serial } => {
                state.serial = serial;
                state.dones += 1;
            }
            zwlr_output_manager_v1::Event::Finished => state.finished = true,
            _ => {}
        }
    }

    event_created_child!(Roster, ZwlrOutputManagerV1, [
        zwlr_output_manager_v1::EVT_HEAD_OPCODE => (ZwlrOutputHeadV1, ()),
    ]);
}

impl Dispatch<ZwlrOutputHeadV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        if let zwlr_output_head_v1::Event::Mode { mode } = event {
            let blank = ModeInfo {
                width: 0,
                height: 0,
                refresh: 0,
                preferred: Preference::Other,
            };
            state.modes.push(ModeEntry {
                proxy: mode.clone(),
                info: blank,
                gone: false,
            });
            if let Some(entry) = state.head_mut(proxy) {
                entry.modes.push(mode);
            }
            return;
        }
        let Some(entry) = state.head_mut(proxy) else {
            return;
        };
        match event {
            zwlr_output_head_v1::Event::Name { name } => entry.info.name = name,
            zwlr_output_head_v1::Event::Description { description } => {
                entry.info.description = description;
            }
            zwlr_output_head_v1::Event::Make { make } => entry.info.make = Some(make),
            zwlr_output_head_v1::Event::Model { model } => entry.info.model = Some(model),
            zwlr_output_head_v1::Event::Enabled { enabled } => {
                entry.info.lit = if enabled == 0 { Lit::Off } else { Lit::On };
            }
            zwlr_output_head_v1::Event::CurrentMode { mode } => entry.current = Some(mode),
            zwlr_output_head_v1::Event::Position { x, y } => entry.info.position = (x, y),
            zwlr_output_head_v1::Event::Transform { transform } => {
                if let Ok(transform) = transform.into_result() {
                    entry.info.transform = transform;
                }
            }
            zwlr_output_head_v1::Event::Scale { scale } => entry.info.scale = scale,
            zwlr_output_head_v1::Event::Finished => entry.gone = true,
            _ => {}
        }
    }

    event_created_child!(Roster, ZwlrOutputHeadV1, [
        zwlr_output_head_v1::EVT_MODE_OPCODE => (ZwlrOutputModeV1, ()),
    ]);
}

impl Dispatch<ZwlrOutputModeV1, ()> for Roster {
    fn event(
        state: &mut Roster,
        proxy: &ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        let Some(entry) = state.mode_mut(proxy) else {
            return;
        };
        match event {
            zwlr_output_mode_v1::Event::Size { width, height } => {
                entry.info.width = width;
                entry.info.height = height;
            }
            zwlr_output_mode_v1::Event::Refresh { refresh } => entry.info.refresh = refresh,
            zwlr_output_mode_v1::Event::Preferred => entry.info.preferred = Preference::Preferred,
            zwlr_output_mode_v1::Event::Finished => entry.gone = true,
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, ConfigId> for Roster {
    fn event(
        state: &mut Roster,
        _proxy: &ZwlrOutputConfigurationV1,
        event: zwlr_output_configuration_v1::Event,
        data: &ConfigId,
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
        let answer = match event {
            zwlr_output_configuration_v1::Event::Succeeded => Answer::Succeeded,
            zwlr_output_configuration_v1::Event::Failed => Answer::Failed,
            zwlr_output_configuration_v1::Event::Cancelled => Answer::Cancelled,
            _ => return,
        };
        state.answers.push((*data, answer));
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for Roster {
    fn event(
        _state: &mut Roster,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        _event: zwlr_output_configuration_head_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Roster>,
    ) {
    }
}
