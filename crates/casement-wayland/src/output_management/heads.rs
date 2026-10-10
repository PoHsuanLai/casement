//! The heads (outputs) and modes a manager announces, and what is sent to keep them current.

use super::wire::{side, transform_to};
use crate::states::WaylandFault;
use casement_core::OutputKey;
use casement_core::OutputState;
use casement_core::output::{
    EdidSerial, Mode, ModePreference, OutputConfig, OutputFacts, OutputPower,
};
use smithay::reexports::wayland_protocols_wlr::output_management::v1::server::{
    zwlr_output_head_v1::{self, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::ZwlrOutputManagerV1,
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};
use smithay::reexports::wayland_server::{Client, DataInit, Dispatch, DisplayHandle, Resource};
use smithay::wayland::Dispatch2;

/// The highest version of `zwlr_output_head_v1`.
const HEAD_VERSION: u32 = 4;

/// The highest version of `zwlr_output_mode_v1`.
const MODE_VERSION: u32 = 3;

/// One head's data: the output it stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadData {
    pub(super) output: OutputKey,
}

/// One mode's data: the output it belongs to and the mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeData {
    pub(super) output: OutputKey,
    pub(super) mode: Mode,
}

/// A head and its modes as one manager's client sees them.
#[derive(Debug)]
pub(super) struct HeadObjects {
    head: ZwlrOutputHeadV1,
    modes: Vec<(Mode, ZwlrOutputModeV1)>,
    pub(super) facts: OutputFacts,
    pub(super) config: OutputConfig,
}

impl HeadObjects {
    /// Sends the head's state after a change of configuration.
    pub(super) fn update(&mut self, config: OutputConfig) {
        self.config = config;
        self.send_state();
    }

    /// Tells the client the head is gone (its modes too).
    pub(super) fn finish(self) {
        for (_, mode) in &self.modes {
            mode.finished();
        }
        self.head.finished();
    }

    /// Whether the head is lit and where, how and at what scale; a dark head is only `enabled
    /// 0`.
    fn send_state(&self) {
        let config = &self.config;
        match config.power {
            OutputPower::Off => self.head.enabled(0),
            OutputPower::On => {
                self.head.enabled(1);
                if let Some((_, mode)) = self.modes.iter().find(|(mode, _)| *mode == config.mode) {
                    self.head.current_mode(mode);
                }
                self.head.position(config.position.x, config.position.y);
                self.head.transform(transform_to(config.transform));
                self.head.scale(config.scale.as_f64());
            }
        }
    }
}

/// What a client reads as the head's description.
fn describe(facts: &OutputFacts) -> String {
    format!(
        "{} {} ({})",
        facts.edid.make,
        facts.edid.model,
        facts.name.as_str()
    )
}

/// Announces `plugged` to `manager`'s client: the head, its modes and its state.
pub(super) fn create_head<D>(
    display: &DisplayHandle,
    client: &Client,
    manager: &ZwlrOutputManagerV1,
    plugged: &OutputState,
) -> Result<HeadObjects, WaylandFault>
where
    D: Dispatch<ZwlrOutputHeadV1, HeadData> + Dispatch<ZwlrOutputModeV1, ModeData> + 'static,
{
    let facts = &plugged.facts;
    let version = manager.version().min(HEAD_VERSION);
    let head = client
        .create_resource::<ZwlrOutputHeadV1, _, D>(
            display,
            version,
            HeadData {
                output: facts.key.clone(),
            },
        )
        .map_err(|_| WaylandFault::SurfaceGone)?;
    manager.head(&head);
    head.name(facts.name.as_str().to_owned());
    head.description(describe(facts));
    head.physical_size(side(facts.physical.w), side(facts.physical.h));
    if version >= 2 {
        head.make(facts.edid.make.clone());
        head.model(facts.edid.model.clone());
        if let EdidSerial::Known(serial) = &facts.edid.serial {
            head.serial_number(serial.clone());
        }
    }
    let mut modes = Vec::with_capacity(facts.modes.len());
    for mode in &facts.modes {
        let object = client
            .create_resource::<ZwlrOutputModeV1, _, D>(
                display,
                version.min(MODE_VERSION),
                ModeData {
                    output: facts.key.clone(),
                    mode: *mode,
                },
            )
            .map_err(|_| WaylandFault::SurfaceGone)?;
        head.mode(&object);
        object.size(side(mode.size.w), side(mode.size.h));
        object.refresh(side(mode.refresh.0));
        if mode.preference == ModePreference::Preferred {
            object.preferred();
        }
        modes.push((*mode, object));
    }
    let objects = HeadObjects {
        head,
        modes,
        facts: facts.clone(),
        config: plugged.config,
    };
    objects.send_state();
    Ok(objects)
}

impl<D> Dispatch2<ZwlrOutputHeadV1, D> for HeadData {
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &ZwlrOutputHeadV1,
        request: zwlr_output_head_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        // `release` is the only request; the destructor does the work.
        let _ = request;
    }
}

impl<D> Dispatch2<ZwlrOutputModeV1, D> for ModeData {
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &ZwlrOutputModeV1,
        request: zwlr_output_mode_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        // `release` is the only request; the destructor does the work.
        let _ = request;
    }
}
