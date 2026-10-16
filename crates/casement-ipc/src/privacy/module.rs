//! `org.quire.SettingsModule1` at [`PRIVACY_PATH`] on casement's session-bus connection: the
//! Privacy list Settings shows, from the model's rows, and the changes Settings makes to it.
//!
//! The model owns the answers. This module reads the board the event loop publishes to, and a
//! change it accepts becomes a [`PrivacyEdit`] in the queue the loop reads; the model then
//! writes its book. Only the Settings app may change anything, only Settings and the shell may
//! read, both proven all the way through the caller-identity library; anyone else is refused
//! and hears no `Changed`.

use super::board::{PrivacyBoard, PrivacyLink};
use super::edit::PrivacyEdit;
use super::keys::{changes, listed};
use super::write::{EditFault, edit_of};
use crate::dbus::{CallerCheck, require};
use calloop::channel::SyncSender;
use ds_settings::live::{
    Access, Caller, LiveError, LiveModule, LiveSchema, Served, Snapshot, Verdict, serve,
};
use ds_settings::schema::KeyPath;
use peephole_bus::peephole_core::roles;
use std::sync::Arc;
use std::sync::mpsc::TrySendError;
use zbus::Connection;

/// Where the module is served.
pub const PRIVACY_PATH: &str = "/org/quire/Compositor1/privacy";

/// The Privacy module.
#[derive(Debug)]
pub struct PrivacyModule {
    callers: Arc<dyn CallerCheck>,
    board: PrivacyBoard,
    edits: SyncSender<PrivacyEdit>,
}

impl PrivacyModule {
    /// The module over `link`, proving callers through `callers`.
    pub fn new(callers: Arc<dyn CallerCheck>, link: PrivacyLink) -> PrivacyModule {
        PrivacyModule {
            callers,
            board: link.board,
            edits: link.edits,
        }
    }

    fn schema(&self) -> LiveSchema {
        LiveSchema {
            version: 1,
            key: listed(&self.board.rows())
                .into_iter()
                .map(|key| key.spec)
                .collect(),
        }
    }
}

fn refused(fault: EditFault) -> LiveError {
    match fault {
        EditFault::Administrator => LiveError::NotPermitted(
            "Your administrator set this, so it cannot be changed here.".to_owned(),
        ),
        EditFault::ReadOut => LiveError::NotPermitted("This is only a note.".to_owned()),
        EditFault::BadValue => {
            LiveError::BadValue("A program is either allowed or not allowed.".to_owned())
        }
    }
}

impl LiveModule for PrivacyModule {
    async fn permit(&self, caller: &Caller, access: Access) -> Verdict {
        let readers = [roles::SETTINGS, roles::SHELL];
        let writers = [roles::SETTINGS];
        let allowed = match access {
            Access::Read => &readers[..],
            Access::Write => &writers[..],
        };
        match require(&*self.callers, Some(&caller.sender.0), allowed).await {
            Ok(_) => Verdict::Allow,
            Err(_) => Verdict::Refuse,
        }
    }

    async fn describe(&self) -> LiveSchema {
        self.schema()
    }

    async fn get(&self, key: &KeyPath) -> Result<toml::Value, LiveError> {
        listed(&self.board.rows())
            .into_iter()
            .find(|listed| listed.spec.path == *key)
            .map(|listed| listed.value)
            .ok_or_else(|| LiveError::UnknownKey(key.0.clone()))
    }

    async fn snapshot(&self) -> Result<Snapshot, LiveError> {
        let keys = listed(&self.board.rows());
        let values = keys
            .iter()
            .map(|key| (key.spec.path.clone(), key.value.clone()))
            .collect();
        let schema = LiveSchema {
            version: 1,
            key: keys.into_iter().map(|key| key.spec).collect(),
        };
        Ok(Snapshot { schema, values })
    }

    async fn set(&self, key: &KeyPath, value: toml::Value) -> Result<(), LiveError> {
        let rows = self.board.rows();
        let found = listed(&rows)
            .into_iter()
            .find(|listed| listed.spec.path == *key)
            .ok_or_else(|| LiveError::UnknownKey(key.0.clone()))?;
        let edit = edit_of(&found.write, &value).map_err(refused)?;
        self.edits
            .try_send(edit.clone())
            .map_err(|error| match error {
                TrySendError::Full(_) => LiveError::Failed(
                    "Privacy could not take that change just now. Try again.".to_owned(),
                ),
                TrySendError::Disconnected(_) => {
                    LiveError::Failed("Privacy is not available right now.".to_owned())
                }
            })?;
        // Settings reads its own change back at once; the model's list replaces this one.
        self.board.publish(edit.applied(&rows));
        Ok(())
    }
}

/// Serves the module on `connection` and starts telling Settings what changes on the board.
pub(crate) async fn serve_privacy(
    connection: &Connection,
    callers: Arc<dyn CallerCheck>,
    link: PrivacyLink,
) -> zbus::Result<()> {
    let board = link.board.clone();
    let served = serve(connection, PRIVACY_PATH, PrivacyModule::new(callers, link))
        .await
        .map_err(|error| zbus::Error::Failure(error.to_string()))?;
    drop(tokio::spawn(follow(served, board)));
    Ok(())
}

/// Sends `Changed` for each key the board's rows move, to the peers the module let read.
async fn follow(served: Served, board: PrivacyBoard) {
    let mut rows = board.subscribe();
    let mut before = Arc::clone(&rows.borrow_and_update());
    while rows.changed().await.is_ok() {
        let after = Arc::clone(&rows.borrow_and_update());
        for (key, value) in changes(&before, &after) {
            // A reader that left a moment ago loses its signal and no one else does.
            let _ = served.changed(&key, &value).await;
        }
        before = after;
    }
}
