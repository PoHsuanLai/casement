//! Knowing when the desktop portal's bus connection goes away (NEW-PI-8). The portal asks for a
//! remote input socket per remote-control session (`ConnectEis`); if it is restarted or crashes,
//! the book of sessions it kept is gone, but the sockets it asked for are casement's and would
//! stay open for the app that holds them. Each connection that asks is remembered by its unique
//! bus name, and when the bus says that name has no owner any more, the event loop is asked to
//! end every socket that connection asked for.

use super::connect_ask::{ConnectAsk, ConnectRefusal, Proven};
use super::intake::Intake;
use peephole_bus::peephole_core::roles;
use std::collections::BTreeMap;
use std::future::poll_fn;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use zbus::export::futures_core::Stream;
use zbus::message::Type;
use zbus::{Connection, MatchRule, MessageStream};

/// The most connections watched at once: the portal is one connection, so a few is plenty; a
/// caller past this is refused.
pub const MAX_WATCHED: usize = 16;

/// How long to wait before asking the event loop again when its queue was full.
const RETRY_AFTER: Duration = Duration::from_millis(250);

/// How often to ask again before giving up.
const RETRIES: u32 = 40;

/// The connections that asked for remote sockets or leases, and who each had proven to be. The
/// agent tier's connections are kept apart from the portal's, each table with its own room, so
/// one cannot fill the other's.
#[derive(Debug, Default)]
pub struct PortalWatch {
    portal: Mutex<BTreeMap<String, Proven>>,
    agent: Mutex<BTreeMap<String, Proven>>,
}

impl PortalWatch {
    /// Starts watching the connection `owner`, which proved to be `caller`: false when there is
    /// no room to watch another.
    pub fn watch(&self, owner: &str, caller: &Proven) -> bool {
        let mut watched = self
            .table_of(caller)
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if watched.contains_key(owner) {
            return true;
        }
        if watched.len() < MAX_WATCHED {
            watched.insert(owner.to_owned(), caller.clone());
            true
        } else {
            false
        }
    }

    /// Stops watching `owner` because it left; who it had proven to be, if it was watched.
    pub fn left(&self, owner: &str) -> Option<Proven> {
        [&self.portal, &self.agent].into_iter().find_map(|table| {
            table
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(owner)
        })
    }

    fn table_of(&self, caller: &Proven) -> &Mutex<BTreeMap<String, Proven>> {
        match caller.role == roles::AGENT_TIER {
            true => &self.agent,
            false => &self.portal,
        }
    }
}

/// Listens for `NameOwnerChanged` from the bus itself and, for every watched connection that
/// leaves, asks the event loop to end its sockets. Runs for the life of the bus thread.
pub(super) async fn listen(connection: Connection, watch: Arc<PortalWatch>, intake: Arc<Intake>) {
    if let Err(error) = run(&connection, &watch, &intake).await {
        tracing::warn!(%error, "the portal's connection is not watched; its remote sockets are ended only by it");
    }
}

async fn run(connection: &Connection, watch: &PortalWatch, intake: &Intake) -> zbus::Result<()> {
    // The sender is the bus driver's well-known name, which no client can send as.
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?
        .build();
    let mut signals = Box::pin(MessageStream::for_match_rule(rule, connection, None).await?);
    while let Some(next) = poll_fn(|cx| signals.as_mut().poll_next(cx)).await {
        let Ok(message) = next else { continue };
        let Ok((name, _old, new)) = message.body().deserialize::<(String, String, String)>() else {
            continue;
        };
        if !new.is_empty() {
            continue;
        }
        if let Some(portal) = watch.left(&name) {
            tell_the_loop(intake, ConnectAsk::owner_gone(&name, portal)).await;
        }
    }
    Ok(())
}

/// Puts `ask` in the event loop's queue, again after a pause while it is full.
async fn tell_the_loop(intake: &Intake, ask: ConnectAsk) {
    for _ in 0..RETRIES {
        match intake.enqueue(ask.clone()) {
            Err(ConnectRefusal::TooMany) => tokio::time::sleep(RETRY_AFTER).await,
            _ => return,
        }
    }
    tracing::warn!(
        "the event loop did not take a portal's departure; its remote sockets stay open"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use peephole_bus::peephole_core::AppKey;

    /// The agent tier filling its table leaves the portal's room alone.
    #[test]
    fn the_agent_tier_cannot_fill_the_portals_table() {
        let watch = PortalWatch::default();
        let agent = Proven::new(
            AppKey::new("org.quire.Cua").expect("an app"),
            roles::AGENT_TIER,
        );
        for n in 0..MAX_WATCHED {
            assert!(watch.watch(&format!(":2.{n}"), &agent));
        }
        assert!(!watch.watch(":2.99", &agent));
        assert!(watch.watch(":1.5", &portal()));
        assert_eq!(watch.left(":2.0"), Some(agent));
    }

    fn portal() -> Proven {
        Proven::new(
            AppKey::new("org.quire.Portal").expect("an app"),
            roles::PORTAL,
        )
    }

    #[test]
    fn a_watched_connection_is_given_back_once_when_it_leaves() {
        let watch = PortalWatch::default();
        assert!(watch.watch(":1.5", &portal()));
        assert!(watch.watch(":1.5", &portal()), "asking twice is one");
        assert_eq!(watch.left(":1.9"), None, "a connection nobody watched");
        assert_eq!(watch.left(":1.5"), Some(portal()));
        assert_eq!(watch.left(":1.5"), None);
    }

    #[test]
    fn the_table_is_bounded() {
        let watch = PortalWatch::default();
        for n in 0..MAX_WATCHED {
            assert!(watch.watch(&format!(":1.{n}"), &portal()));
        }
        assert!(!watch.watch(":2.0", &portal()));
        assert!(
            watch.watch(":1.0", &portal()),
            "a watched one is still fine"
        );
    }
}
