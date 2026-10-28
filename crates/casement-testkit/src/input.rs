//! Input into a fixture: virtual devices on the same path libinput takes (11 §2.5), each
//! event stamped with the fixture's now and the origin the test minted.

use crate::fixture::Fixture;
use crate::origin::TestOrigin;
use casement_backend::{DeviceName, DeviceSpec};
use casement_core::DeviceId;
use casement_core::geometry::LogicalPoint;
use casement_core::geometry::SubPoint;
use casement_core::input::{
    AxisFrame, ButtonCode, DeviceClass, GestureStep, InputKind, KeyCode, PinchDelta, SwitchStep,
    TouchStep,
};
use casement_protocols::vocab::PressEdge;

/// The fixture's input, borrowed.
#[derive(Debug)]
pub struct FixtureInput<'a> {
    fixture: &'a mut Fixture,
}

impl<'a> FixtureInput<'a> {
    /// The input of `fixture`.
    pub fn of(fixture: &'a mut Fixture) -> FixtureInput<'a> {
        FixtureInput { fixture }
    }

    /// Plugs a device of `class` with `origin`, then pumps.
    pub fn plug(&mut self, class: DeviceClass, origin: TestOrigin) -> DeviceId {
        let layout = self.fixture.model().settings().keyboard.layout.clone();
        let spec = DeviceSpec {
            class,
            origin: origin.origin(),
            layout,
            name: DeviceName(format!("fixture {class:?}")),
        };
        let id = self.fixture.headless_mut().input().plug(spec);
        self.fixture.pump();
        id
    }

    /// `kind` from `device` at the fixture's now, then a pump.
    fn push(&mut self, device: DeviceId, kind: InputKind) {
        let at = self.fixture.now();
        self.fixture.headless_mut().input().push(device, kind, at);
        self.fixture.pump();
    }

    /// A key on `device`, then a pump.
    pub fn key(&mut self, device: DeviceId, code: KeyCode, edge: PressEdge) {
        self.push(device, InputKind::Key { code, edge });
    }

    /// Moves the pointer of `device` to `at`, then pumps.
    pub fn pointer_to(&mut self, device: DeviceId, at: LogicalPoint) {
        self.push(device, InputKind::MotionTo { at });
    }

    /// Moves the pointer of `device` by `delta` (a relative motion: what a mouse sends), then
    /// pumps.
    pub fn pointer_by(&mut self, device: DeviceId, delta: SubPoint) {
        self.push(device, InputKind::Motion { delta });
    }

    /// A button on `device`, then a pump.
    pub fn button(&mut self, device: DeviceId, button: ButtonCode, edge: PressEdge) {
        self.push(device, InputKind::Button { button, edge });
    }

    /// One scroll frame on `device`, then a pump.
    pub fn axis(&mut self, device: DeviceId, frame: AxisFrame) {
        self.push(device, InputKind::Axis(frame));
    }

    /// A swipe step on `device`, then a pump.
    pub fn swipe(&mut self, device: DeviceId, step: GestureStep<SubPoint>) {
        self.push(device, InputKind::Swipe(step));
    }

    /// A pinch step on `device`, then a pump.
    pub fn pinch(&mut self, device: DeviceId, step: GestureStep<PinchDelta>) {
        self.push(device, InputKind::Pinch(step));
    }

    /// A hold step on `device`, then a pump.
    pub fn hold(&mut self, device: DeviceId, step: GestureStep<()>) {
        self.push(device, InputKind::Hold(step));
    }

    /// A touch step on `device`, then a pump.
    pub fn touch(&mut self, device: DeviceId, step: TouchStep) {
        self.push(device, InputKind::Touch(step));
    }

    /// A switch change on `device`, then a pump.
    pub fn switch(&mut self, device: DeviceId, step: SwitchStep) {
        self.push(device, InputKind::Switch(step));
    }
}
