//! Identifiers: windows, clients, layers, popups, leases, devices, serials, processes and
//! outputs. Each is a newtype so an argument-order mistake is a compile error.

use crate::error::CoreError;
use crate::output::{EdidFacts, EdidSerial};
use serde::{Deserialize, Serialize};

/// A toplevel window, minted by the edge that saw it map ([`IdMint`]); never reused in a
/// session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowId(pub u64);

/// One Wayland client connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClientKey(pub u64);

/// One layer-shell surface (the bar, the dock, a wallpaper).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LayerKey(pub u64);

/// One popup (an xdg_popup of a window or a layer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PopupKey(pub u64);

/// One agent lease (`quire_agent_lease_v1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeaseId(pub u64);

/// One input device, as the backend numbered it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(pub u32);

/// A Wayland input serial, the proof a client's move, resize or activation request answers a
/// real press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InputSerial(pub u32);

/// A process id, from a client socket's peer credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pid(pub u32);

/// The longest output name, in bytes.
const MAX_OUTPUT_NAME: usize = 63;

/// A connector's name as the kernel or the headless backend gives it: `DP-1`, `eDP-1`,
/// `HDMI-A-1`, `HEADLESS-1`. Names a connector, not a display: a display is an [`OutputKey`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct OutputName(String);

impl OutputName {
    /// The name written as `text`: 1 to 63 ASCII letters, digits, `-` or `_`.
    pub fn parse(text: &str) -> Result<OutputName, CoreError> {
        let well_formed = !text.is_empty()
            && text.len() <= MAX_OUTPUT_NAME
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if well_formed {
            Ok(OutputName(text.to_owned()))
        } else {
            Err(CoreError::Malformed {
                what: "output name",
                text: text.to_owned(),
            })
        }
    }

    /// The name's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OutputName {
    type Error = CoreError;
    fn try_from(text: String) -> Result<OutputName, CoreError> {
        OutputName::parse(&text)
    }
}

impl From<OutputName> for String {
    fn from(name: OutputName) -> String {
        name.0
    }
}

/// A display, stable across replugging and across connectors: its EDID make, model and serial
/// when it reports a serial, else its connector (03 §1.15 M5). The key every stored output
/// setting and every Space's home is filed under.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OutputKey(String);

impl OutputKey {
    /// The key of the display `facts` describe on connector `name`: `edid:<make>/<model>/<serial>`
    /// when the EDID carries a serial, else `connector:<name>` (two identical displays without
    /// serials can only be told apart by where they are plugged). `/` and `%` inside the make,
    /// model and serial text are percent-encoded (`%2F`, `%25`) so the separators stay the only
    /// slashes: make `a/b` + model `c` and make `a` + model `b/c` are different keys (CASC-08).
    /// A key changes only for EDID text that holds one of those two characters.
    pub fn of(facts: &EdidFacts, name: &OutputName) -> OutputKey {
        match &facts.serial {
            EdidSerial::Known(serial) => OutputKey(format!(
                "edid:{}/{}/{}",
                key_text(&facts.make),
                key_text(&facts.model),
                key_text(serial)
            )),
            EdidSerial::Unknown => OutputKey(format!("connector:{}", name.as_str())),
        }
    }

    /// The key's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `text` with the key's separator `/` and its escape `%` percent-encoded.
fn key_text(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut out, c| {
            match c {
                '/' => out.push_str("%2F"),
                '%' => out.push_str("%25"),
                other => out.push(other),
            }
            out
        })
}

/// Mints window, layer and lease ids in order, from one counter shared by all three so no two
/// ids of a session share a number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdMint {
    next: u64,
}

impl IdMint {
    /// A mint whose first id is 1.
    pub fn start() -> IdMint {
        IdMint { next: 1 }
    }

    /// The next window id, and the mint after it.
    pub fn window(self) -> (IdMint, WindowId) {
        let (mint, n) = self.take();
        (mint, WindowId(n))
    }

    /// The next layer id, and the mint after it.
    pub fn layer(self) -> (IdMint, LayerKey) {
        let (mint, n) = self.take();
        (mint, LayerKey(n))
    }

    /// The next lease id, and the mint after it.
    pub fn lease(self) -> (IdMint, LeaseId) {
        let (mint, n) = self.take();
        (mint, LeaseId(n))
    }

    fn take(self) -> (IdMint, u64) {
        (
            IdMint {
                next: self.next.saturating_add(1),
            },
            self.next,
        )
    }
}
