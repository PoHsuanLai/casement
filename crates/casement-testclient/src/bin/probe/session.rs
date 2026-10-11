//! `casement-probe session`: one long-lived test client driven from stdin. Every event the
//! compositor sends is printed as `< object#id.event(args)` (the client's transcript), and what
//! the session itself did as `probe: ...` notes, so a script waits on lines instead of sleeping.
//! The command language is in `command.rs`.

use crate::Failure;
use crate::clip::Clip;
use crate::command::{Command, parse};
use crate::shellrole::{self, Shell};
use crate::surfaces::Surfaces;
use casement_core::time::Mono;
use casement_testclient::ime::session::ImeSession;
use casement_testclient::seat::keyboard::Keyboard;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::{SyncDone, TestClient};
use rustix::event::{PollFd, PollFlags, Timespec};
use std::os::fd::AsFd;
use std::time::Duration;

/// How long one wait for the compositor or stdin lasts.
const TURN: Timespec = Timespec {
    tv_sec: 0,
    tv_nsec: 50_000_000,
};

struct Session {
    client: TestClient,
    base: WmBase,
    _keyboard: Keyboard,
    pointer: Pointer,
    surfaces: Surfaces,
    clip: Clip,
    ime: Option<ImeSession>,
    shell: Option<Shell>,
    turns: u32,
    printed: usize,
    syncs: Vec<SyncDone>,
    synced: u32,
    commands: u32,
}

fn note(text: &str) {
    println!("probe: {text}");
}

impl Session {
    fn start() -> Result<Session, Failure> {
        Session::over(TestClient::connect_env()?)
    }

    /// A session on `client`, which has had its first roundtrip.
    fn over(mut client: TestClient) -> Result<Session, Failure> {
        let base = WmBase::bind(&mut client, 6);
        let keyboard = Keyboard::bind(&client)?;
        let pointer = Pointer::bind(&client)?;
        let clip = Clip::bind(&client, pointer.seat()).map_err(Failure::Probe)?;
        Ok(Session {
            client,
            base,
            _keyboard: keyboard,
            pointer,
            surfaces: Surfaces::default(),
            clip,
            ime: None,
            shell: None,
            turns: 0,
            printed: 0,
            syncs: Vec::new(),
            synced: 0,
            commands: 0,
        })
    }

    /// The stamp the input method's events get: the turn they were read in (the probe reads no
    /// clock; its transcript is read by order, not by time).
    fn now(&self) -> Mono {
        Mono(Duration::from_millis(u64::from(self.turns)))
    }

    fn ime(&mut self) -> Result<&mut ImeSession, String> {
        self.ime
            .as_mut()
            .ok_or_else(|| "run ime-start first".to_owned())
    }

    fn shell(&mut self) -> Result<&mut Shell, String> {
        self.shell
            .as_mut()
            .ok_or_else(|| "run shell-connect first".to_owned())
    }

    /// Makes the session the shell: a new connection from `Compositor1.Connect("shell")`
    /// replaces the client, and every surface made before it goes with the old one.
    fn shell_connect(&mut self) -> Result<serde_json::Value, String> {
        let (client, shell) = shellrole::connect()?;
        let mut next = Session::over(client).map_err(|why| match why {
            Failure::Probe(text) => text,
            _ => "the shell connection could not be used".to_owned(),
        })?;
        next.commands = self.commands;
        next.shell = Some(shell);
        *self = next;
        Ok(serde_json::json!({ "shell": "connected" }))
    }

    /// The JSON note of a shell verb: its answer, or `{"error": ...}`.
    fn shell_note(answer: Result<serde_json::Value, String>) -> Vec<String> {
        let json = answer.unwrap_or_else(|why| serde_json::json!({ "error": why }));
        vec![json.to_string()]
    }

