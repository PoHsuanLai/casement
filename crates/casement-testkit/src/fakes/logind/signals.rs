//! The signals the fake sends: the person's lock, the machine's sleep, the seat's device
//! pauses.

use super::manager::SleepPhase;
use super::session::PauseKind;
use super::{LogindError, MANAGER_PATH, SESSION_PATH};
use serde::Serialize;
use zbus::Connection;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{DynamicType, Fd};

const MANAGER: &str = "org.freedesktop.login1.Manager";
const SESSION: &str = "org.freedesktop.login1.Session";

/// Sends signals on the fake's connection.
#[derive(Debug, Clone)]
pub(super) struct Signals {
    connection: Connection,
}

impl Signals {
    pub(super) fn new(connection: Connection) -> Signals {
        Signals { connection }
    }

    async fn emit<B>(
        &self,
        path: &'static str,
        interface: &'static str,
        member: &'static str,
        body: &B,
    ) -> Result<(), LogindError>
    where
        B: Serialize + DynamicType,
    {
        let emit = |source: zbus::Error| LogindError::Emit {
            member,
            source: Box::new(source),
        };
        SignalEmitter::new(&self.connection, path)
            .map_err(emit)?
            .emit(interface, member, body)
            .await
            .map_err(emit)
    }

    pub(super) async fn lock(&self) -> Result<(), LogindError> {
        self.emit(SESSION_PATH, SESSION, "Lock", &()).await
    }

    pub(super) async fn unlock(&self) -> Result<(), LogindError> {
        self.emit(SESSION_PATH, SESSION, "Unlock", &()).await
    }

    pub(super) async fn prepare_for_sleep(&self, phase: SleepPhase) -> Result<(), LogindError> {
        self.emit(
            MANAGER_PATH,
            MANAGER,
            "PrepareForSleep",
            &(phase.starting(),),
        )
        .await
    }

    pub(super) async fn pause_device(
        &self,
        major: u32,
        minor: u32,
        kind: PauseKind,
    ) -> Result<(), LogindError> {
        self.emit(
            SESSION_PATH,
            SESSION,
            "PauseDevice",
            &(major, minor, kind.wire()),
        )
        .await
    }

    pub(super) async fn resume_device(
        &self,
        major: u32,
        minor: u32,
        device: &std::os::fd::OwnedFd,
    ) -> Result<(), LogindError> {
        self.emit(
            SESSION_PATH,
            SESSION,
            "ResumeDevice",
            &(major, minor, Fd::from(device)),
        )
        .await
    }
}
