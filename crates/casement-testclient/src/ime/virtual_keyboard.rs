//! `zwp_virtual_keyboard_v1`: the client side of an on-screen keyboard. Filled by P0-H7.
//! The keyboard has no events, so the client state needs no sink: it names
//! [`VirtualKeyboardDelegate`] for the manager and the keyboard with
//! `wayland_client::delegate_dispatch!`.

use crate::ClientError;
use crate::GlobalList;
use crate::misc::idle::side_queue;
use rustix::fs::{MemfdFlags, memfd_create};
use std::os::fd::OwnedFd;
use wayland_client::protocol::wl_keyboard::KeymapFormat;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, delegate_dispatch};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

/// The user data of a virtual keyboard.
#[derive(Debug, Clone, Copy, Default)]
pub struct VirtualKeyboardData;

/// The `Dispatch` impls for the manager and the keyboard (neither has events).
#[derive(Debug, Clone, Copy)]
pub struct VirtualKeyboardDelegate;

impl<S> Dispatch<ZwpVirtualKeyboardManagerV1, (), S> for VirtualKeyboardDelegate
where
    S: Dispatch<ZwpVirtualKeyboardManagerV1, ()>,
{
    fn event(
        _state: &mut S,
        _proxy: &ZwpVirtualKeyboardManagerV1,
        _event: <ZwpVirtualKeyboardManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
    }
}

impl<S> Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboardData, S> for VirtualKeyboardDelegate
where
    S: Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboardData>,
{
    fn event(
        _state: &mut S,
        _proxy: &ZwpVirtualKeyboardV1,
        _event: <ZwpVirtualKeyboardV1 as wayland_client::Proxy>::Event,
        _data: &VirtualKeyboardData,
        _conn: &Connection,
        _qh: &QueueHandle<S>,
    ) {
    }
}

/// Binds the manager and creates a virtual keyboard on `seat`.
pub fn bind_virtual_keyboard<S>(
    globals: &GlobalList,
    qh: &QueueHandle<S>,
    seat: &WlSeat,
) -> Result<ZwpVirtualKeyboardV1, ClientError>
where
    S: Dispatch<ZwpVirtualKeyboardManagerV1, ()>
        + Dispatch<ZwpVirtualKeyboardV1, VirtualKeyboardData>
        + 'static,
{
    let manager = globals
        .bind::<ZwpVirtualKeyboardManagerV1, S, ()>(qh, 1..=1, ())
        .map_err(|_| ClientError::MissingGlobal("zwp_virtual_keyboard_manager_v1".to_owned()))?;
    Ok(manager.create_virtual_keyboard(seat, qh, VirtualKeyboardData))
}

/// Sends `keymap` (XKB text) to `keyboard` through a memfd, as the protocol wants it.
pub fn send_keymap(keyboard: &ZwpVirtualKeyboardV1, keymap: &str) -> Result<(), ClientError> {
    let fd: OwnedFd = memfd_create("keymap", MemfdFlags::CLOEXEC)
        .map_err(|error| ClientError::Dispatch(format!("memfd: {error}")))?;
    rustix::io::write(&fd, keymap.as_bytes())
        .map_err(|error| ClientError::Dispatch(format!("keymap write: {error}")))?;
    let size = u32::try_from(keymap.len())
        .map_err(|_| ClientError::Dispatch("keymap too large".to_owned()))?;
    keyboard.keymap(
        KeymapFormat::XkbV1.into(),
        std::os::fd::AsFd::as_fd(&fd),
        size,
    );
    Ok(())
}

/// What the board's side queue dispatches into: nothing it keeps.
#[derive(Debug)]
struct Quiet;

delegate_dispatch!(Quiet: [ZwpVirtualKeyboardManagerV1: ()] => VirtualKeyboardDelegate);
delegate_dispatch!(Quiet: [ZwpVirtualKeyboardV1: VirtualKeyboardData] => VirtualKeyboardDelegate);

impl Dispatch<WlSeat, ()> for Quiet {
    fn event(
        _state: &mut Quiet,
        _proxy: &WlSeat,
        _event: wayland_client::protocol::wl_seat::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Quiet>,
    ) {
    }
}

/// A virtual keyboard on a side queue of a client's connection.
#[derive(Debug)]
pub struct VirtualBoard {
    connection: Connection,
    _queue: EventQueue<Quiet>,
    keyboard: ZwpVirtualKeyboardV1,
}

impl VirtualBoard {
    /// Creates a virtual keyboard on the seat. The request is flushed.
    pub fn start(globals: &GlobalList) -> Result<VirtualBoard, ClientError> {
        let (connection, queue) = side_queue::<Quiet>(globals)?;
        let qh = queue.handle();
        let seat = globals
            .bind::<WlSeat, _, _>(&qh, 1..=1, ())
            .map_err(|_| ClientError::MissingGlobal("wl_seat".to_owned()))?;
        let keyboard = bind_virtual_keyboard(globals, &qh, &seat)?;
        let board = VirtualBoard {
            connection,
            _queue: queue,
            keyboard,
        };
        board.flush()?;
        Ok(board)
    }

    fn flush(&self) -> Result<(), ClientError> {
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }

    /// Sends `keymap` (XKB text).
    pub fn keymap(&self, keymap: &str) -> Result<(), ClientError> {
        send_keymap(&self.keyboard, keymap)?;
        self.flush()
    }

    /// Sends one key (`pressed`: `1`, released: `0`) with the client's own timestamp, which
    /// the compositor does not trust.
    pub fn key(&self, time: u32, key: u32, pressed: u32) -> Result<(), ClientError> {
        self.keyboard.key(time, key, pressed);
        self.flush()
    }
}