    /// Runs one command; the notes it produced.
    fn exec(&mut self, command: Command) -> Result<Vec<String>, String> {
        let press = self.pointer.press_serial();
        match command {
            Command::Toplevel(ask) => self.surfaces.toplevel(&mut self.client, &self.base, &ask)?,
            Command::Redraw(name, size) => {
                return self.surfaces.redraw(&mut self.client, &name, size);
            }
            Command::Maximise(name) => self.surfaces.maximise(&name, true)?,
            Command::Minimise(name) => self.surfaces.minimise(&name)?,
            Command::Unmaximise(name) => self.surfaces.maximise(&name, false)?,
            Command::MoveRequest(name) => {
                let serial = press.ok_or("no button press has reached this client")?;
                self.surfaces
                    .move_request(&name, self.pointer.seat(), serial)?;
            }
            Command::ResizeRequest(name, edge) => {
                let serial = press.ok_or("no button press has reached this client")?;
                self.surfaces
                    .resize_request(&name, self.pointer.seat(), serial, edge)?;
            }
            Command::Popup(ask) => {
                self.surfaces.popup(
                    &mut self.client,
                    &self.base,
                    self.pointer.seat(),
                    press,
                    &ask,
                )?;
            }
            Command::PopupClose(name) => self.surfaces.popup_close(&name)?,
            Command::Layer(cmd) => self.surfaces.layer(&mut self.client, &cmd)?,
            Command::LayerClose(name) => self.surfaces.layer_close(&self.client, &name)?,
            Command::Lock(name) => self.surfaces.lock(&self.client, &name)?,
            Command::Unlock(name) => self.surfaces.unlock(&self.client, &name)?,
            Command::Close(name) => self.surfaces.close(&name)?,
            Command::Copy(text) => self.clip.copy(&self.client, &text, press.unwrap_or(0)),
            Command::Paste => self.clip.paste(&self.client)?,
            Command::Drag(name, text) => {
                let origin = self.surfaces.surface_of(&name)?;
                let serial = press.ok_or("no button press has reached this client")?;
                self.clip.drag(&self.client, &origin, &text, serial);
            }
            Command::ImeStart => {
                let ime = ImeSession::start(self.client.globals()).map_err(|e| e.to_string())?;
                self.ime = Some(ime);
            }
            Command::ImeEnable => self.ime()?.enable().map_err(|e| e.to_string())?,
            Command::ImeGrab => self.ime()?.grab_keyboard().map_err(|e| e.to_string())?,
            Command::ImePreedit(text) => {
                let ime = self.ime()?;
                ime.set_preedit(&text, 0, 0).map_err(|e| e.to_string())?;
                ime.commit_batch().map_err(|e| e.to_string())?;
            }
            Command::ImeCommit(text) => {
                let ime = self.ime()?;
                ime.commit_string(&text).map_err(|e| e.to_string())?;
                ime.commit_batch().map_err(|e| e.to_string())?;
            }
            Command::ShellConnect => return Ok(Session::shell_note(self.shell_connect())),
            Command::ShellRole(role, layer) => {
                let answer = self.surfaces.layer_surface(&layer).and_then(|surface| {
                    self.shell
                        .as_mut()
                        .ok_or_else(|| "run shell-connect first".to_owned())
                        .map(|shell| shell.role(role, &surface))
                });
                return Ok(Session::shell_note(answer));
            }
            Command::MinimiseTarget(toplevel, at, size) => {
                let answer = self
                    .shell()
                    .and_then(|shell| shell.target(&toplevel, at, (size.w, size.h)));
                return Ok(Session::shell_note(answer));
            }
            Command::MinimiseClear(toplevel) => {
                let answer = self.shell().map(|shell| shell.clear(&toplevel));
                return Ok(Session::shell_note(answer));
            }
            Command::Sync => self
                .syncs
                .push(self.client.sync().map_err(|e| e.to_string())?),
            Command::Quit => return Err("quit".to_owned()),
        }
        Ok(Vec::new())
    }

    fn command(&mut self, line: &str) -> bool {
        self.commands += 1;
        match parse(line).and_then(|command| self.exec(command)) {
            Ok(notes) => notes.iter().for_each(|text| note(text)),
            Err(why) if why == "quit" => return false,
            Err(why) => note(&format!("error {line}: {why}")),
        }
        note(&format!("done {}", self.commands));
        true
    }

    /// Reads the compositor, answers it, and prints what is new.
    fn turn(&mut self) {
        self.turns += 1;
        self.client.pump();
        let now = self.now();
        if let Some(ime) = &mut self.ime {
            let _ = ime.pump(now);
        }
        let mut notes = self.surfaces.follow(&mut self.client);
        notes.extend(self.clip.poll(&self.client));
        self.client.pump();
        let lines = &self.client.transcript().0;
        for line in &lines[self.printed.min(lines.len())..] {
            println!("< {}.{}({})", line.object, line.event, line.args.join(", "));
        }
        self.printed = lines.len();
        notes.iter().for_each(|text| note(text));
        let (done, waiting): (Vec<SyncDone>, Vec<SyncDone>) = std::mem::take(&mut self.syncs)
            .into_iter()
            .partition(SyncDone::is_done);
        self.syncs = waiting;
        for _ in done {
            self.synced += 1;
            note(&format!("synced {}", self.synced));
        }
    }
}

/// Runs the session until `quit` or the compositor goes away.
pub(crate) fn run() -> Result<(), Failure> {
    let mut session = Session::start()?;
    note("ready");
    let stdin = std::io::stdin();
    let mut input = Vec::<u8>::new();
    let mut open = true;
    loop {
        let wire = session.client.wait_fd();
        let mut fds = vec![PollFd::new(&wire, PollFlags::IN)];
        if open {
            fds.push(PollFd::new(&stdin, PollFlags::IN));
        }
        rustix::event::poll(&mut fds, Some(&TURN)).map_err(|e| Failure::Probe(e.to_string()))?;
        let readable = open && fds.get(1).is_some_and(|fd| !fd.revents().is_empty());
        drop(fds);
        if readable {
            let mut chunk = [0_u8; 4096];
            match rustix::io::read(stdin.as_fd(), &mut chunk) {
                Ok(0) | Err(_) => open = false,
                Ok(n) => input.extend(&chunk[..n]),
            }
        }
        while let Some(end) = input.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = input.drain(..=end).collect();
            let text = String::from_utf8_lossy(&line);
            if !text.trim().is_empty() && !text.trim().starts_with('#') {
                session.turn();
                if !session.command(text.trim()) {
                    session.turn();
                    return Ok(());
                }
            }
        }
        session.turn();
        if !session.client.is_connected() {
            note("disconnected");
            return Ok(());
        }
    }
}
