//! The thread that registers: a current-thread tokio runtime of its own, the bus connected on
//! the first registration, one call at a time (so the pace holds), and every failure reported.

use super::pace::Pace;
use super::{
    NO_LOOK, QUEUE_LIMIT, RegisterFailure, RegisterFault, RegistryError, SpaceRegistration,
};
use porter_dbus::accounts::SpacesProxy;
use std::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender, channel, error::TrySendError};
use zbus::Connection;
use zbus::fdo;

/// A running worker: registrations go in, failures come back.
#[derive(Debug)]
pub(super) struct Worker {
    queue: Sender<SpaceRegistration>,
    failed: mpsc::Receiver<RegisterFailure>,
}

impl Worker {
    /// Starts the thread. It ends when the worker is dropped.
    pub(super) fn start(address: String) -> Result<Worker, std::io::Error> {
        let (queue, incoming) = channel(QUEUE_LIMIT);
        let (report, failed) = mpsc::channel();
        std::thread::Builder::new()
            .name("casement-spaces".to_owned())
            .spawn(move || run(&address, incoming, &report))?;
        Ok(Worker { queue, failed })
    }

    /// Queues `space`; it is refused when the queue is full or the thread has stopped.
    pub(super) fn send(&self, space: SpaceRegistration) -> Result<(), RegistryError> {
        self.queue.try_send(space).map_err(|error| match error {
            TrySendError::Full(_) => RegistryError::QueueFull,
            TrySendError::Closed(_) => {
                RegistryError::Thread(std::io::Error::other("the thread has stopped"))
            }
        })
    }

    /// The failures reported since the last time.
    pub(super) fn failures(&self) -> Vec<RegisterFailure> {
        self.failed.try_iter().collect()
    }
}

fn run(
    address: &str,
    mut incoming: Receiver<SpaceRegistration>,
    report: &mpsc::Sender<RegisterFailure>,
) {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        // No runtime: dropping the receiver makes every send fail, which the caller reports.
        return;
    };
    runtime.block_on(async move {
        let mut connection: Option<Connection> = None;
        let mut pace = Pace::default();
        let mut refused = false;
        while let Some(space) = incoming.recv().await {
            if refused {
                continue;
            }
            if let Some(wait) = pace.before_send() {
                tokio::time::sleep(wait).await;
            }
            let outcome = match connected(&mut connection, address).await {
                Ok(bus) => register_one(&bus, &space).await,
                Err(error) => Err(fault_of(&error)),
            };
            if let Err(fault) = outcome {
                refused = fault == RegisterFault::Refused;
                let _ = report.send(RegisterFailure {
                    id: space.id,
                    fault,
                });
            }
        }
    });
}

/// The bus, connected the first time it is asked for.
async fn connected(slot: &mut Option<Connection>, address: &str) -> zbus::Result<Connection> {
    if let Some(bus) = slot {
        return Ok(bus.clone());
    }
    let bus = zbus::connection::Builder::address(address)?.build().await?;
    *slot = Some(bus.clone());
    Ok(bus)
}

async fn register_one(bus: &Connection, space: &SpaceRegistration) -> Result<(), RegisterFault> {
    let proxy = SpacesProxy::new(bus).await.map_err(|e| fault_of(&e))?;
    proxy
        .register(space.id.as_str(), &space.name, NO_LOOK)
        .await
        .map_err(|e| fault_of(&e))
}

/// What an error from the bus or the service means for the caller.
fn fault_of(error: &zbus::Error) -> RegisterFault {
    match error {
        zbus::Error::FDO(inner) => match &**inner {
            fdo::Error::AccessDenied(_) => RegisterFault::Refused,
            fdo::Error::ServiceUnknown(_) | fdo::Error::NameHasNoOwner(_) => RegisterFault::Later,
            other => RegisterFault::Failed(other.to_string()),
        },
        zbus::Error::MethodError(name, _, _) => {
            let name = name.as_str();
            match name {
                _ if name.ends_with("AccessDenied") => RegisterFault::Refused,
                _ if name.ends_with("LimitsExceeded") => RegisterFault::Later,
                // The bus answers a call to a name nobody owns as a method error, not as the
                // typed FDO variant: accountd is not up yet.
                _ if name.ends_with("ServiceUnknown") || name.ends_with("NameHasNoOwner") => {
                    RegisterFault::Later
                }
                _ => RegisterFault::Failed(error.to_string()),
            }
        }
        zbus::Error::InputOutput(_) => RegisterFault::Later,
        other => RegisterFault::Failed(other.to_string()),
    }
}
