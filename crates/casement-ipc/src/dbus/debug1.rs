//! `org.quire.Casement.Debug1` at `/org/quire/Casement/Debug1` (feature `debug-inject`, the
//! jail's private bus only): injection, waits, the stepped clock, virtual outputs, model and
//! scene dumps and fault injection for `casementctl debug`.

use crate::inject::{InjectLine, parse_inject};
use crate::shot::{ShotArea, parse_shot};
use crate::wait::{WaitPredicate, parse_wait};
use casement_backend::VirtualOutputSpec;
use casement_core::OutputName;
use std::time::Duration;
use zbus::fdo;

/// The object's path.
pub const OBJECT_PATH: &str = "/org/quire/Casement/Debug1";

/// The object: a channel into the event loop, where every call is carried out.
#[derive(Debug)]
pub struct Debug1 {
    calls: calloop::channel::Sender<DebugCall>,
}

impl Debug1 {
    /// The well-known name the server owns on the private bus (`casementctl` calls it).
    pub const BUS_NAME: &'static str = "org.quire.Casement.Debug1";

    /// The object over `calls`.
    pub fn new(calls: calloop::channel::Sender<DebugCall>) -> Debug1 {
        Debug1 { calls }
    }

    /// Hands `request` to the event loop and waits for its answer. A loop that is gone (the
    /// channel or the reply dropped) is an error to the caller, never a panic here.
    async fn ask(&self, request: DebugRequest) -> fdo::Result<DebugReply> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.calls
            .send(DebugCall { request, reply })
            .map_err(|_| fdo::Error::Failed("casement's event loop is gone".to_owned()))?;
        match answer.await {
            Ok(DebugReply::Refused(why)) => Err(fdo::Error::Failed(why)),
            Ok(reply) => Ok(reply),
            Err(_) => Err(fdo::Error::Failed(
                "casement's event loop dropped the call".to_owned(),
            )),
        }
    }

    /// `ask`, for a call whose only answer is a JSON document.
    async fn ask_json(&self, request: DebugRequest) -> fdo::Result<String> {
        match self.ask(request).await? {
            DebugReply::Json(json) => Ok(json),
            other => Err(unexpected(&other)),
        }
    }

    /// `ask`, for a call whose only answer is "done".
    async fn ask_done(&self, request: DebugRequest) -> fdo::Result<()> {
        match self.ask(request).await? {
            DebugReply::Done => Ok(()),
            other => Err(unexpected(&other)),
        }
    }
}

fn unexpected(reply: &DebugReply) -> fdo::Error {
    fdo::Error::Failed(format!("unexpected answer from the event loop: {reply:?}"))
}

fn invalid(what: &str, text: &str) -> fdo::Error {
    fdo::Error::InvalidArgs(format!("{what}: {text}"))
}

/// One call, with where its answer goes.
#[derive(Debug)]
pub struct DebugCall {
    /// What was asked.
    pub request: DebugRequest,
    /// Where the event loop sends the answer.
    pub reply: tokio::sync::oneshot::Sender<DebugReply>,
}

/// What `casementctl debug` asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugRequest {
    /// `Inject(lines)`.
    Inject(Vec<InjectLine>),
    /// `Wait(predicate, timeout_ms)`.
    Wait(WaitPredicate, Duration),
    /// `Clock("step", n)`.
    Step(u32),
    /// `Clock("advance", ms)`.
    Advance(Duration),
    /// `Output(verb, args)`: `list`, `add SPEC`, `remove NAME`, `set NAME KEY=VALUE`.
    Output(String, Vec<String>),
    /// `Windows()`.
    Windows,
    /// `Scene()`.
    Scene,
    /// `Fault(verb, amount)`: `panic-after-frames`, `stall-client-ms`, `drop-output`.
    Fault(String, u32),
    /// `Shot(area, path)`: write the last presented frame (or a region of it) as a PNG.
    Shot(ShotArea, String),
}

/// What the event loop answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugReply {
    /// `ok <line>` per injected line.
    Injected(Vec<String>),
    /// The wait's outcome (`met` or `timeout`) and the scene then, as JSON.
    Waited(String, String),
    /// A JSON document (output list, windows, scene).
    Json(String),
    /// Done, nothing to say.
    Done,
    /// Refused, and why.
    Refused(String),
}

#[zbus::interface(name = "org.quire.Casement.Debug1")]
impl Debug1 {
    /// `Inject(lines: as) -> as`: `ok <line>` per line, in order. Every line parses before any
    /// is carried out, so a typo on line 7 injects nothing.
    async fn inject(&self, lines: Vec<String>) -> fdo::Result<Vec<String>> {
        let parsed = lines
            .iter()
            .enumerate()
            .map(|(n, line)| {
                parse_inject(line)
                    .map_err(|error| invalid(&format!("line {}", n + 1), &error.to_string()))
            })
            .collect::<fdo::Result<Vec<InjectLine>>>()?;
        match self.ask(DebugRequest::Inject(parsed)).await? {
            // `ok <line>` as the grammar documents it: the line as written, one per line the loop
            // carried out (its own answers are the parsed lines' `Debug` text).
            DebugReply::Injected(done) => Ok(lines
                .iter()
                .take(done.len())
                .map(|line| format!("ok {}", line.trim()))
                .collect()),
            other => Err(unexpected(&other)),
        }
    }

