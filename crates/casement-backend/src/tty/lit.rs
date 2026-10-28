//! Lighting an output (P1-T): `Effect::ApplyOutput` on a real seat. The configuration the model
//! chose becomes one atomic commit on the device that drives the connector, and the surface that
//! scans the output out (the D2 to D3 hand-off) is attached to the scanout set.

use super::TtyBackend;
use super::connector::vrr_enabled;
use super::fallback::mode_ladder;
use super::kms::{
    ConnectorFacts, ConnectorId, CrtcId, KmsCommit, KmsError, KmsRefusal, OutputCommit,
};
use super::scanouts::Scanout;
use super::udev::DevicePath;
use crate::backend::BackendError;
use casement_core::OutputName;
use casement_core::output::{Mode, OutputConfig, OutputPower};
use casement_core::time::{Mono, VblankClock};
use std::time::Duration;

/// How long a mode that was lit and then showed nothing (two recoveries in a row) is left out of
/// the modes tried for the output. A person who asks for it again later is obeyed.
pub(super) const DISTRUST_FOR: Duration = Duration::from_secs(60);

/// Where an output is lit, and with what.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LitOutput {
    /// The device that drives it.
    pub device: DevicePath,
    /// Its connector.
    pub connector: ConnectorId,
    /// The CRTC that scans it out.
    pub crtc: CrtcId,
    /// The configuration it was last lit (or darkened) with.
    pub config: OutputConfig,
}

impl LitOutput {
    /// Whether it is lit.
    pub fn power(&self) -> OutputPower {
        self.config.power
    }
}

/// What applying a configuration did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Lighting {
    /// The output is dark (DPMS off): no frame is wanted.
    Dark,
    /// The output is lit.
    Lit,
    /// The output was dark and is lit again (DPMS On): the caller queues a frame now, which is
    /// what lights a real CRTC.
    Relit,
    /// The display refused the mode asked for and is lit at this one instead (the mode it was
    /// showing, its preferred mode or its smallest): the caller tells the model the real
    /// configuration. A frame is queued now, as for [`Lighting::Relit`].
    Fallback(Mode),
    /// The seat is lent away: nothing was touched. The return re-applies every output.
    Deferred,
}

impl TtyBackend {
    /// Puts `config` on the output `name`: DPMS, or a commit (with a modeset where the mode
    /// changed) and a scanout surface for a newly lit output or a new mode. `now` starts the
    /// vblank clock of a surface made now.
    ///
    /// ```no_run
    /// use casement_backend::tty::{Lighting, TtyBackend};
    /// use casement_core::OutputName;
    /// use casement_core::output::OutputConfig;
    /// use casement_core::time::Mono;
    ///
    /// fn light(tty: &mut TtyBackend, name: &OutputName, config: &OutputConfig, now: Mono) {
    ///     if let Ok(Lighting::Lit) = tty.apply(name, config, now) {
    ///         // queue a frame
    ///     }
    /// }
    /// ```
    pub fn apply(
        &mut self,
        name: &OutputName,
        config: &OutputConfig,
        now: Mono,
    ) -> Result<Lighting, BackendError> {
        if self.seat != super::SeatState::Held {
            return Ok(Lighting::Deferred);
        }
        let (path, connector) = self.driver_of(name)?;
        match config.power {
            OutputPower::Off => self.darken(name, &path, &connector),
            OutputPower::On => self.light(name, &path, &connector, config, now),
        }
    }

    /// The device that drives `name` and the connector's facts.
    fn driver_of(&self, name: &OutputName) -> Result<(DevicePath, ConnectorFacts), KmsError> {
        self.devices
            .devices
            .iter()
            .find_map(|(path, device)| {
                device
                    .kms
                    .resources()
                    .connectors
                    .into_iter()
                    .find(|connector| connector.name == *name)
                    .map(|connector| (path.clone(), connector))
            })
            .ok_or_else(|| KmsError::NoDriver(name.as_str().to_owned()))
    }

    /// DPMS off: the connector goes dark and no frame is queued until it is lit again.
    fn darken(
        &mut self,
        name: &OutputName,
        path: &DevicePath,
        connector: &ConnectorFacts,
    ) -> Result<Lighting, BackendError> {
        if let Some(device) = self.devices.devices.get_mut(path) {
            device.kms.set_power(connector.id, OutputPower::Off)?;
        }
        if let Some(scanout) = self.scanouts.get_mut(name) {
            scanout.blank()?;
        }
        if let Some(lit) = self.lit.get_mut(name) {
            lit.config.power = OutputPower::Off;
        }
        Ok(Lighting::Dark)
    }

