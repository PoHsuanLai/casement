//! Input into a fixture: virtual devices on the same path libinput takes (11 §2.5), each
//! event stamped with the fixture's now and the origin the test minted.

use crate::fixture::Fixture;
use crate::origin::TestOrigin;
use casement_core::DeviceId;
use casement_core::geometry::LogicalPoint;
use casement_core::geometry::SubPoint;
use casement_core::input::{
    AxisFrame, ButtonCode, DeviceClass, Edge, GestureStep, KeyCode, PinchDelta, SwitchStep,
    TouchStep,
};

/// The fixture's input, borrowed.
#[derive(Debug)]
pub struct FixtureInput<'a> {
    #[expect(dead_code, reason = "P0-B-headless: the fixture the input goes to")]
    fixture: &'a mut Fixture,
}

impl<'a> FixtureInput<'a> {
    /// The input of `fixture`.
    pub fn of(fixture: &'a mut Fixture) -> FixtureInput<'a> {
        FixtureInput { fixture }
    }

    /// Plugs a device of `class` with `origin`.
    pub fn plug(&mut self, class: DeviceClass, origin: TestOrigin) -> DeviceId {
        let _ = (class, origin);
        todo!("P0-B-headless: FixtureInput::plug")
    }

    /// A key on `device`, then a pump.
    pub fn key(&mut self, device: DeviceId, code: KeyCode, edge: Edge) {
        let _ = (device, code, edge);
        todo!("P0-B-headless: FixtureInput::key")
    }

    /// Moves the pointer of `device` to `at`, then pumps.
    pub fn pointer_to(&mut self, device: DeviceId, at: LogicalPoint) {
        let _ = (device, at);
        todo!("P0-B-headless: FixtureInput::pointer_to")
    }

    /// A button on `device`, then a pump.
    pub fn button(&mut self, device: DeviceId, button: ButtonCode, edge: Edge) {
        let _ = (device, button, edge);
        todo!("P0-B-headless: FixtureInput::button")
    }

    /// One scroll frame on `device`, then a pump.
    pub fn axis(&mut self, device: DeviceId, frame: AxisFrame) {
        let _ = (device, frame);
        todo!("P0-B-headless: FixtureInput::axis")
    }

    /// A swipe step on `device`.
    pub fn swipe(&mut self, device: DeviceId, step: GestureStep<SubPoint>) {
        let _ = (device, step);
        todo!("P0-B-headless: FixtureInput::swipe")
    }

    /// A pinch step on `device`.
    pub fn pinch(&mut self, device: DeviceId, step: GestureStep<PinchDelta>) {
        let _ = (device, step);
        todo!("P0-B-headless: FixtureInput::pinch")
    }

    /// A hold step on `device`.
    pub fn hold(&mut self, device: DeviceId, step: GestureStep<()>) {
        let _ = (device, step);
        todo!("P0-B-headless: FixtureInput::hold")
    }

    /// A touch step on `device`.
    pub fn touch(&mut self, device: DeviceId, step: TouchStep) {
        let _ = (device, step);
        todo!("P0-B-headless: FixtureInput::touch")
    }

    /// A switch change on `device`.
    pub fn switch(&mut self, device: DeviceId, step: SwitchStep) {
        let _ = (device, step);
        todo!("P0-B-headless: FixtureInput::switch")
    }
}
