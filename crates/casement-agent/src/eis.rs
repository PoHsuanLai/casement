//! Remote input over EIS (P2-T3): the socket the desktop portal's RemoteDesktop session
//! gets, and what comes down it.
//!
//! `org.quire.Compositor1.ConnectEis` asks [`open`] for a socket pair. The client end goes
//! back to the portal (which hands it to the app); the server end is a calloop source,
//! [`RemoteEisSource`], that speaks the EIS protocol with the app, offers the devices the
//! person allowed, and turns what the app sends into [`RemoteInput`]. Every event is tagged
//! `Origin::Remote` here, once, with the portal session's id, so remote input can never pass
//! for the person's own keystrokes (I8, C19). The composition root only stamps the time and
//! feeds the model.
//!
//! Only the portal's unit may ask ([`portal_may_connect`]); it holds no Wayland role.
//! This is the only file that names `reis` (B14). The agent seat's EIS (P4) is not here.

use crate::remote::{Fit, Held, RemoteHandle, RemoteSlot, releases, tagged};
use crate::socket::CallerUnit;
use casement_core::DeviceId;
use casement_core::geometry::{LogicalPoint, LogicalRect, SubPoint, SubPx};
use casement_core::input::{
    AxisFrame, AxisSource, AxisStop, ButtonCode, DeviceClass, Edge, InputEvent, InputKind, KeyCode,
    Origin, PortalSessionId, TouchStep,
};
use casement_core::time::Mono;
use reis::calloop::{EisRequestSource, EisRequestSourceEvent};
use reis::eis;
use reis::request::{Connection, Device, DeviceCapability, EisRequest, Seat};
use smithay::reexports::calloop::{EventSource, Poll, PostAction, Readiness, Token, TokenFactory};
use std::fmt;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;

/// The systemd unit of the desktop portal's backend: the one caller that may ask for a remote
/// EIS socket.
pub const PORTAL_UNIT: &str = "transom.service";

/// Whether `caller` is the portal's unit.
pub fn portal_may_connect(caller: &CallerUnit) -> bool {
    caller.0 == PORTAL_UNIT
}

/// A kind of device a remote session may offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RemoteKind {
    /// A keyboard.
    Keyboard,
    /// A pointer, relative and absolute, with buttons and scrolling.
    Pointer,
    /// A touchscreen.
    Touch,
}

/// The device kinds the person allowed for a session. The wire form is the RemoteDesktop
/// portal's `types` bit set: 1 keyboard, 2 pointer, 4 touchscreen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RemoteKinds(u32);

impl RemoteKinds {
    /// The kinds named by a portal value; other bits are dropped.
    pub fn from_wire(value: u32) -> RemoteKinds {
        RemoteKinds(value & 7)
    }

    /// The portal value.
    pub fn to_wire(self) -> u32 {
        self.0
    }

    /// Whether `kind` is allowed.
    pub fn allows(self, kind: RemoteKind) -> bool {
        let bit = match kind {
            RemoteKind::Keyboard => 1,
            RemoteKind::Pointer => 2,
            RemoteKind::Touch => 4,
        };
        self.0 & bit != 0
    }
}

/// One input event from a remote session, not yet stamped with a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteInput {
    /// Always `Origin::Remote` with the session's id.
    pub origin: Origin,
    /// The virtual device it came from.
    pub device: DeviceId,
    /// What happened.
    pub kind: InputKind,
}

impl RemoteInput {
    /// The model's input event, happening at `at`.
    pub fn at(self, at: Mono) -> InputEvent {
        InputEvent {
            origin: self.origin,
            device: self.device,
            at,
            kind: self.kind,
        }
    }
}

/// What a [`RemoteEisSource`] yields.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemoteEvent {
    /// An input event.
    Input(RemoteInput),
    /// The app went away (or broke the protocol); keys it held have been released first.
    Ended,
}

/// A new remote connection: the source for the event loop and the client end for the portal.
#[derive(Debug)]
pub struct RemoteGrant {
    /// Insert this into the event loop; it yields the session's input.
    pub source: RemoteEisSource,
    /// The client end of the socket, returned to the portal.
    pub client: OwnedFd,
    /// What the compositor keeps to end the connection from outside (`EndEis`).
    pub handle: RemoteHandle,
}

