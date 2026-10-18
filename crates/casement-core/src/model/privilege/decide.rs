//! What a program gets for a power, given the administrator's floor and the person's book.
//! Pure and total.
//!
//! First match wins: the floor's refusal, the book's refusal, the floor's pre-approval, the
//! book's allow, the floor's default for the power, then a question. A person can always say
//! no to a pre-approval; an administrator's no is never overruled.

use super::book::{Book, Verdict};
use super::floor::{Fallback, Floor};
use super::subject::Subject;
use casement_protocols::vocab::{Memory, Power};

/// What a program gets for a power.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// It may use the power.
    Open,
    /// It may not, and is not asked.
    Closed,
    /// It is asked; the answer is kept or lasts for the connection, by `memory`.
    Ask(Memory),
}

/// The two stored opinions about a program and a power.
struct Says {
    floor: Option<Verdict>,
    book: Option<Verdict>,
}

/// What the floor and the book say. A program that cannot be named has no entry in either; a
/// program whose answers are never kept is not looked up in the book (a hand-edited row
/// cannot make an interpreter a standing guest).
fn says(floor: &Floor, book: &Book, subject: Option<&Subject>, power: Power) -> Says {
    match subject {
        None => Says {
            floor: None,
            book: None,
        },
        Some(subject) => Says {
            floor: floor.rule(subject, power),
            book: match floor.remembers(subject) {
                Memory::Remembered => book.get(subject, power),
                Memory::ThisTimeOnly => None,
            },
        },
    }
}

/// How long an answer about this program lives.
fn memory_of(floor: &Floor, subject: Option<&Subject>) -> Memory {
    match subject {
        Some(subject) => floor.remembers(subject),
        None => Memory::ThisTimeOnly,
    }
}

/// What `subject` gets for `power`; a program that cannot be named (`None`) is asked about for
/// its connection only.
pub fn decide(floor: &Floor, book: &Book, subject: Option<&Subject>, power: Power) -> Decision {
    match says(floor, book, subject, power) {
        Says {
            floor: Some(Verdict::Deny),
            ..
        }
        | Says {
            book: Some(Verdict::Deny),
            ..
        } => Decision::Closed,
        Says {
            floor: Some(Verdict::Allow),
            ..
        }
        | Says {
            book: Some(Verdict::Allow),
            ..
        } => Decision::Open,
        Says {
            floor: None,
            book: None,
        } => match floor.fallback(power) {
            Fallback::Deny => Decision::Closed,
            Fallback::Ask => Decision::Ask(memory_of(floor, subject)),
        },
    }
}

/// What a program the desktop trusts for `power` gets: in unless the floor or the book
/// refuses it by name.
pub fn decide_trusted(
    floor: &Floor,
    book: &Book,
    subject: Option<&Subject>,
    power: Power,
) -> Decision {
    match says(floor, book, subject, power) {
        Says {
            floor: Some(Verdict::Deny),
            ..
        }
        | Says {
            book: Some(Verdict::Deny),
            ..
        } => Decision::Closed,
        _ => Decision::Open,
    }
}
