//! Whom a virtual input device was made for, as the desktop's device service (turnstile) wrote it
//! into the device (NEW-FA-4, decided 2026-10-17).
//!
//! The kernel keeps a free-text `phys` on every input device (`/sys/class/input/<event>/device/phys`).
//! Only a process that can open `/dev/uinput` can set it, and on this desktop that is turnstile
//! alone (its udev rule makes the node root-only), which makes a device only for the services it
//! has proven. casement links nothing of turnstile, so this is its own copy of turnstile's
//! `turnstile_wire::Maker::parse`; the test below and turnstile's pin the same strings.

use super::origin::{DeviceClass, Origin};

/// What comes before the role in a label.
const PREFIX: &str = "turnstile/";

/// The proven service a virtual device was made for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Maker {
    /// palmrest, the Magic Mouse gesture daemon (peephole role `pad_daemon`).
    PadDaemon,
    /// keycap, the Mac keyboard mapping daemon (peephole role `keymap_daemon`).
    KeymapDaemon,
}

impl Maker {
    /// The maker a `phys` text names, or `None` for any other text: no label, another prefix, a
    /// role this version does not know, anything after the role. A trailing newline (sysfs adds
    /// one) is ignored.
    pub fn parse(phys: &str) -> Option<Maker> {
        match phys.trim_end_matches('\n').strip_prefix(PREFIX)? {
            "pad_daemon" => Some(Maker::PadDaemon),
            "keymap_daemon" => Some(Maker::KeymapDaemon),
            _ => None,
        }
    }

    /// The origin of events from a virtual device made for this service.
    ///
    /// keycap only replays the person's own keyboard and mouse, so its devices are their hand.
    /// palmrest's are not: its daemon turns Magic Mouse gestures into synthetic key chords (Alt+Left
    /// by default, and the shell's settings choose others, `Ctrl+Alt+F3` among them), which must not
    /// switch the virtual terminal or answer a consent sheet. They still move the pointer's scroll,
    /// fire bindings and drive swipes like a pad's. The mouse's own clicks and motion do not pass
    /// through palmrest (it never grabs the mouse), so they stay physical.
    pub fn origin(self, class: DeviceClass) -> Origin {
        match self {
            Maker::KeymapDaemon => Origin::Physical(class),
            Maker::PadDaemon => Origin::Virtual(class),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// turnstile pins these same strings (its `maker` test in turnstile-wire): change one side and
    /// the other must change with it.
    #[test]
    fn the_labels_are_the_strings_turnstile_writes() {
        for (text, maker) in [
            ("turnstile/pad_daemon", Maker::PadDaemon),
            ("turnstile/keymap_daemon", Maker::KeymapDaemon),
        ] {
            assert_eq!(Maker::parse(text), Some(maker), "{text}");
            assert_eq!(Maker::parse(&format!("{text}\n")), Some(maker), "{text}");
        }
    }

    #[test]
    fn any_other_text_is_no_maker() {
        for text in [
            "",
            "turnstile/",
            "turnstile",
            "turnstile/settings",
            "turnstile/pad_daemon/extra",
            "turnstile/pad_daemon ",
            "turnstile/Pad_daemon",
            "xturnstile/pad_daemon",
            "ydotoold virtual device",
            "isa0060/serio0/input0",
        ] {
            assert_eq!(Maker::parse(text), None, "{text:?}");
        }
    }
}
