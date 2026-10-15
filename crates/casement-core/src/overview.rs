//! Mission Control's life: the windows fly to their slots (entering), rest there (open), and fly
//! back (leaving).

/// Where Mission Control is in its life.
///
/// ```
/// use casement_core::overview::Overview;
///
/// assert_eq!(Overview::default(), Overview::Closed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Overview {
    /// Not shown: the windows are on the desktop.
    #[default]
    Closed,
    /// The windows are flying to their slots.
    Entering,
    /// The windows are in their slots.
    Open,
    /// The windows are flying back to the desktop.
    Leaving,
}
