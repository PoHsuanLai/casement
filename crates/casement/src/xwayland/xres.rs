//! The X server's answer to "which windows are there and whose are they": a short-lived X11
//! connection to the display casement serves, one `QueryTree` of the root, and for each child
//! its `WM_CLASS` and the X-Resource extension's `XResQueryClientIds` with `LocalClientPID`
//! (the pid the server read from the window's connection itself, which the client cannot
//! forge). The connection is made on a thread the loop waits on for a short while only: force
//! quit exists for programs that hang, and a hung X server must not hang the compositor too.

use super::DisplayName;
use super::owners::{WmClass, XWindow, XWindows};
use rustix::process::RawPid;
use std::sync::mpsc;
use std::time::Duration;
use x11rb::connection::Connection as _;
use x11rb::errors::{ConnectError, ConnectionError, ReplyError};
use x11rb::protocol::res::{ClientIdMask, ClientIdSpec, ConnectionExt as _};
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _};
use x11rb::rust_connection::RustConnection;

/// How long force quit waits for the X server to answer.
const DEADLINE: Duration = Duration::from_millis(750);

/// The longest `WM_CLASS` value read, in four-byte units.
const CLASS_UNITS: u32 = 256;

/// Why the X server could not be asked.
#[derive(Debug, thiserror::Error)]
enum XQueryError {
    /// The display could not be connected to.
    #[error("connect: {0}")]
    Connect(#[from] ConnectError),
    /// The connection broke.
    #[error("connection: {0}")]
    Connection(#[from] ConnectionError),
    /// The server answered with an error.
    #[error("reply: {0}")]
    Reply(#[from] ReplyError),
    /// The server has no screen to list the windows of.
    #[error("the server has no screen")]
    NoScreen,
}

/// The X server behind a display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XServer {
    display: DisplayName,
}

impl XServer {
    /// The server serving `display`.
    pub fn at(display: &DisplayName) -> XServer {
        XServer {
            display: display.clone(),
        }
    }
}

impl XWindows for XServer {
    fn top_levels(&self) -> Vec<XWindow> {
        let (sender, receiver) = mpsc::channel();
        let name = self.display.0.clone();
        std::thread::spawn(move || {
            // The receiver is gone when the wait ran out: nobody wants the answer then.
            let _ = sender.send(query(&name));
        });
        match receiver.recv_timeout(DEADLINE) {
            Ok(Ok(windows)) => windows,
            Ok(Err(error)) => {
                tracing::warn!(%error, display = %self.display.0, "the X windows could not be listed");
                Vec::new()
            }
            Err(_) => {
                tracing::warn!(display = %self.display.0, "the X server did not answer in time");
                Vec::new()
            }
        }
    }
}

/// Lists the root's children with their classes and owning processes. A window that goes away
/// between the questions is left out of the answer, not an error.
fn query(display: &str) -> Result<Vec<XWindow>, XQueryError> {
    let (connection, screen) = RustConnection::connect(Some(display))?;
    let root = connection
        .setup()
        .roots
        .get(screen)
        .map(|screen| screen.root)
        .ok_or(XQueryError::NoScreen)?;
    let children = connection.query_tree(root)?.reply()?.children;
    // Every question is sent before the first answer is read.
    let asked = children
        .iter()
        .map(|window| -> Result<_, ConnectionError> {
            let spec = ClientIdSpec {
                client: *window,
                mask: ClientIdMask::LOCAL_CLIENT_PID,
            };
            Ok((
                connection.get_property(
                    false,
                    *window,
                    AtomEnum::WM_CLASS,
                    AtomEnum::STRING,
                    0,
                    CLASS_UNITS,
                )?,
                connection.res_query_client_ids(&[spec])?,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(asked
        .into_iter()
        .map(|(class, ids)| XWindow {
            class: class
                .reply()
                .ok()
                .and_then(|reply| WmClass::parse(&reply.value)),
            pid: ids.reply().ok().and_then(|reply| {
                reply
                    .ids
                    .iter()
                    .flat_map(|id| id.value.iter())
                    .find_map(|pid| RawPid::try_from(*pid).ok())
            }),
        })
        .collect())
}
