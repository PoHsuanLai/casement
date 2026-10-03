//! `wp_security_context_manager_v1` as a client (P0-H8): a sandbox engine makes a listening
//! socket, tells the compositor who is behind it and keeps the write end of a close pipe;
//! clients that connect to the socket arrive sandboxed. Dropping the engine closes the pipe and
//! the compositor stops listening.
//!
//! Self-contained like `misc::idle`, whose side queue it reuses.

use super::idle::{pump_queue, side_queue};
use crate::{ClientError, GlobalList, Pumped};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols::wp::security_context::v1::client::{
    wp_security_context_manager_v1::{self, WpSecurityContextManagerV1},
    wp_security_context_v1::{self, WpSecurityContextV1},
};

/// What a sandbox engine says about the app behind its socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineFacts {
    /// `set_sandbox_engine`, reverse-DNS.
    pub engine: String,
    /// `set_app_id`.
    pub app: String,
    /// `set_instance_id`.
    pub instance: String,
}

/// The helper's dispatch target; the objects it holds send no events.
#[derive(Debug)]
struct Quiet;

/// One committed security context: its socket, and the pipe end whose closing ends it.
#[derive(Debug)]
pub struct Listener {
    connection: Connection,
    queue: EventQueue<Quiet>,
    context: WpSecurityContextV1,
    path: PathBuf,
    close: UnixStream,
}

impl Listener {
    /// Binds a socket at `dir/<name>`, creates a context for it with `facts` and commits it.
    /// The requests are flushed; the compositor starts listening at the fixture's next pump.
    pub fn commit(
        globals: &GlobalList,
        dir: &Path,
        name: &str,
        facts: &EngineFacts,
    ) -> Result<Listener, ClientError> {
        let (connection, queue) = side_queue::<Quiet>(globals)?;
        let qh = queue.handle();
        let manager = globals
            .bind::<WpSecurityContextManagerV1, _, _>(&qh, 1..=1, ())
            .map_err(|_| ClientError::MissingGlobal("wp_security_context_manager_v1".to_owned()))?;
        let path = dir.join(name);
        let socket = UnixListener::bind(&path)
            .map_err(|error| ClientError::Connect(format!("{}: {error}", path.display())))?;
        let (close, close_far) = UnixStream::pair()
            .map_err(|error| ClientError::Connect(format!("close pipe: {error}")))?;
        let listen_fd = OwnedFd::from(socket);
        let close_fd = OwnedFd::from(close_far);
        let context = manager.create_listener(listen_fd.as_fd(), close_fd.as_fd(), &qh, ());
        context.set_sandbox_engine(facts.engine.clone());
        context.set_app_id(facts.app.clone());
        context.set_instance_id(facts.instance.clone());
        context.commit();
        connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))?;
        Ok(Listener {
            connection,
            queue,
            context,
            path,
            close,
        })
    }

    /// A new connection to the listening socket (the client end; the compositor accepts it at
    /// the fixture's next pump).
    pub fn connect(&self) -> Result<UnixStream, ClientError> {
        UnixStream::connect(&self.path)
            .map_err(|error| ClientError::Connect(format!("{}: {error}", self.path.display())))
    }

    /// Reads what arrived (nothing the helper keeps).
    pub fn pump(&mut self) -> Result<Pumped, ClientError> {
        pump_queue(&self.connection, &mut self.queue, &mut Quiet)
    }

    /// Sends `commit` again: the context was used, so the compositor posts `already_used`.
    pub fn commit_again(&mut self) -> Result<(), ClientError> {
        self.context.commit();
        self.connection
            .flush()
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }

    /// Closes the pipe the compositor watches: the listener stops accepting.
    pub fn close(self) -> Result<(), ClientError> {
        self.close
            .shutdown(std::net::Shutdown::Both)
            .map_err(|error| ClientError::Dispatch(error.to_string()))
    }
}

impl Dispatch<WpSecurityContextManagerV1, ()> for Quiet {
    fn event(
        _state: &mut Quiet,
        _proxy: &WpSecurityContextManagerV1,
        _event: wp_security_context_manager_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Quiet>,
    ) {
    }
}

impl Dispatch<WpSecurityContextV1, ()> for Quiet {
    fn event(
        _state: &mut Quiet,
        _proxy: &WpSecurityContextV1,
        _event: wp_security_context_v1::Event,
        _data: &(),
        _connection: &Connection,
        _qh: &QueueHandle<Quiet>,
    ) {
    }
}
