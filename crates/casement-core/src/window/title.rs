//! A window's title.

/// The title a client set. App-authored text: wherever it leaves casement it is untrusted, and
/// casement itself never draws it before `casement-chrome` (P3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowTitle(pub String);
