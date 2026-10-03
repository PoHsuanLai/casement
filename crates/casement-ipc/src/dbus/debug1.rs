//! `org.quire.Casement.Debug1` at `/org/quire/Casement/Debug1` (feature `debug-inject`, the
//! jail's private bus only): injection, waits, the stepped clock, virtual outputs, model and
//! scene dumps and fault injection for `casementctl debug`.

use crate::inject::InjectLine;
use crate::wait::WaitPredicate;
use std::time::Duration;

/// The object's path.
pub const OBJECT_PATH: &str = "/org/quire/Casement/Debug1";

/// The object: a channel into the event loop, where every call is carried out.
#[derive(Debug)]
pub struct Debug1 {
    #[expect(dead_code, reason = "P0-T-dev: the channel into the event loop")]
    calls: calloop::channel::Sender<DebugCall>,
}

impl Debug1 {
    /// The object over `calls`.
    pub fn new(calls: calloop::channel::Sender<DebugCall>) -> Debug1 {
        Debug1 { calls }
    }
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
    /// `Inject(lines: as) -> as`: `ok <line>` per line, in order.
    async fn inject(&self, lines: Vec<String>) -> zbus::fdo::Result<Vec<String>> {
        let _ = lines;
        todo!("P0-T-dev: Debug1.Inject")
    }

    /// `Wait(predicate: s, timeout_ms: u) -> (s, s)`: outcome and scene JSON.
    async fn wait(
        &self,
        predicate: String,
        timeout_ms: u32,
    ) -> zbus::fdo::Result<(String, String)> {
        let _ = (predicate, timeout_ms);
        todo!("P0-T-dev: Debug1.Wait")
    }

    /// `Clock(verb: s, amount: u)`: `step N` or `advance MS` (stepped clock only).
    async fn clock(&self, verb: String, amount: u32) -> zbus::fdo::Result<()> {
        let _ = (verb, amount);
        todo!("P0-T-dev: Debug1.Clock")
    }

    /// `Output(verb: s, args: as) -> s`.
    async fn output(&self, verb: String, args: Vec<String>) -> zbus::fdo::Result<String> {
        let _ = (verb, args);
        todo!("P0-T-dev: Debug1.Output")
    }

    /// `Windows() -> s`: JSON in the shape KWin's window list gave the consumer scripts
    /// (`class`, `frame`, `buffer`; SP1 B2).
    async fn windows(&self) -> zbus::fdo::Result<String> {
        todo!("P0-T-dev: Debug1.Windows")
    }

    /// `Scene() -> s`: the element tree as JSON.
    async fn scene(&self) -> zbus::fdo::Result<String> {
        todo!("P0-T-dev: Debug1.Scene")
    }

    /// `Fault(verb: s, amount: u)`.
    async fn fault(&self, verb: String, amount: u32) -> zbus::fdo::Result<()> {
        let _ = (verb, amount);
        todo!("P0-T-dev: Debug1.Fault")
    }
}
