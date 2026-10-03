//! Keyboard settings (`keyboard.*`).

use std::time::Duration;

/// `keyboard.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardSettings {
    /// The XKB keymap names.
    pub layout: XkbNames,
    /// How long a key is held before it repeats.
    pub repeat_delay: Duration,
    /// How fast it repeats.
    pub repeat_rate: RepeatRate,
}

/// The XKB rule names a keymap is compiled from. An empty name means libxkbcommon's own
/// default for it (and its `XKB_DEFAULT_*` environment), so nothing here assumes a layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XkbNames {
    /// The rules file.
    pub rules: String,
    /// The keyboard model.
    pub model: String,
    /// The layouts, comma-separated.
    pub layout: String,
    /// The variants, comma-separated.
    pub variant: String,
    /// The options, comma-separated.
    pub options: String,
}

/// Key repeats per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepeatRate(pub u32);

impl KeyboardSettings {
    /// XKB's defaults, 400 ms, 10 per second.
    pub(crate) fn defaults() -> KeyboardSettings {
        KeyboardSettings {
            layout: XkbNames {
                rules: String::new(),
                model: String::new(),
                layout: String::new(),
                variant: String::new(),
                options: String::new(),
            },
            repeat_delay: Duration::from_millis(400),
            repeat_rate: RepeatRate(10),
        }
    }
}