/// Opens a remote EIS connection for portal session `session`, numbered `slot` (minted by the
/// compositor, one per connection), offering `kinds`. Absolute pointer and touch positions are
/// over `extent` (the union of the outputs now).
pub fn open(
    session: PortalSessionId,
    slot: RemoteSlot,
    kinds: RemoteKinds,
    extent: LogicalRect,
) -> io::Result<RemoteGrant> {
    let (server, client) = UnixStream::pair()?;
    server.set_nonblocking(true)?;
    let context = eis::Context::new(server)?;
    let link = Link::new(session, slot, kinds, extent);
    let handle = RemoteHandle::new(session, slot, link.held.clone());
    Ok(RemoteGrant {
        source: RemoteEisSource {
            inner: EisRequestSource::new(context, 1),
            link,
        },
        client: OwnedFd::from(client),
        handle,
    })
}

/// The EIS side of one remote session, as a calloop source.
pub struct RemoteEisSource {
    inner: EisRequestSource,
    link: Link,
}

impl fmt::Debug for RemoteEisSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemoteEisSource")
            .field("link", &self.link)
            .finish_non_exhaustive()
    }
}

impl EventSource for RemoteEisSource {
    type Event = RemoteEvent;
    type Metadata = ();
    type Ret = ();
    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(RemoteEvent, &mut ()),
    {
        let RemoteEisSource { inner, link } = self;
        inner.process_events(readiness, token, |event, connection| {
            let (events, action) = link.handle(connection, event);
            for event in events {
                callback(event, &mut ());
            }
            // A client that already hung up is found on the next read.
            let _ = connection.flush();
            Ok(action)
        })
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> smithay::reexports::calloop::Result<()> {
        self.inner.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> smithay::reexports::calloop::Result<()> {
        self.inner.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> smithay::reexports::calloop::Result<()> {
        self.inner.unregister(poll)
    }
}

/// Which of our devices a handle is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Made {
    Keyboard,
    Pointer,
    PointerAbsolute,
    Touch,
}

/// The protocol state of one connection: what was offered and what is held down.
struct Link {
    session: PortalSessionId,
    slot: RemoteSlot,
    kinds: RemoteKinds,
    extent: LogicalRect,
    seat: Option<Seat>,
    devices: Vec<(Made, Device)>,
    held: Held,
}

impl fmt::Debug for Link {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Link")
            .field("session", &self.session)
            .field("slot", &self.slot)
            .field("kinds", &self.kinds)
            .field("devices", &self.devices.len())
            .finish_non_exhaustive()
    }
}

impl Link {
    fn new(
        session: PortalSessionId,
        slot: RemoteSlot,
        kinds: RemoteKinds,
        extent: LogicalRect,
    ) -> Link {
        Link {
            session,
            slot,
            kinds,
            extent,
            seat: None,
            devices: Vec::new(),
            held: Held::default(),
        }
    }

    /// One step of the connection: the events it yields and whether it goes on.
    fn handle(
        &mut self,
        connection: &Connection,
        event: Result<EisRequestSourceEvent, reis::Error>,
    ) -> (Vec<RemoteEvent>, PostAction) {
        match event {
            Err(_) => (self.end(), PostAction::Remove),
            Ok(EisRequestSourceEvent::Connected) => {
                self.seat = Some(connection.add_seat(Some("remote"), self.seat_capabilities()));
                (Vec::new(), PostAction::Continue)
            }
            Ok(EisRequestSourceEvent::Request(EisRequest::Disconnect)) => {
                (self.end(), PostAction::Remove)
            }
            Ok(EisRequestSourceEvent::Request(request)) => {
                (self.request(connection, request), PostAction::Continue)
            }
        }
    }

