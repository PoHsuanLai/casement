//! The focus machine's transition (03 §1.2's table).

use super::{Focus, FocusIn, FocusOut, FocusView};

/// The focus after `input`, and what it asks for.
pub fn step(focus: Focus, input: FocusIn, view: &FocusView) -> (Focus, Vec<FocusOut>) {
    let _ = (focus, input, view);
    todo!("P0-F1-focus: 03 §1.2 focus table")
}
