//! `casement-x11probe session`: the X11 connection, the windows by name, the clipboard, and the
//! loop that interleaves stdin commands with X events. Nothing blocks on the X server except
//! the small round trips a command needs; a paste is answered by the `SelectionNotify` event.

use crate::command::{Command, Kind, MapAsk, parse};
use std::collections::BTreeMap;
use std::io::BufRead;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;
use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::CURRENT_TIME;
use x11rb::connection::Connection;
use x11rb::errors::{ConnectError, ConnectionError, ReplyError, ReplyOrIdError};
use x11rb::protocol::Event;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ClientMessageEvent, ConnectionExt as _, CreateWindowAux, EventMask, InputFocus,
    PropMode, SELECTION_NOTIFY_EVENT, SelectionNotifyEvent, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

/// How long the loop sleeps when neither stdin nor the server had anything.
const IDLE: Duration = Duration::from_millis(10);

/// The longest clipboard text read, in 32-bit units.
const PASTE_UNITS: u32 = 1 << 18;

/// Why the probe stopped.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ProbeError {
    #[error("{0}")]
    Connect(#[from] ConnectError),
    #[error("connection: {0}")]
    Connection(#[from] ConnectionError),
    #[error("reply: {0}")]
    Reply(#[from] ReplyError),
    #[error("id: {0}")]
    Id(#[from] ReplyOrIdError),
}

/// The atoms the probe uses.
struct Atoms {
    clipboard: Atom,
    utf8: Atom,
    targets: Atom,
    active_window: Atom,
    protocols: Atom,
    delete_window: Atom,
    paste_into: Atom,
}

struct Session {
    conn: RustConnection,
    root: Window,
    white: u32,
    atoms: Atoms,
    windows: BTreeMap<String, Window>,
    clip_window: Window,
    clipboard: Option<String>,
    commands: u32,
    synced: u32,
}

fn note(text: &str) {
    println!("x11: {text}");
}

fn atom(conn: &RustConnection, name: &[u8]) -> Result<Atom, ProbeError> {
    Ok(conn.intern_atom(false, name)?.reply()?.atom)
}

/// The events every window selects.
fn window_events() -> EventMask {
    EventMask::KEY_PRESS
        | EventMask::KEY_RELEASE
        | EventMask::BUTTON_PRESS
        | EventMask::BUTTON_RELEASE
        | EventMask::POINTER_MOTION
        | EventMask::ENTER_WINDOW
        | EventMask::LEAVE_WINDOW
        | EventMask::FOCUS_CHANGE
        | EventMask::EXPOSURE
        | EventMask::STRUCTURE_NOTIFY
        | EventMask::PROPERTY_CHANGE
}

/// Connects to `$DISPLAY` and serves stdin commands until `quit` or the end of stdin.
pub(crate) fn run() -> Result<(), ProbeError> {
    let (conn, screen_number) = RustConnection::connect(None)?;
    let screen = &conn.setup().roots[screen_number];
    let (root, white) = (screen.root, screen.white_pixel);
    let atoms = Atoms {
        clipboard: atom(&conn, b"CLIPBOARD")?,
        utf8: atom(&conn, b"UTF8_STRING")?,
        targets: atom(&conn, b"TARGETS")?,
        active_window: atom(&conn, b"_NET_ACTIVE_WINDOW")?,
        protocols: atom(&conn, b"WM_PROTOCOLS")?,
        delete_window: atom(&conn, b"WM_DELETE_WINDOW")?,
        paste_into: atom(&conn, b"CASEMENT_PROBE_PASTE")?,
    };
    // A window that never maps: the clipboard's owner and the paste's requestor.
    let clip_window = conn.generate_id()?;
    conn.create_window(
        COPY_DEPTH_FROM_PARENT,
        clip_window,
        root,
        0,
        0,
        1,
        1,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )?;
    conn.flush()?;
    let mut session = Session {
        conn,
        root,
        white,
        atoms,
        windows: BTreeMap::new(),
        clip_window,
        clipboard: None,
        commands: 0,
        synced: 0,
    };
    note("ready");
    session.serve(&stdin_lines())
}

/// Stdin's lines, read on a thread so the loop never blocks on them.
fn stdin_lines() -> Receiver<String> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    receive
}

impl Session {
    fn serve(&mut self, lines: &Receiver<String>) -> Result<(), ProbeError> {
        loop {
            let mut busy = false;
            match lines.try_recv() {
                Ok(line) if line.trim().is_empty() => busy = true,
                Ok(line) => {
                    busy = true;
                    if self.command(&line)? == Flow::Stop {
                        return Ok(());
                    }
                }
                Err(TryRecvError::Disconnected) => return Ok(()),
                Err(TryRecvError::Empty) => {}
            }
            self.conn.flush()?;
            while let Some(event) = self.conn.poll_for_event()? {
                busy = true;
                self.event(&event)?;
            }
            if !busy {
                std::thread::sleep(IDLE);
            }
        }
    }

    /// Runs one line; every line, good or bad, counts one `done`.
    fn command(&mut self, line: &str) -> Result<Flow, ProbeError> {
        let flow = match parse(line) {
            Ok(Command::Quit) => Flow::Stop,
            Ok(command) => {
                if let Err(why) = self.run_command(command) {
                    note(&format!("error {why}"));
                }
                Flow::Go
            }
            Err(why) => {
                note(&format!("error {why}"));
                Flow::Go
            }
        };
        self.conn.flush()?;
        self.commands += 1;
        note(&format!("done {}", self.commands));
        Ok(flow)
    }

    fn run_command(&mut self, command: Command) -> Result<(), ProbeError> {
        match command {
            Command::Map(ask) => self.map(&ask),
            Command::Unmap(name) => self.with_window(&name, |s, w| {
                s.conn.unmap_window(w)?;
                Ok(())
            }),
            Command::Close(name) => self.with_window(&name, |s, w| {
                s.conn.destroy_window(w)?;
                Ok(())
            }),
            Command::Focus(name) => self.with_window(&name, |s, w| {
                s.conn
                    .set_input_focus(InputFocus::PARENT, w, CURRENT_TIME)?;
                Ok(())
            }),
            Command::Activate(name) => self.with_window(&name, |s, w| s.activate(w)),
            Command::Info(name) => self.with_window(&name, |s, w| s.info(&name, w)),
            Command::OwnClipboard(text) => {
                self.conn.set_selection_owner(
                    self.clip_window,
                    self.atoms.clipboard,
                    CURRENT_TIME,
                )?;
                self.clipboard = Some(text);
                note("clipboard-owned");
                Ok(())
            }
            Command::Paste => {
                self.conn.convert_selection(
                    self.clip_window,
                    self.atoms.clipboard,
                    self.atoms.utf8,
                    self.atoms.paste_into,
                    CURRENT_TIME,
                )?;
                Ok(())
            }
            Command::Sync => {
                self.conn.get_input_focus()?.reply()?;
                self.synced += 1;
                note(&format!("synced {}", self.synced));
                Ok(())
            }
            Command::Quit => Ok(()),
        }
    }

    fn with_window(
        &mut self,
        name: &str,
        action: impl FnOnce(&mut Session, Window) -> Result<(), ProbeError>,
    ) -> Result<(), ProbeError> {
        match self.windows.get(name).copied() {
            Some(window) => action(self, window),
            None => {
                note(&format!("error no window named {name}"));
                Ok(())
            }
        }
    }

    fn map(&mut self, ask: &MapAsk) -> Result<(), ProbeError> {
        let window = self.conn.generate_id()?;
        let redirect = u32::from(ask.kind == Kind::OverrideRedirect);
        let aux = CreateWindowAux::new()
            .background_pixel(self.white)
            .override_redirect(redirect)
            .event_mask(window_events());
        self.conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            self.root,
            ask.at.0,
            ask.at.1,
            ask.size.0,
            ask.size.1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &aux,
        )?;
        let class = format!("{}\0{}\0", ask.instance, ask.class);
        self.conn.change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            class.as_bytes(),
        )?;
        self.conn.change_property32(
            PropMode::REPLACE,
            window,
            self.atoms.protocols,
            AtomEnum::ATOM,
            &[self.atoms.delete_window],
        )?;
        self.windows.insert(ask.name.clone(), window);
        self.conn.map_window(window)?;
        self.conn.flush()?;
        Ok(())
    }

    /// The client message a toolkit sends to ask the window manager to activate `window`.
    fn activate(&self, window: Window) -> Result<(), ProbeError> {
        let message =
            ClientMessageEvent::new(32, window, self.atoms.active_window, [1, 0, 0, 0, 0]);
        let mask = EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY;
        self.conn.send_event(false, self.root, mask, message)?;
        Ok(())
    }

    /// Geometry, override-redirect, the screen and the resource database, as one line each.
    fn info(&self, name: &str, window: Window) -> Result<(), ProbeError> {
        let geometry = self.conn.get_geometry(window)?.reply()?;
        let origin = self
            .conn
            .translate_coordinates(window, self.root, 0, 0)?
            .reply()?;
        let attributes = self.conn.get_window_attributes(window)?.reply()?;
        note(&format!(
            "info {name} size={}x{} root-at={},{} override-redirect={} map-state={:?}",
            geometry.width,
            geometry.height,
            origin.dst_x,
            origin.dst_y,
            attributes.override_redirect,
            attributes.map_state
        ));
        let screen = &self.conn.setup().roots[0];
        note(&format!(
            "screen {}x{} px {}x{} mm",
            screen.width_in_pixels,
            screen.height_in_pixels,
            screen.width_in_millimeters,
            screen.height_in_millimeters
        ));
        let resources = self
            .conn
            .get_property(
                false,
                self.root,
                AtomEnum::RESOURCE_MANAGER,
                AtomEnum::STRING,
                0,
                1 << 14,
            )?
            .reply()?;
        let text = String::from_utf8_lossy(&resources.value);
        let dpi = text
            .lines()
            .find_map(|line| line.strip_prefix("Xft.dpi:"))
            .map_or("none", str::trim);
        note(&format!("xft-dpi {dpi}"));
        Ok(())
    }

    /// The name of `window`, or its number for a window that is not one of ours.
    fn name_of(&self, window: Window) -> String {
        self.windows
            .iter()
            .find(|(_, id)| **id == window)
            .map_or_else(|| format!("#{window}"), |(name, _)| name.clone())
    }

    fn event(&mut self, event: &Event) -> Result<(), ProbeError> {
        match event {
            Event::MapNotify(e) => self.report(e.window, "mapped", String::new()),
            Event::UnmapNotify(e) => self.report(e.window, "unmapped", String::new()),
            Event::DestroyNotify(e) => self.report(e.window, "destroyed", String::new()),
            Event::ConfigureNotify(e) => self.report(
                e.window,
                "configure",
                format!("{}x{} at {},{}", e.width, e.height, e.x, e.y),
            ),
            Event::Expose(e) => self.report(e.window, "expose", format!("count={}", e.count)),
            Event::FocusIn(e) => self.report(e.event, "focus-in", format!("mode={:?}", e.mode)),
            Event::FocusOut(e) => self.report(e.event, "focus-out", format!("mode={:?}", e.mode)),
            Event::EnterNotify(e) => {
                self.report(e.event, "enter", format!("at {},{}", e.event_x, e.event_y))
            }
            Event::LeaveNotify(e) => self.report(e.event, "leave", String::new()),
            Event::MotionNotify(e) => {
                self.report(e.event, "motion", format!("at {},{}", e.event_x, e.event_y))
            }
            Event::ButtonPress(e) => self.report(
                e.event,
                "button-press",
                format!("detail={} at {},{}", e.detail, e.event_x, e.event_y),
            ),
            Event::ButtonRelease(e) => {
                self.report(e.event, "button-release", format!("detail={}", e.detail))
            }
            Event::KeyPress(e) => self.report(e.event, "key-press", format!("detail={}", e.detail)),
            Event::KeyRelease(e) => {
                self.report(e.event, "key-release", format!("detail={}", e.detail))
            }
            Event::ClientMessage(e) if e.type_ == self.atoms.protocols => {
                if e.data.as_data32()[0] == self.atoms.delete_window {
                    self.report(e.window, "close-requested", String::new());
                }
            }
            Event::SelectionRequest(e) => self.answer_request(e)?,
            Event::SelectionNotify(e) => self.pasted(e.property)?,
            Event::SelectionClear(_) => {
                self.clipboard = None;
                note("clipboard-lost");
            }
            Event::Error(error) => note(&format!("x-error {error:?}")),
            _ => {}
        }
        Ok(())
    }

    fn report(&self, window: Window, kind: &str, detail: String) {
        note(format!("event {} {kind} {detail}", self.name_of(window)).trim_end());
    }

    /// Another client pastes from us: the text, or the list of targets we serve.
    fn answer_request(
        &self,
        request: &x11rb::protocol::xproto::SelectionRequestEvent,
    ) -> Result<(), ProbeError> {
        let supported = [self.atoms.utf8, AtomEnum::STRING.into()];
        let served = match &self.clipboard {
            Some(text) if request.target == self.atoms.targets => {
                let mut targets = vec![self.atoms.targets];
                targets.extend(supported);
                self.conn.change_property32(
                    PropMode::REPLACE,
                    request.requestor,
                    request.property,
                    AtomEnum::ATOM,
                    &targets,
                )?;
                Some(text)
            }
            Some(text) if supported.contains(&request.target) => {
                self.conn.change_property8(
                    PropMode::REPLACE,
                    request.requestor,
                    request.property,
                    request.target,
                    text.as_bytes(),
                )?;
                Some(text)
            }
            _ => None,
        };
        let property = if served.is_some() {
            request.property
        } else {
            Atom::from(AtomEnum::NONE)
        };
        let reply = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time: request.time,
            requestor: request.requestor,
            selection: request.selection,
            target: request.target,
            property,
        };
        self.conn
            .send_event(false, request.requestor, EventMask::NO_EVENT, reply)?;
        self.conn.flush()?;
        note(&format!(
            "clipboard-request served={}",
            if served.is_some() { "yes" } else { "no" }
        ));
        Ok(())
    }

    /// Our paste was answered: print the text, or that the owner refused.
    fn pasted(&self, property: Atom) -> Result<(), ProbeError> {
        if property == Atom::from(AtomEnum::NONE) {
            note("clipboard-text refused");
            return Ok(());
        }
        let reply = self
            .conn
            .get_property(
                true,
                self.clip_window,
                property,
                AtomEnum::ANY,
                0,
                PASTE_UNITS,
            )?
            .reply()?;
        note(&format!(
            "clipboard-text {}",
            String::from_utf8_lossy(&reply.value)
        ));
        Ok(())
    }
}

/// Whether the loop goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Go,
    Stop,
}
