//! What Settings did to one answer in the person's book.

use super::book::Verdict;
use super::subject::Subject;
use casement_protocols::vocab::Power;

/// One change Settings made to the person's answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// The program may use the power.
    Allow(Subject, Power),
    /// The program may not: every connected client of it loses the power at once.
    Deny(Subject, Power),
    /// The answer is forgotten ("Ask Again"): the program is asked at its next use, and a
    /// connected client of it loses the power now.
    Forget(Subject, Power),
}

impl Edit {
    /// The program or app and the power the edit is about.
    pub fn target(&self) -> (&Subject, Power) {
        match self {
            Edit::Allow(subject, power)
            | Edit::Deny(subject, power)
            | Edit::Forget(subject, power) => (subject, *power),
        }
    }

    /// The answer the edit leaves, none when it forgets one.
    pub fn verdict(&self) -> Option<Verdict> {
        match self {
            Edit::Allow(..) => Some(Verdict::Allow),
            Edit::Deny(..) => Some(Verdict::Deny),
            Edit::Forget(..) => None,
        }
    }
}
