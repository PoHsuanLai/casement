//! The Privacy list (Settings > Privacy): which program may use which power, served for
//! Settings as a live settings module on casement's own bus connection.
//!
//! The model owns the answers ([`Privilege`](casement_core::privilege::Privilege)); the
//! composition root creates a [`PrivacyLink`], gives the bus thread one half (through
//! `CallerSetup::with_privacy`), publishes the model's rows to the [`PrivacyBoard`] after every
//! step that changes them, and turns each [`PrivacyEdit`] the returned channel yields into an
//! event (`PrivacyEdit::into_event`, stamped with the wall-clock second).

mod board;
mod edit;
mod keys;
mod module;
mod write;

pub use board::{EDIT_QUEUE, PrivacyBoard, PrivacyLink};
pub use edit::PrivacyEdit;
pub use keys::{
    Listed, NOBODY_ASKED, OFF, ON, PRIVACY_PAGE, SET_BY_ADMINISTRATOR, Why, Write, listed,
};
pub use module::{PRIVACY_PATH, PrivacyModule};

pub(crate) use module::serve_privacy;

#[cfg(test)]
mod tests;
