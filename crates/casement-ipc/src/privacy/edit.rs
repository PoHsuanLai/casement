//! What Settings asked for in the Privacy list, as the model hears it: the person switched a
//! program's power on or off, or asked to be asked again. The change is the model's own
//! [`Edit`]; this module only shows Settings its own change before the model answers.

use casement_core::privilege::{Edit, PrivacyRow, Source};

/// The list `rows` becomes when `edit` is made: the person's row for the program and power
/// changes its answer, or goes; a program that had no answer for the power gets one (its
/// first). The administrator's rows are never touched. The model publishes the real list a
/// moment later; this one lets Settings read its own change back.
pub(super) fn applied(edit: &Edit, rows: &[PrivacyRow]) -> Vec<PrivacyRow> {
    let (subject, power) = edit.target();
    let ours = |row: &PrivacyRow| {
        row.source == Source::Person && row.power == power && row.subject == *subject
    };
    let first = (!rows.iter().any(ours))
        .then(|| edit.verdict())
        .flatten()
        .map(|verdict| PrivacyRow {
            power,
            subject: subject.clone(),
            verdict,
            source: Source::Person,
            at: None,
        });
    rows.iter()
        .filter(|row| !(ours(row) && matches!(edit, Edit::Forget(..))))
        .map(|row| match (ours(row), edit.verdict()) {
            (true, Some(verdict)) => PrivacyRow {
                verdict,
                ..row.clone()
            },
            _ => row.clone(),
        })
        .chain(first)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::policy::Power;
    use casement_core::privilege::{ProgramPath, Subject, UnixSeconds, Verdict};

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
    fn the_first_answer_for_a_power_adds_the_persons_row() {
        let rows = vec![row(
            "/usr/bin/grim",
            Power::Screen,
            Verdict::Allow,
            Source::Person,
        )];
        let allow = Edit::Allow(subject("/usr/bin/grim"), Power::Keys);
        let after = applied(&allow, &rows);
        assert_eq!(after.len(), 2);
        assert_eq!(after[0], rows[0]);
        assert_eq!(after[1].power, Power::Keys);
        assert_eq!(after[1].verdict, Verdict::Allow);
        assert_eq!(after[1].source, Source::Person);
        // Asking again about a power nobody answered has nothing to forget.
        let forget = Edit::Forget(subject("/usr/bin/grim"), Power::Keys);
        assert_eq!(applied(&forget, &rows), rows);
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
        let deny = Edit::Deny(subject("/usr/bin/grim"), Power::Screen);
        let after = applied(&deny, &rows);
        assert_eq!(after.len(), 3);
        assert_eq!(after[0].verdict, Verdict::Deny);
        assert_eq!(after[1], rows[1], "another power");
        assert_eq!(after[2], rows[2], "the administrator's row");
        let forget = Edit::Forget(subject("/usr/bin/grim"), Power::Screen);
        assert_eq!(
            applied(&forget, &rows),
            vec![rows[1].clone(), rows[2].clone()]
        );
    }
}