    /// Lights `name` as `config` says, or as near to it as the display takes: a mode it refuses
    /// (the cable cannot carry it, the link is short of bandwidth) is followed by the mode it was
    /// showing, then its preferred mode, then its smallest, and the answer says which one it is
    /// lit at ([`Lighting::Fallback`]). Only when none works is it an error.
    fn light(
        &mut self,
        name: &OutputName,
        path: &DevicePath,
        connector: &ConnectorFacts,
        config: &OutputConfig,
        now: Mono,
    ) -> Result<Lighting, BackendError> {
        let previous = self.lit.get(name).map(|lit| lit.config.mode);
        let avoid = self
            .distrust
            .get(name)
            .filter(|(_, at)| now.since(*at) < DISTRUST_FOR)
            .map(|(mode, _)| *mode);
        let mut refusal = None;
        for mode in mode_ladder(config.mode, previous, &connector.modes, avoid) {
            let attempt = OutputConfig { mode, ..*config };
            match self.light_at(name, path, connector, &attempt, now) {
                Ok(lighting) if mode == config.mode => return Ok(lighting),
                Ok(_) => {
                    tracing::warn!(
                        output = name.as_str(),
                        "the display refused the mode asked for; it is lit at another"
                    );
                    return Ok(Lighting::Fallback(mode));
                }
                Err(error) => {
                    tracing::warn!(output = name.as_str(), %error, "the display refused a mode");
                    refusal = Some(error);
                }
            }
        }
        Err(refusal.unwrap_or_else(|| KmsError::Refused(KmsRefusal::Mode).into()))
    }

    /// Commits `config` and attaches a surface where the output has none for this mode.
    fn light_at(
        &mut self,
        name: &OutputName,
        path: &DevicePath,
        connector: &ConnectorFacts,
        config: &OutputConfig,
        now: Mono,
    ) -> Result<Lighting, BackendError> {
        let crtc = self.crtc_for(name, path, connector)?;
        let facts = self
            .devices
            .devices
            .get(path)
            .map(|device| device.kms.resources())
            .and_then(|resources| resources.crtcs.into_iter().find(|facts| facts.id == crtc))
            .ok_or(KmsError::Refused(KmsRefusal::NoCrtc))?;
        let vrr = vrr_enabled(self.vrr, connector, &facts);
        let commit = KmsCommit::new(
            vec![OutputCommit::new(connector.id, crtc, config.mode, vrr)],
            true,
        );
        let was = self.lit.get(name).cloned();
        // A surface is made for a mode, a scale and a transform; a new one for any change. The
        // old one goes first: a device keeps one surface per connector.
        let picture = |config: &OutputConfig| (config.mode, config.scale, config.transform);
        let remade = was.as_ref().map(|lit| picture(&lit.config)) != Some(picture(config));
        if remade {
            let _ = self.scanouts.detach(name);
        }
        let device = self
            .devices
            .devices
            .get_mut(path)
            .ok_or_else(|| KmsError::DeviceGone(path.0.clone()))?;
        device.kms.commit(commit)?;
        let was_dark = was.as_ref().map(LitOutput::power) == Some(OutputPower::Off);
        if was_dark {
            device.kms.set_power(connector.id, OutputPower::On)?;
        }
        if remade || self.scanouts.get(name).is_none() {
            let surface = device.kms.scanout(connector.id, config)?;
            let clock = VblankClock::new(config.mode.refresh, now);
            let scanout = Scanout::new(surface, clock);
            let scanout = match self.carried.remove(name) {
                Some(watch) => scanout.carrying(watch),
                None => scanout,
            };
            self.scanouts.attach(name.clone(), scanout);
        }
        self.lit.insert(
            name.clone(),
            LitOutput {
                device: path.clone(),
                connector: connector.id,
                crtc,
                config: *config,
            },
        );
        Ok(match was_dark {
            true => Lighting::Relit,
            false => Lighting::Lit,
        })
    }

    /// The CRTC that scans `name` out: the one it is lit on, else the first the connector can
    /// use that no other output of the device holds.
    fn crtc_for(
        &self,
        name: &OutputName,
        path: &DevicePath,
        connector: &ConnectorFacts,
    ) -> Result<CrtcId, KmsError> {
        if let Some(lit) = self.lit.get(name) {
            return Ok(lit.crtc);
        }
        let held = |crtc: &CrtcId| {
            self.lit
                .values()
                .any(|lit| lit.device == *path && lit.crtc == *crtc)
        };
        connector
            .possible_crtcs
            .iter()
            .copied()
            .find(|crtc| !held(crtc))
            .ok_or(KmsError::Refused(KmsRefusal::NoCrtc))
    }

    /// How many gamma steps the CRTC of `name` has per channel: the one it is lit on, else the
    /// first the connector can use. None for an output no device drives.
    pub fn gamma_size(&self, name: &OutputName) -> Option<u32> {
        let (path, connector) = self.driver_of(name).ok()?;
        let crtc = match self.lit.get(name) {
            Some(lit) => lit.crtc,
            None => *connector.possible_crtcs.first()?,
        };
        let resources = self.devices.devices.get(&path)?.kms.resources();
        resources
            .crtcs
            .iter()
            .find(|facts| facts.id == crtc)
            .map(|facts| facts.gamma_size)
    }

    /// Forgets every output no device reports plugged any more: its scanout, its lighting and
    /// its capture shadow go with it.
    pub(super) fn prune(&mut self) {
        let plugged: Vec<OutputName> = self
            .devices
            .devices
            .values()
            .flat_map(|device| device.known.0.values().map(|facts| facts.name.clone()))
            .collect();
        let gone: Vec<OutputName> = self
            .lit
            .keys()
            .filter(|name| !plugged.contains(name))
            .cloned()
            .collect();
        for name in gone {
            self.lit.remove(&name);
            let _ = self.scanouts.detach(&name);
            self.readback.forget(&name);
            self.capture_stopped(&name);
            self.carried.remove(&name);
            self.distrust.remove(&name);
        }
    }
}
