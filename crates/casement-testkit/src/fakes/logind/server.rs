//! [`FakeLogind`]: the fake's connection, its runtime and the test-facing drivers.

use super::control::Control;
use super::log::Recorder;
use super::manager::{Manager, SleepPhase};
use super::seat::Seat;
use super::session::{PauseKind, Session};
use super::signals::Signals;
use super::{
    BUS_NAME, CONTROL_PATH, LogindCall, LogindError, MANAGER_PATH, SEAT_PATH, SESSION_PATH,
};
use std::time::Duration;
use tokio::runtime::{Builder, Runtime};
use zbus::Connection;
use zbus::connection::Builder as ConnectionBuilder;

/// The fake logind, serving `org.freedesktop.login1` on one bus until dropped.
///
/// It owns a small runtime that drives the connection, so the drivers below are plain blocking
/// calls and must not be called from inside an async runtime.
#[derive(Debug)]
pub struct FakeLogind {
    runtime: Option<Runtime>,
    log: Recorder,
    signals: Signals,
}

impl FakeLogind {
    /// Serves on the bus at `address` (a [`PrivateBus`](super::PrivateBus) in a test) and owns
    /// `org.freedesktop.login1` there.
    ///
    /// ```no_run
    /// use casement_testkit::fakes::{FakeLogind, PrivateBus};
    /// let bus = PrivateBus::start().expect("bus");
    /// let logind = FakeLogind::serve(bus.address()).expect("serve");
    /// assert!(logind.calls().is_empty());
    /// ```
    pub fn serve(address: &str) -> Result<FakeLogind, LogindError> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|source| LogindError::Runtime { source })?;
        let log = Recorder::default();
        let handle = runtime.handle().clone();
        let served = runtime.block_on(connect(address, &log, handle));
        let connection = served.map_err(|source| LogindError::Bus {
            address: address.to_owned(),
            source,
        })?;
        Ok(FakeLogind {
            runtime: Some(runtime),
            log,
            signals: Signals::new(connection),
        })
    }

    /// The calls recorded so far, in order.
    pub fn calls(&self) -> Vec<LogindCall> {
        self.log.calls()
    }

    /// The `what` of each inhibitor still held (its pipe not yet closed).
    pub fn held_inhibitors(&self) -> Vec<String> {
        self.log.held()
    }

    /// Blocks until every inhibitor is released, for at most `timeout`; an error names what is
    /// still held.
    pub fn wait_released(&self, timeout: Duration) -> Result<(), LogindError> {
        self.log.wait_released(timeout)
    }

    /// Emits the session's `Lock` signal.
    pub fn lock(&self) -> Result<(), LogindError> {
        self.block_on(self.signals.lock())
    }

    /// Emits the session's `Unlock` signal.
    pub fn unlock(&self) -> Result<(), LogindError> {
        self.block_on(self.signals.unlock())
    }

    /// Emits `PrepareForSleep` for `phase`. Starting returns at once; a test then calls
    /// [`wait_released`](FakeLogind::wait_released) to see the inhibitors let the sleep go on.
    ///
    /// ```no_run
    /// use casement_testkit::fakes::{FakeLogind, SleepPhase};
    /// # fn run(logind: &FakeLogind) -> Result<(), casement_testkit::fakes::LogindError> {
    /// logind.prepare_for_sleep(SleepPhase::Starting)?;
    /// # Ok(()) }
    /// ```
    pub fn prepare_for_sleep(&self, phase: SleepPhase) -> Result<(), LogindError> {
        self.block_on(self.signals.prepare_for_sleep(phase))
    }

    /// Emits the session's `PauseDevice(major, minor, kind)` signal.
    pub fn pause_device(&self, major: u32, minor: u32, kind: PauseKind) -> Result<(), LogindError> {
        self.block_on(self.signals.pause_device(major, minor, kind))
    }

    /// Emits the session's `ResumeDevice(major, minor, fd)` signal with a fresh fixture
    /// descriptor.
    pub fn resume_device(&self, major: u32, minor: u32) -> Result<(), LogindError> {
        let (read, _write) = super::inhibit::pipe()?;
        self.block_on(self.signals.resume_device(major, minor, &read))
    }

    fn block_on<T>(
        &self,
        work: impl Future<Output = Result<T, LogindError>>,
    ) -> Result<T, LogindError> {
        match &self.runtime {
            Some(runtime) => runtime.block_on(work),
            None => Err(LogindError::Runtime {
                source: std::io::Error::other("runtime already shut down"),
            }),
        }
    }
}

impl Drop for FakeLogind {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

/// Connects, serves the four objects and claims the name.
async fn connect(
    address: &str,
    log: &Recorder,
    handle: tokio::runtime::Handle,
) -> zbus::Result<Connection> {
    // The control object needs the connection for its signals, so it is added after the build.
    let connection = ConnectionBuilder::address(address)?
        .name(BUS_NAME)?
        .serve_at(MANAGER_PATH, Manager::new(log.clone(), handle))?
        .serve_at(SESSION_PATH, Session::new(log.clone()))?
        .serve_at(SEAT_PATH, Seat::new(log.clone()))?
        .build()
        .await?;
    let control = Control::new(log.clone(), Signals::new(connection.clone()));
    connection.object_server().at(CONTROL_PATH, control).await?;
    Ok(connection)
}
