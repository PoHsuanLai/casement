//! The timing lines (`casement::timing`) as a test reads them: the fixture keeps them in a list
//! instead of writing them to a log, and its clock is virtual, so a stamp is the `Mono` in
//! nanoseconds.

use super::Fixture;
use casement::timing::{Stamp, Timing};

impl Fixture {
    /// Makes the compositor keep its timing lines for [`Fixture::timing_lines`].
    pub(super) fn keep_timing_lines(&mut self) {
        self.state.timing = Timing::new(Stamp::NONE).kept();
    }

    /// The timing lines written so far, in order.
    pub fn timing_lines(&self) -> &[String] {
        self.state.timing.lines()
    }
}
