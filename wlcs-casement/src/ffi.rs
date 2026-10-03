//! The wlcs ABI: `wlcs_server_integration!` and the `Wlcs`, `Pointer` and `Touch` traits of the
//! MIT `wlcs` crate. This file is the crate's one `unsafe` module: wlcs names a client and a
//! surface by raw libwayland-client pointers, and only `wl_display_get_fd` and `wl_proxy_get_id`
//! turn them into the ids the adapter uses. Each `unsafe` block carries its SAFETY line.
//!
//! The bodies are the fill of lane P0-T-wlcs.
#![allow(unsafe_code)]

use crate::adapter::WlcsEvent;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;
use wayland_sys::client::{wl_display, wl_proxy};
use wayland_sys::common::wl_fixed_t;
// The names `wlcs_server_integration!` expands to.
use wlcs::ffi_display_server_api::WlcsServerIntegration;
use wlcs::ffi_wrappers::wlcs_server;

/// The handle wlcs creates once per test: the compositor thread and the channel into it.
#[derive(Debug)]
pub struct CasementHandle {
    #[expect(dead_code, reason = "P0-T-wlcs: the compositor thread and its channel")]
    server: Option<(Sender<WlcsEvent>, JoinHandle<()>)>,
}

wlcs::wlcs_server_integration!(CasementHandle);

impl wlcs::Wlcs for CasementHandle {
    type Pointer = PointerHandle;
    type Touch = TouchHandle;

    fn new() -> Self {
        CasementHandle { server: None }
    }

    fn start(&mut self) {
        todo!("P0-T-wlcs: spawn crate::serve on its own thread")
    }

    fn stop(&mut self) {
        todo!("P0-T-wlcs: send Exit and join")
    }

    fn create_client_socket(&self) -> std::io::Result<std::os::fd::OwnedFd> {
        todo!("P0-T-wlcs: a socket pair, the server end sent as NewClient")
    }

    fn position_window_absolute(
        &self,
        display: *mut wl_display,
        surface: *mut wl_proxy,
        x: i32,
        y: i32,
    ) {
        // SAFETY (to be written with the body): wlcs passes a live client display and surface
        // proxy for the duration of this call; only their fd and object id are read.
        let _ = (display, surface, x, y);
        todo!("P0-T-wlcs: PositionWindow from wl_display_get_fd and wl_proxy_get_id")
    }

    fn create_pointer(&mut self) -> Option<Self::Pointer> {
        todo!("P0-T-wlcs")
    }

    fn create_touch(&mut self) -> Option<Self::Touch> {
        todo!("P0-T-wlcs")
    }

    fn get_descriptor(&self) -> &wlcs::ffi_display_server_api::WlcsIntegrationDescriptor {
        todo!("P0-T-wlcs: the extension list from tests/golden/globals.txt")
    }
}

/// A wlcs pointer device.
#[derive(Debug)]
pub struct PointerHandle {
    sender: Sender<WlcsEvent>,
}

impl wlcs::Pointer for PointerHandle {
    fn move_absolute(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        let _ = (&self.sender, x, y);
        todo!("P0-T-wlcs")
    }
    fn move_relative(&mut self, dx: wl_fixed_t, dy: wl_fixed_t) {
        let _ = (dx, dy);
        todo!("P0-T-wlcs")
    }
    fn button_up(&mut self, button: i32) {
        let _ = button;
        todo!("P0-T-wlcs")
    }
    fn button_down(&mut self, button: i32) {
        let _ = button;
        todo!("P0-T-wlcs")
    }
}

/// A wlcs touch device.
#[derive(Debug)]
pub struct TouchHandle {
    sender: Sender<WlcsEvent>,
}

impl wlcs::Touch for TouchHandle {
    fn touch_down(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        let _ = (&self.sender, x, y);
        todo!("P0-T-wlcs")
    }
    fn touch_move(&mut self, x: wl_fixed_t, y: wl_fixed_t) {
        let _ = (x, y);
        todo!("P0-T-wlcs")
    }
    fn touch_up(&mut self) {
        todo!("P0-T-wlcs")
    }
}
