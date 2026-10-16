//! Where an input event came from (C19): set where it enters, never changed downstream.

use crate::ids::LeaseId;
use serde::{Deserialize, Serialize};

/// The source of an input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Origin {
    /// A kernel input device casement opened itself, not a virtual one.
    Physical(DeviceClass),
    /// A user-writable virtual device (uinput, `virtual_keyboard`): never physical (I8).
    Virtual(DeviceClass),
    /// A trusted feed (palmrest's stream): scroll and system gestures only.
    Feed(FeedSource),
    /// The agent seat of this lease.
    Agent(LeaseId),
    /// A RemoteDesktop portal session.
    Remote(PortalSessionId),
    /// Headless tests only; never compiled into a release seat.
    Test,
}

impl Origin {
    /// Whether this origin counts as a person at the hardware (I8): a physical device, or the
    /// headless tests standing in for one. A virtual device never does.
    pub fn is_physical(&self) -> bool {
        matches!(self, Origin::Physical(_) | Origin::Test)
    }

    /// Whether a key chord from this origin may fire a binding: the keyboard the person
    /// types on, a virtual keyboard (an on-screen one, keycap's output) and the tests. Never
    /// the agent seat, a remote session or a feed.
    pub fn fires_bindings(&self) -> bool {
        matches!(
            self,
            Origin::Physical(_) | Origin::Virtual(_) | Origin::Test
        )
    }

    /// Whether a key from this origin may switch the virtual terminal (CASW-13, decided
    /// 2026-10-16): only a kernel keyboard casement opened itself. A virtual keyboard, a remote
    /// session, the agent seat and a feed never do, so no client can send the compositor to a
    /// text console, the lock included.
    pub fn switches_vt(&self) -> bool {
        matches!(self, Origin::Physical(_))
    }
}

/// What kind of device an event came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    /// A keyboard.
    Keyboard,
    /// A mouse or trackball.
    Pointer,
    /// A touchpad.
    Touchpad,
    /// A drawing tablet.
    Tablet,
    /// A touchscreen.
    Touch,
    /// A switch (lid, tablet mode).
    Switch,
}

/// A trusted feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedSource {
    /// palmrest's touchpad stream.
    Palmrest,
}

/// One RemoteDesktop portal session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PortalSessionId(pub u64);
