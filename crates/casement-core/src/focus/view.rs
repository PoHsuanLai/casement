//! What the focus machine reads of the model.

use crate::ids::WindowId;
use crate::space::SpaceBook;
use crate::window::Window;
use std::collections::BTreeMap;

/// The model's windows and Spaces, borrowed for one focus step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusView<'a> {
    /// Every window.
    pub windows: &'a BTreeMap<WindowId, Window>,
    /// Every Space.
    pub spaces: &'a SpaceBook,
}
