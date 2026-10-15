//! `FakeKms`: `Kms` over a shared world a test scripts.

use super::super::scanout::{FakeScanout, GlassHandle};
use super::capture_gpu::FakeCaptureGpu;
use super::topology::SyntheticTopology;
use casement_backend::headless::ConnectorEvent;
use casement_backend::tty::{
    CaptureGpu, CommitVerdict, ConnectorFacts, ConnectorId, ConnectorStatus, DevicePath, Kms,
    KmsCommit, KmsError, KmsOpener, KmsRefusal, KmsResources, Lit, ScanoutSurface, judge,
    output_facts,
};
use casement_core::output::{GammaRamp, Mode, OutputConfig, OutputPower};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// How a monitor's link behaves after a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LinkBehaviour {
    /// Nothing changes.
    #[default]
    Stable,
    /// The link retrains after every commit: the monitor drops its last listed mode that is not
    /// in use, and offers it again after the next commit, reporting `Changed` each time.
    Retrains,
}

/// What the fake device is and has been told.
#[derive(Debug, Default)]
pub struct KmsWorld {
    /// What the device offers now.
    pub resources: KmsResources,
    /// The commits applied, in order.
    pub commits: Vec<KmsCommit>,
    /// What each lit connector is set to now.
    pub lit: Lit,
    /// The power each connector was last given.
    pub power: BTreeMap<ConnectorId, OutputPower>,
    /// The gamma each connector was last given.
    pub gamma: BTreeMap<ConnectorId, GammaRamp>,
    /// The connector events not yet taken by `events`.
    pub pending: Vec<ConnectorEvent>,
    /// How links behave after a commit.
    pub link: LinkBehaviour,
    /// The modes a retraining link has dropped, by connector.
    pub dropped: BTreeMap<ConnectorId, Mode>,
    /// The scanout surface last handed out for each connector (P1-T).
    pub glass: BTreeMap<ConnectorId, GlassHandle>,
    /// How many times the seat was lent away (`pause`).
    pub pauses: u32,
    /// How many times it came back (`resume`).
    pub resumes: u32,
    /// The renderer it lends to draw captures (none: it lends nothing).
    pub capture_gpu: Option<FakeCaptureGpu>,
}

/// A fake display device.
#[derive(Debug, Clone, Default)]
pub struct FakeKms {
    /// The world, shared with every clone.
    pub world: Arc<Mutex<KmsWorld>>,
}

impl FakeKms {
    /// A device offering `resources`.
    pub fn with(resources: KmsResources) -> FakeKms {
        FakeKms {
            world: Arc::new(Mutex::new(KmsWorld {
                resources,
                ..KmsWorld::default()
            })),
        }
    }

    /// A device offering `topology`, with the link behaviour it calls for.
    pub fn synthetic(topology: SyntheticTopology) -> FakeKms {
        let kms = FakeKms::with(topology.resources());
        kms.world().link = topology.link();
        kms
    }

    /// The world, locked.
    pub fn world(&self) -> MutexGuard<'_, KmsWorld> {
        self.world.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A monitor is plugged: the connector appears (replacing one of the same id) and its event
    /// is queued: `Plugged`, or `Changed` when the connector was already attached.
    pub fn plug(&self, connector: ConnectorFacts) {
        let mut world = self.world();
        let facts = output_facts(&connector);
        let was_attached = world
            .resources
            .connectors
            .iter()
            .any(|c| c.id == connector.id && c.status == ConnectorStatus::Connected);
        world.resources.connectors.retain(|c| c.id != connector.id);
        world.resources.connectors.push(connector);
        world.resources.connectors.sort_by_key(|c| c.id);
        world.pending.push(match was_attached {
            true => ConnectorEvent::Changed(facts),
            false => ConnectorEvent::Plugged(facts),
        });
    }

    /// The monitor on `connector` is unplugged: the connector stays, disconnected, as on a real
    /// device, whatever it was lit with is dark, and the `Unplugged` event is queued.
    pub fn unplug(&self, connector: ConnectorId) {
        let mut guard = self.world();
        let world = &mut *guard;
        world.lit.remove(&connector);
        world.dropped.remove(&connector);
        let attached = world
            .resources
            .connectors
            .iter_mut()
            .find(|c| c.id == connector && c.status == ConnectorStatus::Connected);
        if let Some(attached) = attached {
            let key = output_facts(attached).key;
            attached.status = ConnectorStatus::Disconnected;
            world.pending.push(ConnectorEvent::Unplugged(key));
        }
    }

    /// The commits applied so far.
    pub fn commits(&self) -> Vec<KmsCommit> {
        self.world().commits.clone()
    }

    /// The scanout surface last handed out for `connector`.
    pub fn glass(&self, connector: ConnectorId) -> Option<GlassHandle> {
        self.world().glass.get(&connector).cloned()
    }

    /// How many times the seat was lent away and how many times it came back.
    pub fn seat_changes(&self) -> (u32, u32) {
        let world = self.world();
        (world.pauses, world.resumes)
    }

