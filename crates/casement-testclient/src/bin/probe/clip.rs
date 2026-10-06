//! Selection and drag-and-drop with real data: a source that writes its text when asked, a
//! device that accepts every drag and reads what it is offered. (The library's data-device
//! helper only logs; the transcript cannot show what the pipe carried.)

use casement_testclient::{ClientState, TestClient};
use rustix::io::Errno;
use rustix::pipe::{PipeFlags, pipe_with};
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsFd, OwnedFd};
use std::sync::{Arc, Mutex, PoisonError};
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::{DndAction, WlDataDeviceManager};
use wayland_client::protocol::wl_data_offer::{self, WlDataOffer};
use wayland_client::protocol::wl_data_source::{self, WlDataSource};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, event_created_child};

/// The one type this probe offers and accepts.
const MIME: &str = "text/plain";

/// The offers a device was sent.
#[derive(Default)]
struct Offers {
    selection: Option<WlDataOffer>,
    drag: Option<WlDataOffer>,
    /// The offer a `drop` arrived for (`leave` follows it at once and clears `drag`).
    dropped: Option<WlDataOffer>,
}

/// The user data of the device: where its offers are kept.
#[derive(Clone, Default)]
struct DeviceData(Arc<Mutex<Offers>>);

impl DeviceData {
    fn with<R>(&self, f: impl FnOnce(&mut Offers) -> R) -> R {
        f(&mut self.0.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// The user data of the manager and of offers.
#[derive(Clone, Copy, Default)]
struct Plain;

/// The user data of a source: the text it sends.
struct SourceText(String);

/// What a pending read is for.
enum Reading {
    Paste,
    Drop(WlDataOffer),
}

struct Pending {
    fd: OwnedFd,
    text: Vec<u8>,
    what: Reading,
}

/// A data device and the reads in flight.
pub(crate) struct Clip {
    manager: WlDataDeviceManager,
    device: WlDataDevice,
    data: DeviceData,
    pending: Vec<Pending>,
}

impl Clip {
    pub(crate) fn bind(client: &TestClient, seat: &WlSeat) -> Result<Clip, String> {
        let manager: WlDataDeviceManager = client.bind(1..=3, Plain).map_err(|e| e.to_string())?;
        let data = DeviceData::default();
        let device = manager.get_data_device(seat, &client.qh(), data.clone());
        Ok(Clip {
            manager,
            device,
            data,
            pending: Vec::new(),
        })
    }

    fn source(&self, client: &TestClient, text: &str) -> WlDataSource {
        let source = self
            .manager
            .create_data_source(&client.qh(), SourceText(text.to_owned()));
        source.offer(MIME.to_owned());
        source
    }

    /// Sets the clipboard to `text`.
    pub(crate) fn copy(&self, client: &TestClient, text: &str, serial: u32) {
        self.device
            .set_selection(Some(&self.source(client, text)), serial);
        let _ = client.connection().flush();
    }

    /// Starts a drag of `text` from `origin`.
    pub(crate) fn drag(&self, client: &TestClient, origin: &WlSurface, text: &str, serial: u32) {
        let source = self.source(client, text);
        source.set_actions(DndAction::Copy);
        self.device.start_drag(Some(&source), origin, None, serial);
        let _ = client.connection().flush();
    }

    /// Reads the clipboard offer; the text comes back through [`Clip::poll`].
    pub(crate) fn paste(&mut self, client: &TestClient) -> Result<(), String> {
        let offer = self
            .data
            .with(|offers| offers.selection.clone())
            .ok_or("there is no selection to paste")?;
        self.receive(client, &offer, Reading::Paste)
    }

    fn receive(
        &mut self,
        client: &TestClient,
        offer: &WlDataOffer,
        what: Reading,
    ) -> Result<(), String> {
        let (read, write) =
            pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK).map_err(|e| e.to_string())?;
        offer.receive(MIME.to_owned(), write.as_fd());
        client.connection().flush().map_err(|e| e.to_string())?;
        self.pending.push(Pending {
            fd: read,
            text: Vec::new(),
            what,
        });
        Ok(())
    }

    /// Starts the read a drop asks for and collects the text of finished reads, as notes.
    pub(crate) fn poll(&mut self, client: &TestClient) -> Vec<String> {
        let dropped = self.data.with(|offers| offers.dropped.take());
        if let Some(offer) = dropped {
            let _ = self.receive(client, &offer, Reading::Drop(offer.clone()));
        }
        let mut notes = Vec::new();
        let mut keep = Vec::new();
        for mut read in std::mem::take(&mut self.pending) {
            let mut chunk = [0_u8; 4096];
            let finished = loop {
                match rustix::io::read(&read.fd, &mut chunk) {
                    Ok(0) => break true,
                    Ok(n) => read.text.extend(&chunk[..n]),
                    Err(Errno::AGAIN) => break false,
                    Err(_) => break true,
                }
            };
            if !finished {
                keep.push(read);
                continue;
            }
            let text = String::from_utf8_lossy(&read.text);
            match read.what {
                Reading::Paste => notes.push(format!("pasted {text:?}")),
                Reading::Drop(offer) => {
                    notes.push(format!("dropped {text:?}"));
                    offer.finish();
                    let _ = client.connection().flush();
                }
            }
        }
        self.pending = keep;
        notes
    }
}

impl Dispatch<WlDataDeviceManager, Plain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataDeviceManager,
        _: <WlDataDeviceManager as wayland_client::Proxy>::Event,
        _: &Plain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlDataOffer, Plain> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataOffer,
        _: wl_data_offer::Event,
        _: &Plain,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlDataSource, SourceText> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataSource,
        event: wl_data_source::Event,
        sends: &SourceText,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_data_source::Event::Send { fd, .. } = event {
            let _ = File::from(fd).write_all(sends.0.as_bytes());
        }
    }
}

impl Dispatch<WlDataDevice, DeviceData> for ClientState {
    fn event(
        _: &mut Self,
        _: &WlDataDevice,
        event: wl_data_device::Event,
        data: &DeviceData,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_data_device::Event::Enter {
                serial,
                id: Some(offer),
                ..
            } => {
                offer.accept(serial, Some(MIME.to_owned()));
                offer.set_actions(DndAction::Copy, DndAction::Copy);
                data.with(|offers| offers.drag = Some(offer));
            }
            wl_data_device::Event::Leave => data.with(|offers| offers.drag = None),
            wl_data_device::Event::Drop => data.with(|offers| offers.dropped = offers.drag.clone()),
            wl_data_device::Event::Selection { id } => data.with(|offers| offers.selection = id),
            _ => {}
        }
    }

    event_created_child!(ClientState, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, Plain),
    ]);
}
