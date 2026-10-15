//! Plugging and unplugging virtual outputs: the same `ConnectorEvent` path KMS hotplug takes.
//! And what an output shows: shots of its last presented frame.

use crate::fixture::{Fixture, ShotTarget};
use casement::core_loop;
use casement_backend::{FramePixels, VirtualOutputSpec};
use casement_core::geometry::{LogicalPoint, LogicalRect, to_physical};
use casement_core::output::{GammaRamp, Mode};
use casement_core::{Event, OutputName};

impl Fixture {
    /// Plugs an output and pumps; returns its connector name.
    pub fn plug(&mut self, spec: VirtualOutputSpec) -> OutputName {
        let name = core_loop::plug_output(self.state_mut(), spec);
        self.pump();
        name
    }

    /// Unplugs `name` and pumps.
    pub fn unplug(&mut self, name: &OutputName) {
        core_loop::unplug_output(self.state_mut(), name);
        self.pump();
    }

    /// Puts `scale` on `name` as if its display had been reconfigured (mode, transform and
    /// position kept) and pumps. An output the model does not know changes nothing.
    pub fn rescale(&mut self, name: &OutputName, scale: ds_core::geometry::scale::Scale) {
        let known = self
            .model()
            .outputs()
            .0
            .iter()
            .find(|(_, state)| state.facts.name == *name)
            .map(|(key, state)| (key.clone(), state.config));
        if let Some((key, config)) = known {
            let config = casement_core::output::OutputConfig { scale, ..config };
            core_loop::configure_output(self.state_mut(), &key, config);
        }
        self.pump();
    }

    /// Gives `name` the modes in `extra` besides the ones it offers (a display with several
    /// reports them all) and pumps. An output the model does not know changes nothing.
    pub fn offer_modes(&mut self, name: &OutputName, extra: &[Mode]) {
        let known = self
            .model()
            .outputs()
            .0
            .values()
            .find(|state| state.facts.name == *name)
            .map(|state| state.facts.clone());
        if let Some(mut facts) = known {
            let new: Vec<Mode> = extra
                .iter()
                .filter(|mode| !facts.modes.contains(mode))
                .copied()
                .collect();
            facts.modes.extend(new);
            core_loop::feed(self.state_mut(), vec![Event::OutputPlugged(facts)]);
        }
        self.pump();
    }

    /// The gamma ramp a client last set on `name` (the identity curve until one does); none
    /// when it is not plugged.
    pub fn gamma_of(&self, name: &OutputName) -> Option<GammaRamp> {
        self.headless().gamma_of(name).cloned()
    }

    /// A shot of `target`, from the last presented frame. A window is cut out of the frame of
    /// the output it is on (the frame it is drawn in, not a render of it alone).
    pub fn shot(&mut self, target: ShotTarget) -> image::RgbaImage {
        let (output, region) = self.shot_region(target);
        let frame = match self.last_pixels(&output) {
            Some(FramePixels::Rgba(frame)) => frame,
            Some(FramePixels::Dummy) => {
                panic!("{} is drawn by the dummy renderer", output.as_str())
            }
            Some(FramePixels::Resident(_)) => {
                panic!("{} keeps its frame on the GPU", output.as_str())
            }
            None => panic!("{} presented no frame: advance first", output.as_str()),
        };
        let whole = image::RgbaImage::from_raw(frame.size.w, frame.size.h, frame.bytes.clone())
            .expect("a read-back frame is w x h x 4 bytes");
        match region {
            None => whole,
            Some(rect) => {
                image::imageops::crop_imm(&whole, rect.x, rect.y, rect.w, rect.h).to_image()
            }
        }
    }

    /// The output a shot is of, and the part of its frame in physical pixels (`None`: all).
    fn shot_region(&self, target: ShotTarget) -> (OutputName, Option<ShotRect>) {
        match target {
            ShotTarget::Output(name) => (name, None),
            ShotTarget::Region(name, rect) => {
                let region = self.physical_of(&name, rect);
                (name, Some(region))
            }
            ShotTarget::Window(window) => {
                let arranged = self
                    .state()
                    .core
                    .arrangement
                    .windows
                    .iter()
                    .find(|target| target.window == window)
                    .unwrap_or_else(|| panic!("window {} is not arranged", window.0));
                let name = self
                    .model()
                    .outputs()
                    .0
                    .get(&arranged.output)
                    .map(|output| output.facts.name.clone())
                    .expect("a window's output is plugged");
                let region = self.physical_of(&name, arranged.rect);
                (name, Some(region))
            }
        }
    }

    /// `rect` (global logical) in `name`'s frame, clipped to it.
    fn physical_of(&self, name: &OutputName, rect: LogicalRect) -> ShotRect {
        let output = self
            .headless()
            .outputs()
            .get(name)
            .unwrap_or_else(|| panic!("{} is not plugged", name.as_str()));
        let local = LogicalRect {
            origin: LogicalPoint {
                x: rect.origin.x - output.spec.position.x,
                y: rect.origin.y - output.spec.position.y,
            },
            size: rect.size,
        };
        let wanted = to_physical(local, output.spec.scale);
        let clip = |edge: i64, limit: u32| u32::try_from(edge.max(0)).unwrap_or(0).min(limit);
        let (w, h) = (output.spec.mode.size.w, output.spec.mode.size.h);
        let left = clip(i64::from(wanted.x), w);
        let top = clip(i64::from(wanted.y), h);
        let right = clip(i64::from(wanted.x) + i64::from(wanted.w), w);
        let bottom = clip(i64::from(wanted.y) + i64::from(wanted.h), h);
        ShotRect {
            x: left,
            y: top,
            w: right.saturating_sub(left),
            h: bottom.saturating_sub(top),
        }
    }
}

/// A region of a frame, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShotRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}