    /// The power `connector` was last given.
    pub fn power_of(&self, connector: ConnectorId) -> Option<OutputPower> {
        self.world().power.get(&connector).copied()
    }

    fn known(world: &KmsWorld, connector: ConnectorId) -> Result<(), KmsError> {
        match world.resources.connectors.iter().any(|c| c.id == connector) {
            true => Ok(()),
            false => Err(KmsError::UnknownConnector(connector)),
        }
    }
}

impl KmsWorld {
    /// A retraining link flaps after a commit: each committed connector drops or regains a mode.
    fn retrain(&mut self, committed: &[ConnectorId]) {
        for id in committed {
            let in_use = self.lit.get(id).map(|o| o.mode);
            if let Some(mode) = self.dropped.remove(id) {
                self.connector_mut(*id, |c| c.modes.push(mode));
            } else {
                let spare = self
                    .resources
                    .connectors
                    .iter()
                    .find(|c| c.id == *id)
                    .and_then(|c| c.modes.iter().rev().find(|m| Some(**m) != in_use).copied());
                if let Some(mode) = spare {
                    self.connector_mut(*id, |c| c.modes.retain(|m| *m != mode));
                    self.dropped.insert(*id, mode);
                }
            }
            let changed = self.resources.connectors.iter().find(|c| c.id == *id);
            if let Some(connector) = changed {
                self.pending
                    .push(ConnectorEvent::Changed(output_facts(connector)));
            }
        }
    }

    fn connector_mut(&mut self, id: ConnectorId, edit: impl FnOnce(&mut ConnectorFacts)) {
        if let Some(connector) = self.resources.connectors.iter_mut().find(|c| c.id == id) {
            edit(connector);
        }
    }
}

impl Kms for FakeKms {
    fn resources(&self) -> KmsResources {
        self.world().resources.clone()
    }

    fn test(&mut self, commit: &KmsCommit) -> CommitVerdict {
        let world = self.world();
        judge(&world.resources, &world.lit, commit)
    }

    fn commit(&mut self, commit: KmsCommit) -> Result<(), KmsError> {
        let mut world = self.world();
        match judge(&world.resources, &world.lit, &commit) {
            CommitVerdict::Accepted => {}
            CommitVerdict::NeedsModeset => {
                return Err(KmsError::Refused(KmsRefusal::ModesetForbidden));
            }
            CommitVerdict::Refused(refusal) => return Err(KmsError::Refused(refusal)),
            _ => return Err(KmsError::Refused(KmsRefusal::Unsupported)),
        }
        let committed: Vec<ConnectorId> = commit.outputs.iter().map(|o| o.connector).collect();
        for output in &commit.outputs {
            world.lit.insert(output.connector, output.clone());
        }
        world.commits.push(commit);
        if world.link == LinkBehaviour::Retrains {
            world.retrain(&committed);
        }
        Ok(())
    }

    fn set_power(&mut self, connector: ConnectorId, power: OutputPower) -> Result<(), KmsError> {
        let mut world = self.world();
        FakeKms::known(&world, connector)?;
        world.power.insert(connector, power);
        Ok(())
    }

    fn set_gamma(&mut self, connector: ConnectorId, ramp: &GammaRamp) -> Result<(), KmsError> {
        let mut world = self.world();
        FakeKms::known(&world, connector)?;
        world.gamma.insert(connector, ramp.clone());
        Ok(())
    }

    fn events(&mut self) -> Vec<ConnectorEvent> {
        std::mem::take(&mut self.world().pending)
    }

    fn scanout(
        &mut self,
        connector: ConnectorId,
        config: &OutputConfig,
    ) -> Result<Box<dyn ScanoutSurface>, KmsError> {
        let _ = config;
        let mut world = self.world();
        if !world.lit.contains_key(&connector) {
            return Err(KmsError::UnknownConnector(connector));
        }
        let glass = GlassHandle::new(FakeScanout::following());
        world.glass.insert(connector, glass.clone());
        Ok(Box::new(glass))
    }

    fn capture_gpu(&self) -> Option<Box<dyn CaptureGpu>> {
        let gpu = self.world().capture_gpu.clone()?;
        Some(Box::new(gpu))
    }

    fn pause(&mut self) {
        self.world().pauses += 1;
    }

    fn resume(&mut self) -> Result<(), KmsError> {
        self.world().resumes += 1;
        Ok(())
    }
}

/// Opens the fake devices a test registered, by path.
#[derive(Debug, Default)]
pub struct FakeKmsOpener {
    /// The devices that can be opened.
    pub devices: BTreeMap<DevicePath, FakeKms>,
}

impl KmsOpener for FakeKmsOpener {
    fn open(&mut self, path: &DevicePath) -> Result<Box<dyn Kms>, KmsError> {
        match self.devices.get(path) {
            Some(device) => Ok(Box::new(device.clone())),
            None => Err(KmsError::DeviceGone(path.0.clone())),
        }
    }
}
