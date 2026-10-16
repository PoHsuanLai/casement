//! The answers as Settings lists them: the person's, and the administrator's.

use super::Privilege;
use super::book::{UnixSeconds, Verdict};
use super::subject::Subject;
use casement_protocols::vocab::Power;

/// Who set a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// The person, by answering a question or in Settings. They can change it.
    Person,
    /// The administrator's floor. It cannot be switched off in Settings.
    Administrator,
}

/// One program's answer for one power.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyRow {
    /// The power.
    pub power: Power,
    /// The program or app.
    pub subject: Subject,
    /// The answer.
    pub verdict: Verdict,
    /// Who set it.
    pub source: Source,
    /// When the person answered; none for the administrator's rows.
    pub at: Option<UnixSeconds>,
}

impl Privilege {
    /// Every row Settings lists, ordered by power, then program, then who set it.
    pub fn rows(&self) -> Vec<PrivacyRow> {
        let administrator = self.floor.rules().iter().map(|rule| PrivacyRow {
            power: rule.power,
            subject: rule.subject.clone(),
            verdict: rule.verdict,
            source: Source::Administrator,
            at: None,
        });
        let person = self.book.grants().into_iter().map(|grant| PrivacyRow {
            power: grant.power,
            subject: grant.subject,
            verdict: grant.verdict,
            source: Source::Person,
            at: Some(grant.at),
        });
        let mut rows: Vec<PrivacyRow> = person.chain(administrator).collect();
        rows.sort_by(|a, b| (a.power, &a.subject, a.source).cmp(&(b.power, &b.subject, b.source)));
        rows
    }
}
