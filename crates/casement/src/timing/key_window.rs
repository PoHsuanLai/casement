//! The `key-window` line: the key window changed.

use super::stamp::Stamp;
use super::token::Token;

/// What caused a change of key window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCause {
    /// A key press the compositor took (a chord of its own).
    Keyboard,
    /// A pointer press (a click on a window).
    Pointer,
    /// Something asked for it: the shell, a client's activation, a window list client.
    Request,
    /// None of those: a window opened or closed, a Space changed.
    Other,
}

impl KeyCause {
    /// The word the line prints.
    pub fn word(self) -> &'static str {
        match self {
            KeyCause::Keyboard => "keyboard",
            KeyCause::Pointer => "pointer",
            KeyCause::Request => "request",
            KeyCause::Other => "other",
        }
    }
}

/// A change of key window waiting for the frame that shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChange {
    /// The new key window's app id.
    pub app_id: Token,
    /// Why it changed.
    pub cause: KeyCause,
    /// The input event that caused it; [`Stamp::NONE`] when none did.
    pub input_at: Stamp,
    /// When the compositor made the window key.
    pub focused_at: Stamp,
}

impl KeyChange {
    /// The change as shown by a frame presented at `at` (never before the change itself).
    pub fn presented(self, at: Stamp) -> KeyWindow {
        KeyWindow {
            presented_at: at.or_later(self.focused_at),
            change: self,
        }
    }
}

/// A change of key window, complete: what the `key-window` line says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyWindow {
    /// The change.
    pub change: KeyChange,
    /// The first frame presented after it.
    pub presented_at: Stamp,
}

impl KeyWindow {
    /// The line, message first, then `key=value` fields.
    pub fn line(&self) -> String {
        format!(
            "key-window app_id={} cause={} input_at={} focused_at={} presented_at={}",
            self.change.app_id,
            self.change.cause.word(),
            self.change.input_at,
            self.change.focused_at,
            self.presented_at,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(app_id: &str, cause: KeyCause, input_at: u64) -> KeyChange {
        KeyChange {
            app_id: Token::of(app_id),
            cause,
            input_at: Stamp(input_at),
            focused_at: Stamp(2_000),
        }
    }

    #[test]
    fn the_line_has_the_fields_the_desktop_scripts_read_in_order() {
        // name, change, presented at, line
        let cases = [
            (
                "a chord",
                change("foot", KeyCause::Keyboard, 1_500),
                3_000,
                "key-window app_id=foot cause=keyboard input_at=1500 focused_at=2000 presented_at=3000",
            ),
            (
                "a click",
                change("org.gnome.Nautilus", KeyCause::Pointer, 1_900),
                2_500,
                "key-window app_id=org.gnome.Nautilus cause=pointer input_at=1900 focused_at=2000 presented_at=2500",
            ),
            (
                "a request with no input",
                change("firefox", KeyCause::Request, 0),
                2_600,
                "key-window app_id=firefox cause=request input_at=0 focused_at=2000 presented_at=2600",
            ),
            (
                "another cause",
                change("Blender", KeyCause::Other, 0),
                2_100,
                "key-window app_id=Blender cause=other input_at=0 focused_at=2000 presented_at=2100",
            ),
            (
                "a frame stamped before the change is read as the change",
                change("foot", KeyCause::Other, 0),
                1_000,
                "key-window app_id=foot cause=other input_at=0 focused_at=2000 presented_at=2000",
            ),
        ];
        for (name, change, presented, line) in cases {
            assert_eq!(change.presented(Stamp(presented)).line(), line, "{name}");
        }
    }
}
