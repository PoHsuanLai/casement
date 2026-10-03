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
