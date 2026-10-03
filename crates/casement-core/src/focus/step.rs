//! The focus machine's transition (03 §1.2's table).
//!
//! Each [`FocusIn`] has one handler in `step/handlers.rs` that says where focus goes; the
//! effects that follow from the move itself (the keyboard, the emphasis, the active app) are
//! worked out once, from the difference, in `step/outcome.rs`.

mod handlers;
mod keying;
mod outcome;

use super::{Focus, FocusIn, FocusOut, FocusView};
use outcome::Outcome;

/// The focus after `input`, and what it asks for. Effects come in one order: raise, the press
/// route, the keyboard, emphasis, the active app, then attention and the bell. The keyboard,
/// emphasis and active-app effects appear only when they changed.
pub fn step(focus: Focus, input: FocusIn, view: &FocusView) -> (Focus, Vec<FocusOut>) {
    let before = focus.clone();
    let outcome = match input {
        FocusIn::Press(press) => handlers::press(focus, &press, view),
        FocusIn::Token { window, token } => handlers::token(focus, window, token, view),
        FocusIn::Mapped(window) => handlers::mapped(focus, window, view),
        FocusIn::Unmapped(window) => handlers::unmapped(focus, window, view),
        FocusIn::ActivateApp { app, by } => handlers::activate_app(focus, app, by, view),
        FocusIn::HideApp(app) => handlers::hide_app(focus, &app, view),
        FocusIn::UnhideApp(_) => Outcome::of(focus),
        FocusIn::SpaceShown { .. } => handlers::space_shown(focus, view),
        FocusIn::SheetOpened { parent, sheet } => {
            handlers::sheet_opened(focus, parent, sheet, view)
        }
        FocusIn::SheetClosed { parent } => handlers::sheet_closed(focus, parent, view),
    };
    outcome.finish(&before, view)
}
