//! The fake service manager: `FakeNotify` records the notices casement sends, so a test asserts
//! that ready came after the outputs and stopping on the way out. A clone shares the record.

use casement::notify::{Notice, Notify, NotifyError};
use std::sync::{Arc, Mutex, PoisonError};

/// A recorder standing in for systemd's notification socket.
#[derive(Debug, Clone, Default)]
pub struct FakeNotify {
    notices: Arc<Mutex<Vec<Notice>>>,
}

impl FakeNotify {
    /// A recorder with no notices.
    pub fn new() -> FakeNotify {
        FakeNotify::default()
    }

    /// The notices sent so far, in order.
    pub fn notices(&self) -> Vec<Notice> {
        self.notices
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Notify for FakeNotify {
    fn send(&mut self, notice: &Notice) -> Result<(), NotifyError> {
        self.notices
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(notice.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_are_recorded_in_order_and_shared_with_clones() {
        let mut fake = FakeNotify::new();
        let reader = fake.clone();
        fake.send(&Notice::Watchdog).expect("sent");
        fake.send(&Notice::Stopping).expect("sent");
        assert_eq!(reader.notices(), [Notice::Watchdog, Notice::Stopping]);
    }
}
