//! Which global needs which consent, and which desktop services are trusted without asking.
//!
//! A [`Power`] is the unit of consent: what the person decides on. The match in [`power_of`]
//! is exhaustive on purpose, like the sandbox table: a new global fails to compile until
//! someone decides whether it needs consent.

use super::globals::GlobalKind;
use crate::privilege::ServiceRole;
pub use casement_protocols::vocab::Power;
use casement_protocols::vocab::WireCode;
use std::collections::BTreeSet;

/// The power `global` needs consent for; none for a global every client may use. Layer shell
/// and the security-context manager stay without one: layer shell's keyboard grab is closed in
/// the model, and a security-context client is a sandbox, which sees none of these globals.
pub fn power_of(global: GlobalKind) -> Option<Power> {
    match global {
        GlobalKind::ImageCapture | GlobalKind::ToplevelCapture => Some(Power::Screen),
        GlobalKind::VirtualKeyboard => Some(Power::Keys),
        GlobalKind::InputMethod => Some(Power::TypingHelp),
        GlobalKind::ExtDataControl | GlobalKind::WlrDataControl => Some(Power::Clipboard),
        GlobalKind::SessionLock => Some(Power::Lock),
        GlobalKind::OutputManagement | GlobalKind::OutputPower | GlobalKind::Gamma => {
            Some(Power::Displays)
        }
        GlobalKind::ForeignToplevelList | GlobalKind::WlrToplevel | GlobalKind::Workspace => {
            Some(Power::Windows)
        }
        GlobalKind::Compositor
        | GlobalKind::Subcompositor
        | GlobalKind::Shm
        | GlobalKind::Output
        | GlobalKind::Viewporter
        | GlobalKind::SinglePixel
        | GlobalKind::AlphaModifier
        | GlobalKind::Presentation
        | GlobalKind::FractionalScale
        | GlobalKind::Fifo
        | GlobalKind::CommitTiming
        | GlobalKind::ContentType
        | GlobalKind::ColorManagement
        | GlobalKind::XdgShell
        | GlobalKind::XdgDecoration
        | GlobalKind::XdgActivation
        | GlobalKind::XdgDialog
        | GlobalKind::XdgToplevelIcon
        | GlobalKind::XdgToplevelTag
        | GlobalKind::XdgForeign
        | GlobalKind::SystemBell
        | GlobalKind::QuireWindow
        | GlobalKind::LayerShell
        | GlobalKind::Seat
        | GlobalKind::CursorShape
        | GlobalKind::PointerConstraints
        | GlobalKind::RelativePointer
        | GlobalKind::PointerGestures
        | GlobalKind::ShortcutsInhibit
        | GlobalKind::Tablet
        | GlobalKind::DataDevice
        | GlobalKind::PrimarySelection
        | GlobalKind::Dmabuf
        | GlobalKind::TextInput
        | GlobalKind::IdleNotify
        | GlobalKind::IdleInhibit
        | GlobalKind::SecurityContext
        | GlobalKind::BackgroundEffect
        | GlobalKind::QuireDesktop
        | GlobalKind::QuireShell => None,
    }
}

/// The powers a desktop service of `role` is trusted with, by identity, on the public socket:
/// the shell with all of them, the portal with the screen and the windows (its screenshots,
/// recordings and window switcher). Every other role is trusted with none; its program is
/// asked like any other.
pub fn trusted_powers(role: &ServiceRole) -> &'static [Power] {
    match role.as_str() {
        "shell" => <Power as WireCode>::ALL,
        "portal" => &[Power::Screen, Power::Windows],
        _ => &[],
    }
}

/// A set of powers: the ones a client is refused at connect time, for instance.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PowerSet(BTreeSet<Power>);

impl PowerSet {
    /// The set with `power` in it.
    pub fn with(self, power: Power) -> PowerSet {
        let mut powers = self.0;
        powers.insert(power);
        PowerSet(powers)
    }

    /// Whether `power` is in the set.
    pub fn contains(&self, power: Power) -> bool {
        self.0.contains(&power)
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The powers, in order.
    pub fn iter(&self) -> impl Iterator<Item = Power> + '_ {
        self.0.iter().copied()
    }
}

impl FromIterator<Power> for PowerSet {
    fn from_iter<I: IntoIterator<Item = Power>>(powers: I) -> PowerSet {
        PowerSet(powers.into_iter().collect())
    }
}
