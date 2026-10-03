//! The session seam: who opens devices. Nothing at P0 (headless opens no device); a fake over
//! memfd fixtures in tests (`casement_testkit::fakes::FakeSession`); libseat from P1.

mod seam;

pub use seam::{LibseatSlot, SessionFault, SessionSeam};