    /// `Wait(predicate: s, timeout_ms: u) -> (s, s)`: outcome (`met` or `timeout`) and the scene
    /// JSON then.
    async fn wait(&self, predicate: String, timeout_ms: u32) -> fdo::Result<(String, String)> {
        let parsed =
            parse_wait(&predicate).map_err(|error| invalid("predicate", &error.to_string()))?;
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        match self.ask(DebugRequest::Wait(parsed, timeout)).await? {
            DebugReply::Waited(outcome, scene) => Ok((outcome, scene)),
            other => Err(unexpected(&other)),
        }
    }

    /// `Clock(verb: s, amount: u)`: `step N` (N vblanks) or `advance MS` (stepped clock only).
    async fn clock(&self, verb: String, amount: u32) -> fdo::Result<()> {
        let request = match verb.as_str() {
            "step" => DebugRequest::Step(amount),
            "advance" => DebugRequest::Advance(Duration::from_millis(u64::from(amount))),
            _ => return Err(invalid("clock verb (step|advance)", &verb)),
        };
        self.ask_done(request).await
    }

    /// `Output(verb: s, args: as) -> s`: `list` (JSON), `add SPEC`, `remove NAME`,
    /// `set NAME KEY=VALUE...` (`scale=SCALE120`, `mode=WxH@MILLIHERTZ`, `position=X,Y`,
    /// `power=on|off`); `add`, `remove` and `set` answer an empty string.
    async fn output(&self, verb: String, args: Vec<String>) -> fdo::Result<String> {
        check_output_call(&verb, &args)?;
        match self.ask(DebugRequest::Output(verb, args)).await? {
            DebugReply::Json(json) => Ok(json),
            DebugReply::Done => Ok(String::new()),
            other => Err(unexpected(&other)),
        }
    }

    /// `Windows() -> s`: JSON in the shape KWin's window list gave the consumer scripts
    /// (`class`, `frame`, `buffer`; SP1 B2).
    async fn windows(&self) -> fdo::Result<String> {
        self.ask_json(DebugRequest::Windows).await
    }

    /// `Scene() -> s`: the element tree as JSON.
    async fn scene(&self) -> fdo::Result<String> {
        self.ask_json(DebugRequest::Scene).await
    }

    /// `Shot(area: s, path: s) -> s`: writes the last frame an output presented (the whole
    /// output, or a logical region of it) as a PNG at `path` (inside the jail, which the compositor
    /// and the caller share) and answers its size as JSON (`{"width":W,"height":H,"scale":S120}`).
    /// A stand-in for ext-image-copy-capture until P1-C1.
    async fn shot(&self, area: String, path: String) -> fdo::Result<String> {
        let parsed = parse_shot(&area).map_err(|error| invalid("shot area", &error.to_string()))?;
        self.ask_json(DebugRequest::Shot(parsed, path)).await
    }

    /// `Fault(verb: s, amount: u)`: `panic-after-frames N`, `stall-client-ms MS`, and
    /// `drop-output INDEX` (the position in `Output("list")`, from 0).
    async fn fault(&self, verb: String, amount: u32) -> fdo::Result<()> {
        match verb.as_str() {
            "panic-after-frames" | "stall-client-ms" | "drop-output" => {
                self.ask_done(DebugRequest::Fault(verb, amount)).await
            }
            _ => Err(invalid(
                "fault verb (panic-after-frames|stall-client-ms|drop-output)",
                &verb,
            )),
        }
    }
}

/// An `Output` call's shape, checked before the event loop sees it.
fn check_output_call(verb: &str, args: &[String]) -> fdo::Result<()> {
    let arity =
        |expected: &str| invalid(&format!("output {verb} takes {expected}"), &args.join(" "));
    match (verb, args) {
        ("list", []) => Ok(()),
        ("list", _) => Err(arity("no arguments")),
        ("add", [spec]) => VirtualOutputSpec::parse(spec)
            .map(drop)
            .map_err(|error| invalid("output spec", &error.to_string())),
        ("add", _) => Err(arity("one SPEC")),
        ("remove", [name]) => OutputName::parse(name)
            .map(drop)
            .map_err(|error| invalid("output name", &error.to_string())),
        ("remove", _) => Err(arity("one NAME")),
        ("set", [name, settings @ ..]) if !settings.is_empty() => {
            OutputName::parse(name).map_err(|error| invalid("output name", &error.to_string()))?;
            settings
                .iter()
                .all(|setting| {
                    setting
                        .split_once('=')
                        .is_some_and(|(k, v)| !k.is_empty() && !v.is_empty())
                })
                .then_some(())
                .ok_or_else(|| invalid("output set wants KEY=VALUE", &settings.join(" ")))
        }
        ("set", _) => Err(arity("NAME and at least one KEY=VALUE")),
        _ => Err(invalid("output verb (list|add|remove|set)", verb)),
    }
}

#[cfg(test)]
mod tests;
