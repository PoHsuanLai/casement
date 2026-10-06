//! The surfaces a session holds by name (toplevels, popups, layer surfaces, session locks) and
//! what it does with them: answer each configure by drawing at the size asked for, and say what
//! happened as `probe: ...` notes.

use crate::command::{Dpi, LayerCmd, PopupAsk, Size, ToplevelAsk};
use casement_core::geometry::PhysicalSize;
use casement_testclient::misc::viewporter::{Viewport, Viewporter};
use casement_testclient::surface::layer::{LayerAsk, LayerSurface};
use casement_testclient::surface::lock::{LockSurface, Locker};
use casement_testclient::surface::popup::{Popup, PopupSpec};
use casement_testclient::surface::subsurface::Surface;
use casement_testclient::surface::toplevel::{ResizeEdge, Toplevel, WmBase};
use casement_testclient::{Pattern, ShmBuffer, TestClient, shm_buffer};
use std::collections::BTreeMap;

/// The pattern of everything that is not a toplevel.
const CHROME: Pattern = Pattern::Solid(0xff30_3a50);

fn physical(size: Size) -> PhysicalSize {
    PhysicalSize {
        w: size.w,
        h: size.h,
    }
}

/// A toplevel and what it last drew.
struct Window {
    top: Toplevel,
    buffer: Option<ShmBuffer>,
    viewport: Option<Viewport>,
    /// The size drawn last.
    size: Size,
    /// The size asked for: what a configure that leaves the size to the client gets.
    natural: Size,
    pattern: Pattern,
    dpi: Dpi,
    handled: usize,
}

struct PopupWin {
    popup: Popup,
    buffer: Option<ShmBuffer>,
    size: Size,
    handled: usize,
    done: bool,
}

struct LayerWin {
    layer: LayerSurface,
    buffer: Option<ShmBuffer>,
    size: Size,
    closed: bool,
}

struct LockWin {
    locker: Option<Locker>,
    cover: LockSurface,
    buffer: Option<ShmBuffer>,
    locked: bool,
    finished: bool,
}

/// Every surface the session made, by the name the script gave it.
#[derive(Default)]
pub(crate) struct Surfaces {
    windows: BTreeMap<String, Window>,
    popups: BTreeMap<String, PopupWin>,
    layers: BTreeMap<String, LayerWin>,
    locks: BTreeMap<String, LockWin>,
    viewporter: Option<Viewporter>,
}

fn missing(kind: &str, name: &str) -> String {
    format!("no {kind} named {name}")
}

impl Window {
    /// Acks the newest configure and shows a buffer of `size` drawn for this window's scale.
    fn draw(&mut self, client: &mut TestClient, viewporter: &Viewporter, size: Size) {
        self.top.ack_latest();
        let buffer = shm_buffer(client, physical(self.dpi.buffer(size)), self.pattern);
        match self.dpi {
            Dpi::One => {}
            Dpi::Two => self.top.surface.set_buffer_scale(2),
            Dpi::OneAndAHalf => {
                let wrapped = Surface {
                    surface: self.top.surface.clone(),
                };
                let viewport = self
                    .viewport
                    .get_or_insert_with(|| viewporter.viewport(client, &wrapped));
                viewport.destination(
                    i32::try_from(size.w).unwrap_or(i32::MAX),
                    i32::try_from(size.h).unwrap_or(i32::MAX),
                );
            }
        }
        self.top.show(&buffer);
        self.buffer = Some(buffer);
        self.size = size;
    }
}

impl Surfaces {
    /// Makes a toplevel; it maps when its first configure has been answered.
    pub(crate) fn toplevel(
        &mut self,
        client: &mut TestClient,
        base: &WmBase,
        ask: &ToplevelAsk,
    ) -> Result<(), String> {
        if self.windows.contains_key(&ask.name) {
            return Err(format!("{} exists", ask.name));
        }
        let top = base.toplevel(client);
        top.set_app_id(&ask.app_id);
        top.set_title(&ask.name);
        if ask.maximised {
            top.set_maximized();
        }
        top.commit_bare();
        self.windows.insert(
            ask.name.clone(),
            Window {
                top,
                buffer: None,
                viewport: None,
                size: ask.size,
                natural: ask.size,
                pattern: ask.pattern,
                dpi: ask.dpi,
                handled: 0,
            },
        );
        Ok(())
    }

    fn window(&mut self, name: &str) -> Result<&mut Window, String> {
        self.windows
            .get_mut(name)
            .ok_or_else(|| missing("toplevel", name))
    }