    /// What the seat advertises: only what the person allowed.
    fn seat_capabilities(&self) -> reis::enumflags2::BitFlags<DeviceCapability> {
        let mut caps = reis::enumflags2::BitFlags::empty();
        if self.kinds.allows(RemoteKind::Keyboard) {
            caps |= DeviceCapability::Keyboard;
        }
        if self.kinds.allows(RemoteKind::Pointer) {
            caps |= DeviceCapability::Pointer
                | DeviceCapability::PointerAbsolute
                | DeviceCapability::Button
                | DeviceCapability::Scroll;
        }
        if self.kinds.allows(RemoteKind::Touch) {
            caps |= DeviceCapability::Touch;
        }
        caps
    }

    fn request(&mut self, connection: &Connection, request: EisRequest) -> Vec<RemoteEvent> {
        match request {
            EisRequest::Bind(bind) => {
                self.bind(connection, &bind.seat, bind.capabilities);
                Vec::new()
            }
            other => self
                .input(other)
                .into_iter()
                .map(RemoteEvent::Input)
                .collect(),
        }
    }

    /// Creates the devices a bind asked for (once each, only what is allowed). An app that
    /// wants to receive input gets none: this edge only takes input in.
    fn bind(
        &mut self,
        connection: &Connection,
        seat: &Seat,
        wanted: reis::enumflags2::BitFlags<DeviceCapability>,
    ) {
        if connection.context_type() != eis::handshake::ContextType::Sender {
            return;
        }
        let plans = [
            (
                Made::Keyboard,
                RemoteKind::Keyboard,
                "remote keyboard",
                DeviceCapability::Keyboard,
                vec![DeviceCapability::Keyboard],
            ),
            (
                Made::Pointer,
                RemoteKind::Pointer,
                "remote pointer",
                DeviceCapability::Pointer,
                vec![
                    DeviceCapability::Pointer,
                    DeviceCapability::Button,
                    DeviceCapability::Scroll,
                ],
            ),
            (
                Made::PointerAbsolute,
                RemoteKind::Pointer,
                "remote pointer (absolute)",
                DeviceCapability::PointerAbsolute,
                vec![
                    DeviceCapability::PointerAbsolute,
                    DeviceCapability::Button,
                    DeviceCapability::Scroll,
                ],
            ),
            (
                Made::Touch,
                RemoteKind::Touch,
                "remote touchscreen",
                DeviceCapability::Touch,
                vec![DeviceCapability::Touch],
            ),
        ];
        for (made, kind, name, asked, caps) in plans {
            let new = wanted.contains(asked)
                && self.kinds.allows(kind)
                && !self.devices.iter().any(|(have, _)| *have == made);
            if !new {
                continue;
            }
            let (width, height) = (self.extent.size.w, self.extent.size.h);
            let device = seat.add_device(
                Some(name),
                eis::device::DeviceType::Virtual,
                caps.into_iter().collect(),
                |device| {
                    if matches!(made, Made::PointerAbsolute | Made::Touch) {
                        device.device().region(0, 0, width, height, 1.0);
                    }
                },
            );
            device.resumed();
            self.devices.push((made, device));
        }
    }

