//! The person's book: the answers they chose to keep, one per program and power. The file it
//! is stored in is the composition root's seam; the text codec is in `book_codec.rs`.

use super::limits::MAX_GRANTS;
use super::subject::Subject;
use casement_protocols::vocab::Power;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A wall-clock instant in whole seconds since 1970, as the edge reads it. Core never reads a
/// clock: the stamp on a grant is only shown in Settings and used to drop the oldest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnixSeconds(pub u64);

/// What a stored answer says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The program may use the power.
    Allow,
    /// It may not.
    Deny,
}

/// One stored answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    /// The power.
    pub power: Power,
    /// The program or app.
    pub subject: Subject,
    /// What the person said.
    pub verdict: Verdict,
    /// When.
    pub at: UnixSeconds,
}

/// Every answer the person kept. At most [`MAX_GRANTS`]; the oldest makes room.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Book {
    grants: BTreeMap<(Power, Subject), (Verdict, UnixSeconds)>,
}

impl Book {
    /// A book with no answers.
    pub fn empty() -> Book {
        Book::default()
    }

    /// What the person said about `subject` using `power`, if they did.
    pub fn get(&self, subject: &Subject, power: Power) -> Option<Verdict> {
        self.grants
            .get(&(power, subject.clone()))
            .map(|(verdict, _)| *verdict)
    }

    /// The book with `grant` in it, replacing any answer for the same program and power. A
    /// full book drops its oldest other answer first.
    pub fn with(self, grant: Grant) -> Book {
        let key = (grant.power, grant.subject);
        let mut grants = self.grants;
        grants.insert(key.clone(), (grant.verdict, grant.at));
        if grants.len() > MAX_GRANTS {
            let oldest = grants
                .iter()
                .filter(|(other, _)| **other != key)
                .min_by_key(|(_, (_, at))| *at)
                .map(|(other, _)| other.clone());
            if let Some(oldest) = oldest {
                grants.remove(&oldest);
            }
        }
        Book { grants }
    }

    /// The book without the answer for `subject` and `power`.
    pub fn without(self, subject: &Subject, power: Power) -> Book {
        let mut grants = self.grants;
        grants.remove(&(power, subject.clone()));
        Book { grants }
    }

    /// The book with only the answers `keep` says yes to (the start-up pruning of programs
    /// that are no longer on disk).
    pub fn retained(self, mut keep: impl FnMut(&Grant) -> bool) -> Book {
        let grants = self
            .grants
            .into_iter()
            .filter(|((power, subject), (verdict, at))| {
                keep(&Grant {
                    power: *power,
                    subject: subject.clone(),
                    verdict: *verdict,
                    at: *at,
                })
            })
            .collect();
        Book { grants }
    }

    /// Every answer, in power then program order.
    pub fn grants(&self) -> Vec<Grant> {
        self.grants
            .iter()
            .map(|((power, subject), (verdict, at))| Grant {
                power: *power,
                subject: subject.clone(),
                verdict: *verdict,
                at: *at,
            })
            .collect()
    }

    /// How many answers the book holds.
    pub fn len(&self) -> usize {
        self.grants.len()
    }

    /// Whether the book holds no answer.
    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}