    pub(crate) fn redraw(
        &mut self,
        client: &mut TestClient,
        name: &str,
        size: Option<Size>,
    ) -> Result<Vec<String>, String> {
        let viewporter = self
            .viewporter
            .get_or_insert_with(|| Viewporter::bind(client));
        let window = self
            .windows
            .get_mut(name)
            .ok_or_else(|| missing("toplevel", name))?;
        let size = size.unwrap_or(window.size);
        if size != window.size {
            window.natural = size;
        }
        window.draw(client, viewporter, size);
        Ok(vec![format!("drawn {name} {}x{}", size.w, size.h)])
    }

    pub(crate) fn maximise(&mut self, name: &str, on: bool) -> Result<(), String> {
        let window = self.window(name)?;
        if on {
            window.top.set_maximized();
        } else {
            window.top.toplevel.unset_maximized();
        }
        Ok(())
    }

    pub(crate) fn move_request(
        &mut self,
        name: &str,
        seat: &wayland_client::protocol::wl_seat::WlSeat,
        serial: u32,
    ) -> Result<(), String> {
        self.window(name)?.top.request_move(seat, serial);
        Ok(())
    }

    pub(crate) fn resize_request(
        &mut self,
        name: &str,
        seat: &wayland_client::protocol::wl_seat::WlSeat,
        serial: u32,
        edge: ResizeEdge,
    ) -> Result<(), String> {
        self.window(name)?.top.request_resize(seat, serial, edge);
        Ok(())
    }

    /// The `wl_surface` of toplevel `name`, for a drag to start from.
    pub(crate) fn surface_of(
        &self,
        name: &str,
    ) -> Result<wayland_client::protocol::wl_surface::WlSurface, String> {
        self.windows
            .get(name)
            .map(|window| window.top.surface.clone())
            .ok_or_else(|| missing("toplevel", name))
    }

    /// Makes a popup of toplevel `ask.parent`, grabbing with `serial` when asked.
    pub(crate) fn popup(
        &mut self,
        client: &mut TestClient,
        base: &WmBase,
        seat: &wayland_client::protocol::wl_seat::WlSeat,
        serial: Option<u32>,
        ask: &PopupAsk,
    ) -> Result<(), String> {
        let parent = &self
            .windows
            .get(&ask.parent)
            .ok_or_else(|| missing("toplevel", &ask.parent))?
            .top;
        let spec = PopupSpec {
            size: (
                i32::try_from(ask.size.w).unwrap_or(i32::MAX),
                i32::try_from(ask.size.h).unwrap_or(i32::MAX),
            ),
            ..PopupSpec::menu_at(ask.at)
        };
        let surface = client.create_surface().map_err(|e| e.to_string())?;
        let popup = Popup::of(client, base, parent, surface, spec);
        if ask.grab {
            let serial = serial.ok_or("popup grab needs a button press this client received")?;
            popup.grab(seat, serial);
        }
        popup.surface.commit();
        self.popups.insert(
            ask.name.clone(),
            PopupWin {
                popup,
                buffer: None,
                size: ask.size,
                handled: 0,
                done: false,
            },
        );
        Ok(())
    }

    pub(crate) fn popup_close(&mut self, name: &str) -> Result<(), String> {
        self.popups
            .remove(name)
            .map(|popup| popup.popup.destroy())
            .ok_or_else(|| missing("popup", name))
    }

    pub(crate) fn layer(&mut self, client: &mut TestClient, cmd: &LayerCmd) -> Result<(), String> {
        let ask = LayerAsk {
            level: cmd.level,
            namespace: Box::leak(cmd.name.clone().into_boxed_str()),
            anchor: cmd.anchor,
            exclusive_zone: 0,
            keyboard: cmd.keyboard,
            size: (cmd.size.w, cmd.size.h),
        };
        let layer = LayerSurface::create(client, &ask).map_err(|e| e.to_string())?;
        self.layers.insert(
            cmd.name.clone(),
            LayerWin {
                layer,
                buffer: None,
                size: cmd.size,
                closed: false,
            },
        );
        Ok(())
    }

    pub(crate) fn layer_close(&mut self, client: &TestClient, name: &str) -> Result<(), String> {
        self.layers
            .remove(name)
            .map(|layer| layer.layer.destroy(client))
            .ok_or_else(|| missing("layer", name))
    }