    /// The model's input for one request; none for the requests that carry none.
    fn input(&mut self, request: EisRequest) -> Vec<RemoteInput> {
        let (class, kind) = match request {
            EisRequest::KeyboardKey(key) => {
                if self.held.key(key.key, key_edge(key.state)) == Fit::Full {
                    return Vec::new();
                }
                (DeviceClass::Keyboard, key_kind(key.key, key.state))
            }
            EisRequest::PointerMotion(motion) => {
                (DeviceClass::Pointer, motion_kind(motion.dx, motion.dy))
            }
            EisRequest::PointerMotionAbsolute(motion) => (
                DeviceClass::Pointer,
                motion_to_kind(self.extent, motion.dx_absolute, motion.dy_absolute),
            ),
            EisRequest::Button(button) => {
                if self.held.button(button.button, button_edge(button.state)) == Fit::Full {
                    return Vec::new();
                }
                (
                    DeviceClass::Pointer,
                    button_kind(button.button, button.state),
                )
            }
            EisRequest::ScrollDelta(scroll) => {
                (DeviceClass::Pointer, scroll_kind(scroll.dx, scroll.dy))
            }
            EisRequest::ScrollDiscrete(scroll) => (
                DeviceClass::Pointer,
                discrete_kind(scroll.discrete_dx, scroll.discrete_dy),
            ),
            EisRequest::ScrollStop(_) | EisRequest::ScrollCancel(_) => {
                (DeviceClass::Pointer, scroll_stop_kind())
            }
            EisRequest::TouchDown(touch) => (
                DeviceClass::Touch,
                touch_kind(
                    TouchWhere::Down,
                    self.extent,
                    touch.touch_id,
                    touch.x,
                    touch.y,
                ),
            ),
            EisRequest::TouchMotion(touch) => (
                DeviceClass::Touch,
                touch_kind(
                    TouchWhere::Motion,
                    self.extent,
                    touch.touch_id,
                    touch.x,
                    touch.y,
                ),
            ),
            EisRequest::TouchUp(touch) => (
                DeviceClass::Touch,
                InputKind::Touch(TouchStep::Up {
                    slot: touch.touch_id,
                }),
            ),
            EisRequest::TouchCancel(_) => (DeviceClass::Touch, InputKind::Touch(TouchStep::Cancel)),
            _ => return Vec::new(),
        };
        // A device whose kind the person did not allow never sends, but the table is the
        // authority, not the client's say-so.
        if !self.allowed(class) {
            return Vec::new();
        }
        let mut out = vec![self.tag(class, kind)];
        if matches!(class, DeviceClass::Touch)
            && matches!(
                out[0].kind,
                InputKind::Touch(
                    TouchStep::Down { .. } | TouchStep::Motion { .. } | TouchStep::Up { .. }
                )
            )
        {
            // A touch frame closes each step.
            out.push(self.tag(class, InputKind::Touch(TouchStep::Frame)));
        }
        out
    }

    fn allowed(&self, class: DeviceClass) -> bool {
        match class {
            DeviceClass::Keyboard => self.kinds.allows(RemoteKind::Keyboard),
            DeviceClass::Touch => self.kinds.allows(RemoteKind::Touch),
            _ => self.kinds.allows(RemoteKind::Pointer),
        }
    }

    fn tag(&self, class: DeviceClass, kind: InputKind) -> RemoteInput {
        tagged(self.session, self.slot, class, kind)
    }

    /// The connection is over: release what the app held (nothing, when the compositor ended
    /// the session first), then say so.
    fn end(&mut self) -> Vec<RemoteEvent> {
        releases(self.session, self.slot, &self.held)
            .into_iter()
            .map(RemoteEvent::Input)
            .chain(std::iter::once(RemoteEvent::Ended))
            .collect()
    }
}

/// Sub-pixels per logical pixel.
const SUB_PER_LOGICAL: f32 = 120.0;

fn sub_px(logical: f32) -> SubPx {
    let scaled = (logical * SUB_PER_LOGICAL).round();
    // NaN compares false and becomes 0; the clamp makes the narrowing exact.
    let held = if scaled.is_nan() {
        0.0
    } else {
        scaled.clamp(-2.0e9, 2.0e9)
    };
    SubPx(held as i32)
}

fn sub_point(x: f32, y: f32) -> SubPoint {
    SubPoint {
        x: sub_px(x),
        y: sub_px(y),
    }
}

fn whole(value: f32) -> i32 {
    if value.is_nan() {
        return 0;
    }
    value.round().clamp(-2.0e9, 2.0e9) as i32
}

/// A position over `extent`, which the app sees as `(0, 0)` to its size.
fn placed(extent: LogicalRect, x: f32, y: f32) -> LogicalPoint {
    let span = |side: u32| i32::try_from(side).unwrap_or(i32::MAX);
    let inside = |at: f32, side: u32| whole(at).clamp(0, span(side).saturating_sub(1).max(0));
    LogicalPoint {
        x: extent.origin.x.saturating_add(inside(x, extent.size.w)),
        y: extent.origin.y.saturating_add(inside(y, extent.size.h)),
    }
}

fn key_edge(state: eis::keyboard::KeyState) -> Edge {
    match state {
        eis::keyboard::KeyState::Press => Edge::Press,
        _ => Edge::Release,
    }
}

fn button_edge(state: eis::button::ButtonState) -> Edge {
    match state {
        eis::button::ButtonState::Press => Edge::Press,
        _ => Edge::Release,
    }
}

fn key_kind(code: u32, state: eis::keyboard::KeyState) -> InputKind {
    InputKind::Key {
        code: KeyCode(code),
        edge: key_edge(state),
    }
}

fn button_kind(code: u32, state: eis::button::ButtonState) -> InputKind {
    InputKind::Button {
        button: ButtonCode(code),
        edge: button_edge(state),
    }
}

fn motion_kind(dx: f32, dy: f32) -> InputKind {
    InputKind::Motion {
        delta: sub_point(dx, dy),
    }
}

fn motion_to_kind(extent: LogicalRect, x: f32, y: f32) -> InputKind {
    InputKind::MotionTo {
        at: placed(extent, x, y),
    }
}

fn scroll_kind(dx: f32, dy: f32) -> InputKind {
    InputKind::Axis(AxisFrame {
        source: AxisSource::Continuous,
        v120: (0, 0),
        px: sub_point(dx, dy),
        stop: AxisStop::Running,
    })
}

/// EIS discrete scrolling counts 120 to a wheel click, the same unit as `v120`.
fn discrete_kind(dx: i32, dy: i32) -> InputKind {
    InputKind::Axis(AxisFrame {
        source: AxisSource::Wheel,
        v120: (dx, dy),
        px: sub_point(0.0, 0.0),
        stop: AxisStop::Running,
    })
}

fn scroll_stop_kind() -> InputKind {
    InputKind::Axis(AxisFrame {
        source: AxisSource::Continuous,
        v120: (0, 0),
        px: sub_point(0.0, 0.0),
        stop: AxisStop::Stopped,
    })
}

/// Which touch step a position belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TouchWhere {
    Down,
    Motion,
}

