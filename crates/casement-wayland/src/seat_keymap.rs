//! Switching the seat keyboard's layout while the compositor runs: what the seat's keymap was
//! compiled from, which names it refused, and the call that puts a new keymap on the keyboard.
//!
//! The decision is pure ([`KeymapBook::plan`]); [`switch_layout`] is the one call into smithay.
//! smithay's `set_xkb_config` builds the new state, replays the keys still held onto it (so a
//! held key stays held), recomputes the modifiers from them, sends every bound `wl_keyboard` the
//! new keymap and the focused client the modifiers. Latched and locked modifiers are not
//! carried over (the new state starts without them), and no key is released or repeated.

use crate::states::WaylandFault;
use casement_core::settings::XkbNames;
use smithay::input::SeatHandler;
use smithay::input::keyboard::{KeyboardHandle, XkbConfig};

/// The `XkbConfig` that compiles `names`. An empty name is libxkbcommon's default for it.
pub fn xkb_config(names: &XkbNames) -> XkbConfig<'_> {
    XkbConfig {
        rules: &names.rules,
        model: &names.model,
        layout: &names.layout,
        variant: &names.variant,
        options: Some(names.options.clone()).filter(|options| !options.is_empty()),
    }
}

/// What to do about the layout a settings change asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeymapStep {
    /// It is the keymap the seat has: nothing to do.
    Unchanged,
    /// It is the one that already failed to compile, and that was reported: nothing to do.
    Refused,
    /// A new layout: compile it and put it on the keyboard.
    Compile,
}

/// How a refusal is to be reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Report {
    /// The first time these names failed: say so.
    Say,
    /// The same names failed before: stay quiet.
    Quiet,
}

/// The names the seat's keymap came from and the names it refused last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeymapBook {
    running: XkbNames,
    refused: Option<XkbNames>,
}

impl KeymapBook {
    /// A seat whose keymap was compiled from `running`.
    ///
    /// ```
    /// use casement_core::settings::CasementSettings;
    /// use casement_wayland::seat_keymap::{KeymapBook, KeymapStep};
    ///
    /// let names = CasementSettings::defaults().keyboard.layout;
    /// assert_eq!(KeymapBook::new(names.clone()).plan(&names), KeymapStep::Unchanged);
    /// ```
    pub fn new(running: XkbNames) -> KeymapBook {
        KeymapBook {
            running,
            refused: None,
        }
    }

    /// The names the seat's keymap is compiled from.
    pub fn running(&self) -> &XkbNames {
        &self.running
    }

    /// What a settings change that asks for `wanted` needs.
    pub fn plan(&self, wanted: &XkbNames) -> KeymapStep {
        if *wanted == self.running {
            KeymapStep::Unchanged
        } else if self.refused.as_ref() == Some(wanted) {
            KeymapStep::Refused
        } else {
            KeymapStep::Compile
        }
    }

    /// The keyboard now has the keymap of `names`.
    pub fn switched(&mut self, names: XkbNames) {
        self.running = names;
        self.refused = None;
    }

    /// `names` did not compile; the old keymap stays. Says whether to report it.
    pub fn refuse(&mut self, names: XkbNames) -> Report {
        let report = if self.refused.as_ref() == Some(&names) {
            Report::Quiet
        } else {
            Report::Say
        };
        self.refused = Some(names);
        report
    }
}

/// Compiles `names` and puts the keymap on `keyboard`, for every bound client. On failure the
/// keyboard keeps its keymap, untouched.
pub fn switch_layout<D>(
    keyboard: &KeyboardHandle<D>,
    data: &mut D,
    names: &XkbNames,
) -> Result<(), WaylandFault>
where
    D: SeatHandler + 'static,
{
    keyboard
        .set_xkb_config(data, xkb_config(names))
        .map_err(|error| WaylandFault::Keymap(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::settings::CasementSettings;

    fn names(layout: &str) -> XkbNames {
        XkbNames {
            layout: layout.to_owned(),
            ..CasementSettings::defaults().keyboard.layout
        }
    }

    #[test]
    fn a_layout_change_compiles_and_the_same_names_or_a_refused_pair_do_not() {
        let mut book = KeymapBook::new(names("us"));
        assert_eq!(book.plan(&names("us")), KeymapStep::Unchanged);
        assert_eq!(book.plan(&names("de")), KeymapStep::Compile);
        assert_eq!(book.refuse(names("zz")), Report::Say);
        assert_eq!(book.plan(&names("zz")), KeymapStep::Refused);
        assert_eq!(book.plan(&names("us")), KeymapStep::Unchanged);
        assert_eq!(book.plan(&names("de")), KeymapStep::Compile);
        assert_eq!(
            book.running(),
            &names("us"),
            "a refusal keeps the old keymap"
        );
    }

    #[test]
    fn a_refusal_is_reported_once_per_pair_of_names() {
        let mut book = KeymapBook::new(names("us"));
        assert_eq!(book.refuse(names("zz")), Report::Say);
        assert_eq!(book.refuse(names("zz")), Report::Quiet);
        assert_eq!(book.refuse(names("yy")), Report::Say);
        assert_eq!(book.refuse(names("zz")), Report::Say);
    }

    #[test]
    fn a_switch_forgets_the_refusal() {
        let mut book = KeymapBook::new(names("us"));
        book.refuse(names("zz"));
        book.switched(names("de"));
        assert_eq!(book.running(), &names("de"));
        assert_eq!(book.plan(&names("zz")), KeymapStep::Compile);
    }
}
