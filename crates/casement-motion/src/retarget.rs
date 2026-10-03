//! Retargeting: from one arrangement to the next, every leg starting from where it is at the
//! speed it has (design/27 §3.12 rule 4).

use crate::MotionBook;
use casement_core::Cause;
use casement_core::time::Mono;
use casement_layout::Arrangement;
use ds_style::appearance::motion::MotionLevel;

/// The book after the arrangement changed from `from` to `to` at `now` because of `cause`.
/// A leg whose target changed starts a new leg from its state at `now` (position and velocity
/// kept); a window that appears fades and grows in from its target, one that leaves fades out;
/// the spring comes from `cause` (`SpringSpec::for_touch`), in its reduced form under
/// [`MotionLevel::Reduced`].
pub fn retarget(
    book: MotionBook,
    from: &Arrangement,
    to: &Arrangement,
    cause: &Cause,
    level: MotionLevel,
    now: Mono,
) -> MotionBook {
    let _ = (book, from, to, cause, level, now);
    todo!("P0-F1-motion: new legs from each leg's state at now")
}
