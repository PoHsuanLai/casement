//! What Settings asked for in the Privacy list, as the model hears it: the person switched a
//! program's power on or off, or asked to be asked again.

use casement_core::Event;
use casement_core::policy::Power;
use casement_core::privilege::{PrivacyRow, Source, Subject, UnixSeconds, Verdict};

/// One change Settings made to the person's answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrivacyEdit {
    /// The program may use the power.
    Allow {
        /// The program or app.
        subject: Subject,
        /// The power.
        power: Power,
    },
    /// The program may not.
    Deny {
        /// The program or app.
        subject: Subject,
        /// The power.
        power: Power,
    },
    /// The answer is forgotten: the program is asked at its next use.
    Forget {
        /// The program or app.
        subject: Subject,
        /// The power.
        power: Power,
    },
}

impl PrivacyEdit {
    /// The event the model is told. `at` is the wall-clock second the edge read (it only stamps
    /// the book); casement-ipc reads no clock.
    pub fn into_event(self, at: UnixSeconds) -> Event {
        match self {
            PrivacyEdit::Allow { subject, power } => Event::PrivilegeGranted { subject, power, at },
            PrivacyEdit::Deny { subject, power } => Event::PrivilegeRevoked { subject, power, at },
            PrivacyEdit::Forget { subject, power } => Event::PrivilegeForgotten { subject, power },
        }
    }

    /// The list `rows` becomes when this edit is made: the person's row for the program and
    /// power changes its answer, or goes. The administrator's rows are never touched. The model
    /// publishes the real list a moment later; this one lets Settings read its own change back.
    pub fn applied(&self, rows: &[PrivacyRow]) -> Vec<PrivacyRow> {
        let (subject, power) = self.target();
        let ours = |row: &PrivacyRow| {
            row.source == Source::Person && row.power == power && row.subject == *subject
        };
        rows.iter()
            .filter(|row| !(ours(row) && matches!(self, PrivacyEdit::Forget { .. })))
            .map(|row| match (ours(row), self.verdict()) {
                (true, Some(verdict)) => PrivacyRow {
                    verdict,
                    ..row.clone()
                },
                _ => row.clone(),
            })
            .collect()
    }

    fn target(&self) -> (&Subject, Power) {
        match self {
            PrivacyEdit::Allow { subject, power }
            | PrivacyEdit::Deny { subject, power }
            | PrivacyEdit::Forget { subject, power } => (subject, *power),
        }
    }

    fn verdict(&self) -> Option<Verdict> {
        match self {
            PrivacyEdit::Allow { .. } => Some(Verdict::Allow),
            PrivacyEdit::Deny { .. } => Some(Verdict::Deny),
            PrivacyEdit::Forget { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::privilege::ProgramPath;

    fn subject(path: &str) -> Subject {
        Subject::Program(ProgramPath::parse(path).expect("a path"))
    }

    fn row(path: &str, power: Power, verdict: Verdict, source: Source) -> PrivacyRow {
        PrivacyRow {
            power,
            subject: subject(path),
            verdict,
            source,
            at: Some(UnixSeconds(5)),
        }
    }

    #[test]
    fn an_edit_becomes_the_event_the_model_hears() {
        let at = UnixSeconds(9);
        let (s, p) = (subject("/usr/bin/grim"), Power::Screen);
        let cases = [
            (
                PrivacyEdit::Allow {
                    subject: s.clone(),
                    power: p,
                },
                Event::PrivilegeGranted {
                    subject: s.clone(),
                    power: p,
                    at,
                },
            ),
            (
                PrivacyEdit::Deny {
                    subject: s.clone(),
                    power: p,
                },
                Event::PrivilegeRevoked {
                    subject: s.clone(),
                    power: p,
                    at,
                },
            ),
            (
                PrivacyEdit::Forget {
                    subject: s.clone(),
                    power: p,
                },
                Event::PrivilegeForgotten {
                    subject: s,
                    power: p,
                },
            ),
        ];
        for (edit, want) in cases {
            assert_eq!(edit.clone().into_event(at), want, "{edit:?}");
        }
    }

    #[test]
    fn an_edit_changes_only_the_persons_row_it_names() {
        let rows = vec![
            row(
                "/usr/bin/grim",
                Power::Screen,
                Verdict::Allow,
                Source::Person,
            ),
            row("/usr/bin/grim", Power::Keys, Verdict::Allow, Source::Person),
            row(
                "/usr/bin/grim",
                Power::Screen,
                Verdict::Deny,
                Source::Administrator,
            ),
        ];
        let deny = PrivacyEdit::Deny {
            subject: subject("/usr/bin/grim"),
            power: Power::Screen,
        };
        let after = deny.applied(&rows);
        assert_eq!(after.len(), 3);
        assert_eq!(after[0].verdict, Verdict::Deny);
        assert_eq!(after[1], rows[1], "another power");
        assert_eq!(after[2], rows[2], "the administrator's row");
        let forget = PrivacyEdit::Forget {
            subject: subject("/usr/bin/grim"),
            power: Power::Screen,
        };
        assert_eq!(
            forget.applied(&rows),
            vec![rows[1].clone(), rows[2].clone()]
        );
    }
}
