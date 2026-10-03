//! The wlcs ABI: `wlcs_server_integration!` and the `Wlcs`, `Pointer` and `Touch` traits of the
//! MIT `wlcs` crate. This file is the crate's one `unsafe` module: wlcs names a client and a
//! surface by raw libwayland-client pointers, and only `wl_display_get_fd` and `wl_proxy_get_id`
//! turn them into the ids the compositor uses. Each `unsafe` block carries its SAFETY line.
//! Shape after smithay's `wlcs_anvil` (MIT; THIRD-PARTY.md).
#![allow(unsafe_code)]

use crate::adapter::{
    Button, Edge, Fixed, FixedPoint, Position, WlcsClient, WlcsEvent, WlcsSurface, WlcsTouchId,
};
use crate::descriptor::DESCRIPTOR;
use smithay::reexports::calloop::channel::{Sender, channel};
use std::io::{Error, ErrorKind};
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::JoinHandle;
use wayland_sys::client::{wl_display, wl_display_get_fd, wl_proxy, wl_proxy_get_id};
use wayland_sys::common::wl_fixed_t;
// The names `wlcs_server_integration!` expands to.
use wlcs::ffi_display_server_api::WlcsServerIntegration;
use wlcs::ffi_wrappers::wlcs_server;

/// The handle wlcs creates once per test: the compositor thread and the channel into it.
#[derive(Debug)]
pub struct CasementHandle {
    server: Option<(Sender<WlcsEvent>, JoinHandle<()>)>,
}

wlcs::wlcs_server_integration!(CasementHandle);

/// Touch points are numbered across the process, so two handles never share a slot.
static TOUCH_IDS: AtomicU32 = AtomicU32::new(0);

fn point(x: wl_fixed_t, y: wl_fixed_t) -> FixedPoint {
    FixedPoint {
        x: Fixed(x),
        y: Fixed(y),
    }
}

impl wlcs::Wlcs for CasementHandle {
    type Pointer = PointerHandle;
    type Touch = TouchHandle;

    fn new() -> Self {
        CasementHandle { server: None }
    }

    fn start(&mut self) {
        let (sender, events) = channel();
        let join = std::thread::spawn(move || crate::serve(events));
        self.server = Some((sender, join));
    }

    fn stop(&mut self) {
        if let Some((sender, join)) = self.server.take() {
            let _ = sender.send(WlcsEvent::Exit);
            let _ = join.join();
        }
    }

    fn create_client_socket(&self) -> std::io::Result<OwnedFd> {
        let Some((sender, _)) = &self.server else {
            return Err(Error::from(ErrorKind::NotFound));
        };
        let (client_side, server_side) = UnixStream::pair()?;
        let client = WlcsClient(client_side.as_raw_fd());
        sender
            .send(WlcsEvent::NewClient {
                stream: server_side,
                client,
            })
            .map_err(|e| Error::new(ErrorKind::ConnectionReset, e))?;
        Ok(client_side.into())
    }

    fn position_window_absolute(
        &self,
        display: *mut wl_display,
        surface: *mut wl_proxy,
        x: i32,
        y: i32,
    ) {
        // SAFETY: wlcs passes a live client display for the duration of this call; only its fd
        // number is read.
        let fd = unsafe { wl_display_get_fd(display) };
        // SAFETY: as above, for the surface proxy; only its object id is read.
        let id = unsafe { wl_proxy_get_id(surface) };
        if let Some((sender, _)) = &self.server {
            let _ = sender.send(WlcsEvent::PositionWindow {
                client: WlcsClient(fd),
                surface: WlcsSurface(id),
                at: Position { x, y },
            });
        }
    }

    fn create_pointer(&mut self) -> Option<Self::Pointer> {
        let (sender, _) = self.server.as_ref()?;
        Some(PointerHandle {
            sender: sender.clone(),
        })
    }

    fn create_touch(&mut self) -> Option<Self::Touch> {
        let (sender, _) = self.server.as_ref()?;
        let id = WlcsTouchId(TOUCH_IDS.fetch_add(1, Ordering::Relaxed));
        Some(TouchHandle {
            sender: sender.clone(),
            id,
        })
    }

    fn get_descriptor(&self) -> &wlcs::ffi_display_server_api::WlcsIntegrationDescriptor {
        &DESCRIPTOR
    }
}

/// A wlcs pointer device.
#[derive(Debug)]
pub struct PointerHandle {
    sender: Sender<WlcsEvent>,
}

impl PointerHandle {
    fn button(&self, button: i32, edge: Edge) {
        let button = Button(u32::try_from(button).unwrap_or_default());
        let _ = self.sender.send(WlcsEvent::Button { button, edge });
    }
}

impl wlcs::Pointer for PointerHandle {
    fn move_absolute(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        let _ = self.sender.send(WlcsEvent::PointerTo(point(x, y)));
    }
    fn move_relative(&mut self, dx: wl_fixed_t, dy: wl_fixed_t) {
        let _ = self.sender.send(WlcsEvent::PointerBy(point(dx, dy)));
    }
    fn button_up(&mut self, button: i32) {
        self.button(button, Edge::Release);
    }
    fn button_down(&mut self, button: i32) {
        self.button(button, Edge::Press);
    }
}

/// A wlcs touch device.
#[derive(Debug)]
pub struct TouchHandle {
    sender: Sender<WlcsEvent>,
    id: WlcsTouchId,
}

impl wlcs::Touch for TouchHandle {
    fn touch_down(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        let _ = self.sender.send(WlcsEvent::TouchDown {
            id: self.id,
            at: point(x, y),
        });
    }
    fn touch_move(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        let _ = self.sender.send(WlcsEvent::TouchMove {
            id: self.id,
            at: point(x, y),
        });
    }
    fn touch_up(&mut self) {
        let _ = self.sender.send(WlcsEvent::TouchUp { id: self.id });
    }
}