    /// Asks for a session lock and covers the first output.
    pub(crate) fn lock(&mut self, client: &TestClient, name: &str) -> Result<(), String> {
        let locker = Locker::request(client).map_err(|e| e.to_string())?;
        let cover = locker.cover(client).map_err(|e| e.to_string())?;
        self.locks.insert(
            name.to_owned(),
            LockWin {
                locker: Some(locker),
                cover,
                buffer: None,
                locked: false,
                finished: false,
            },
        );
        Ok(())
    }

    pub(crate) fn unlock(&mut self, client: &TestClient, name: &str) -> Result<(), String> {
        let lock = self
            .locks
            .remove(name)
            .ok_or_else(|| missing("lock", name))?;
        lock.locker
            .ok_or_else(|| format!("{name} is unlocked already"))?
            .unlock(client);
        Ok(())
    }

    /// Destroys toplevel `name`.
    pub(crate) fn close(&mut self, name: &str) -> Result<(), String> {
        self.windows
            .remove(name)
            .map(|window| window.top.destroy())
            .ok_or_else(|| missing("toplevel", name))
    }

    /// Answers whatever the compositor sent since the last call: draws for each new configure,
    /// notes each change. The notes are printed in order.
    pub(crate) fn follow(&mut self, client: &mut TestClient) -> Vec<String> {
        let viewporter = self
            .viewporter
            .get_or_insert_with(|| Viewporter::bind(client));
        let mut notes = Vec::new();
        for (name, window) in &mut self.windows {
            let configures = window.top.configures();
            let Some(newest) = configures
                .last()
                .filter(|_| configures.len() > window.handled)
            else {
                continue;
            };
            let first = window.handled == 0;
            window.handled = configures.len();
            let (w, h) = newest.size;
            let size = match (u32::try_from(w), u32::try_from(h)) {
                (Ok(w @ 1..), Ok(h @ 1..)) => Size { w, h },
                _ => window.natural,
            };
            let states: Vec<String> = newest
                .states
                .iter()
                .map(|state| format!("{state:?}").to_lowercase())
                .collect();
            notes.push(format!("configure {name} {w}x{h} [{}]", states.join(" ")));
            window.draw(client, viewporter, size);
            let verb = if first { "mapped" } else { "drawn" };
            notes.push(format!("{verb} {name} {}x{}", size.w, size.h));
        }
        for (name, popup) in &mut self.popups {
            let log = popup.popup.log.with(|seen| (seen.serials.len(), seen.done));
            if log.1 > 0 && !popup.done {
                popup.done = true;
                notes.push(format!("popup-done {name}"));
            }
            if log.0 > popup.handled {
                let first = popup.handled == 0;
                popup.handled = log.0;
                popup.popup.ack_latest();
                let buffer = shm_buffer(client, physical(popup.size), CHROME);
                popup.popup.show(&buffer);
                popup.buffer = Some(buffer);
                if let Some((x, y, w, h)) = popup.popup.geometry().filter(|_| first) {
                    notes.push(format!("popup-mapped {name} {x},{y} {w}x{h}"));
                }
            }
        }
        for (name, layer) in &mut self.layers {
            if layer.closed {
                continue;
            }
            if layer.layer.is_closed() {
                layer.closed = true;
                notes.push(format!("layer-closed {name}"));
            } else if let (Some(configure), None) = (layer.layer.configure(), &layer.buffer) {
                layer.layer.ack(client);
                let size = Size {
                    w: configure.width.max(layer.size.w),
                    h: configure.height.max(layer.size.h),
                };
                let buffer = shm_buffer(client, physical(size), CHROME);
                layer.layer.show(client, &buffer);
                layer.buffer = Some(buffer);
                notes.push(format!(
                    "layer-mapped {name} {}x{} configure {}x{}",
                    size.w, size.h, configure.width, configure.height
                ));
            }
        }
        for (name, lock) in &mut self.locks {
            if let (Some((_, w, h)), None) = (lock.cover.configure(), &lock.buffer) {
                lock.cover.ack(client);
                let size = Size { w, h };
                let buffer = shm_buffer(client, physical(size), Pattern::Solid(0xff10_1030));
                lock.cover.show(client, &buffer);
                lock.buffer = Some(buffer);
                notes.push(format!("lock-surface {name} {w}x{h}"));
            }
            let (locked, finished) = lock
                .locker
                .as_ref()
                .map_or((false, false), |l| (l.is_locked(), l.is_finished()));
            if locked && !lock.locked {
                notes.push(format!("locked {name}"));
            }
            if finished && !lock.finished {
                notes.push(format!("lock-finished {name}"));
            }
            lock.locked = locked;
            lock.finished = finished;
        }
        notes
    }
}