fn touch_kind(which: TouchWhere, extent: LogicalRect, slot: u32, x: f32, y: f32) -> InputKind {
    let at = placed(extent, x, y);
    InputKind::Touch(match which {
        TouchWhere::Down => TouchStep::Down { slot, at },
        TouchWhere::Motion => TouchStep::Motion { slot, at },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::LogicalSize;

    fn extent() -> LogicalRect {
        LogicalRect::new(
            LogicalPoint { x: -100, y: 0 },
            LogicalSize { w: 1000, h: 500 },
        )
    }

    #[test]
    fn only_the_portals_unit_may_ask() {
        let unit = |name: &str| CallerUnit(name.to_owned());
        assert!(portal_may_connect(&unit("transom.service")));
        for other in ["sill.service", "cuad.service", "firefox.service", ""] {
            assert!(!portal_may_connect(&unit(other)), "{other}");
        }
    }

    /// (case, the portal's value, keyboard, pointer, touch)
    #[test]
    fn the_portals_device_bits_are_read() {
        let cases = [
            ("none", 0, false, false, false),
            ("keyboard", 1, true, false, false),
            ("pointer", 2, false, true, false),
            ("touch", 4, false, false, true),
            ("all", 7, true, true, true),
            ("unknown bits are dropped", 8, false, false, false),
        ];
        for (case, wire, keyboard, pointer, touch) in cases {
            let kinds = RemoteKinds::from_wire(wire);
            assert_eq!(kinds.allows(RemoteKind::Keyboard), keyboard, "{case}");
            assert_eq!(kinds.allows(RemoteKind::Pointer), pointer, "{case}");
            assert_eq!(kinds.allows(RemoteKind::Touch), touch, "{case}");
        }
    }

    #[test]
    fn a_key_keeps_its_code_and_edge() {
        assert_eq!(
            key_kind(30, eis::keyboard::KeyState::Press),
            InputKind::Key {
                code: KeyCode(30),
                edge: Edge::Press
            }
        );
        assert_eq!(
            button_kind(0x110, eis::button::ButtonState::Released),
            InputKind::Button {
                button: ButtonCode(0x110),
                edge: Edge::Release
            }
        );
    }

    #[test]
    fn motion_is_in_sub_pixels_and_absolute_positions_land_on_the_desktop() {
        assert_eq!(
            motion_kind(1.5, -2.0),
            InputKind::Motion {
                delta: SubPoint {
                    x: SubPx(180),
                    y: SubPx(-240)
                }
            }
        );
        // (case, the app's position, where it lands over an extent that starts at x = -100)
        let cases = [
            ("the corner", (0.0, 0.0), (-100, 0)),
            ("inside", (10.4, 20.6), (-90, 21)),
            ("past the far edge is held in", (5000.0, 5000.0), (899, 499)),
            ("before the near edge is held in", (-5.0, -5.0), (-100, 0)),
            ("not a number", (f32::NAN, 1.0), (-100, 1)),
        ];
        for (case, (x, y), (want_x, want_y)) in cases {
            assert_eq!(
                motion_to_kind(extent(), x, y),
                InputKind::MotionTo {
                    at: LogicalPoint {
                        x: want_x,
                        y: want_y
                    }
                },
                "{case}"
            );
        }
    }

    #[test]
    fn a_wheel_click_is_120_and_a_stop_ends_the_scroll() {
        let InputKind::Axis(wheel) = discrete_kind(0, 120) else {
            panic!("an axis");
        };
        assert_eq!(wheel.v120, (0, 120));
        assert_eq!(wheel.source, AxisSource::Wheel);
        let InputKind::Axis(stop) = scroll_stop_kind() else {
            panic!("an axis");
        };
        assert_eq!(stop.stop, AxisStop::Stopped);
    }

    #[test]
    fn every_event_of_a_session_is_remote_and_its_devices_are_apart() {
        let session = PortalSessionId(9);
        let link = Link::new(
            session,
            RemoteSlot::default(),
            RemoteKinds::from_wire(7),
            extent(),
        );
        for class in [
            DeviceClass::Keyboard,
            DeviceClass::Pointer,
            DeviceClass::Touch,
        ] {
            let input = link.tag(class, InputKind::Touch(TouchStep::Cancel));
            assert_eq!(input.origin, Origin::Remote(session));
            assert!(!input.origin.fires_bindings());
            assert!(!input.origin.is_physical());
        }
        let ids = [
            DeviceClass::Keyboard,
            DeviceClass::Pointer,
            DeviceClass::Touch,
        ]
        .map(|class| link.tag(class, InputKind::Touch(TouchStep::Cancel)).device);
        assert!(ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2]);
    }

    #[test]
    fn ending_releases_what_was_held() {
        let mut link = Link::new(
            PortalSessionId(3),
            RemoteSlot::default(),
            RemoteKinds::from_wire(7),
            extent(),
        );
        link.held.key(30, Edge::Press);
        link.held.key(31, Edge::Press);
        link.held.key(31, Edge::Release);
        link.held.button(0x110, Edge::Press);
        let events = link.end();
        let kinds: Vec<_> = events
            .iter()
            .map(|event| match event {
                RemoteEvent::Input(input) => Some(input.kind.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                Some(InputKind::Key {
                    code: KeyCode(30),
                    edge: Edge::Release
                }),
                Some(InputKind::Button {
                    button: ButtonCode(0x110),
                    edge: Edge::Release
                }),
                None,
            ]
        );
        assert_eq!(events.last(), Some(&RemoteEvent::Ended));
    }

    /// PORTAL-05: when the compositor ended the session first (through the handle), the source
    /// finding the socket closed has nothing left to release.
    #[test]
    fn a_session_ended_from_outside_releases_once() {
        let grant = open(
            PortalSessionId(5),
            RemoteSlot::default(),
            RemoteKinds::from_wire(7),
            extent(),
        )
        .expect("open");
        let RemoteGrant {
            mut source, handle, ..
        } = grant;
        source.link.held.key(30, Edge::Press);
        assert_eq!(
            handle.release().len(),
            1,
            "the held key is released from outside"
        );
        assert_eq!(source.link.end(), vec![RemoteEvent::Ended], "and not again");
    }

    #[test]
    fn a_connection_opens_with_a_client_end() {
        let grant = open(
            PortalSessionId(1),
            RemoteSlot::default(),
            RemoteKinds::from_wire(3),
            extent(),
        )
        .expect("open");
        assert!(grant.source.link.devices.is_empty());
    }
}
