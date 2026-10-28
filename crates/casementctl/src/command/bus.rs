//! The client side of `org.quire.Casement.Debug1` on the jail's private bus.

use super::plan::Call;
use std::time::Duration;
use zbus::{Connection, Proxy};

const DESTINATION: &str = "org.quire.Casement.Debug1";
const PATH: &str = "/org/quire/Casement/Debug1";
const INTERFACE: &str = "org.quire.Casement.Debug1";
/// Slack on top of a wait's own bound: the call itself must come back before this runs out.
const WAIT_SLACK: Duration = Duration::from_secs(15);
/// How long any other call may take. The loop answers in a frame; this only catches a hang.
const CALL_PATIENCE: Duration = Duration::from_secs(60);

/// Why a call failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BusError {
    /// `$DBUS_SESSION_BUS_ADDRESS` is not set: not started through `dev/up.sh`.
    #[error(
        "DBUS_SESSION_BUS_ADDRESS is not set (start casement with dev/up.sh and source its env file)"
    )]
    NoBus,
    /// The bus or the call refused.
    #[error("{0}")]
    Zbus(#[from] zbus::Error),
    /// The call did not come back in time.
    #[error("no answer from casement within {0:?}")]
    TimedOut(Duration),
}

/// What a call answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// Lines, one per injected line.
    Lines(Vec<String>),
    /// A wait's outcome and the scene then.
    Waited {
        /// `met` or `timeout`.
        outcome: String,
        /// The scene as JSON.
        scene: String,
    },
    /// A document or a short text (an empty one for a call that answers nothing).
    Text(String),
}

/// The connection to the private bus named by `$DBUS_SESSION_BUS_ADDRESS`.
pub async fn connect() -> Result<Connection, BusError> {
    let address = std::env::var("DBUS_SESSION_BUS_ADDRESS").map_err(|_| BusError::NoBus)?;
    Ok(zbus::connection::Builder::address(address.as_str())?
        .build()
        .await?)
}

/// Makes `call` on `connection`.
pub async fn make(connection: &Connection, call: Call) -> Result<Answer, BusError> {
    let proxy = Proxy::new(connection, DESTINATION, PATH, INTERFACE).await?;
    let patience = match &call {
        Call::Wait { timeout, .. } => *timeout + WAIT_SLACK,
        _ => CALL_PATIENCE,
    };
    let sent = send(&proxy, call);
    tokio::time::timeout(patience, sent)
        .await
        .map_err(|_| BusError::TimedOut(patience))?
}

async fn send(proxy: &Proxy<'_>, call: Call) -> Result<Answer, BusError> {
    Ok(match call {
        Call::Inject(lines) => Answer::Lines(proxy.call("Inject", &(lines,)).await?),
        Call::Wait { predicate, timeout } => {
            let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
            let (outcome, scene): (String, String) = proxy.call("Wait", &(predicate, ms)).await?;
            Answer::Waited { outcome, scene }
        }
        Call::Clock(verb, amount) => {
            proxy.call::<_, _, ()>("Clock", &(verb, amount)).await?;
            Answer::Text(String::new())
        }
        Call::Output(verb, args) => Answer::Text(proxy.call("Output", &(verb, args)).await?),
        Call::Windows => Answer::Text(proxy.call("Windows", &()).await?),
        Call::Scene => Answer::Text(proxy.call("Scene", &()).await?),
        Call::Shot { area, path } => Answer::Text(proxy.call("Shot", &(area, path)).await?),
        Call::Fault(verb, amount) => {
            proxy.call::<_, _, ()>("Fault", &(verb, amount)).await?;
            Answer::Text(String::new())
        }
    })
}
