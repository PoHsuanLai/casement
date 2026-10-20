//! The keys of the Privacy module, as a pure function of the rows: a section for each power,
//! and under it a switch for each program that has been answered (`privacy.<power>.<n>`) with an
//! "Ask Again" action beside it (`privacy.<power>.<n>.forget`). A row the administrator set is a
//! read-out that says so; a section nobody has asked in says that.
//!
//! Every program the rows name also has a switch for each power it has no answer for, standing at
//! [`ASK`]: Settings can only change a row the module lists, and a program that has not asked
//! yet would otherwise have nothing to switch on. Setting such a row makes the person's first
//! answer, the same as answering the question would.

use super::write::{Why, Write};
use casement_core::policy::Power;
use casement_core::privilege::{Engine, MAX_GRANTS, PrivacyRow, Source, Subject, Verdict};
use casement_protocols::vocab::{SubjectKind, WireCode};
use ds_settings::schema::{
    ActionLabel, ActionWeight, AgentSetting, Exposure, Help, KeyKind, KeyPath, KeySpec, Label,
    LiveAction, Note, Page, Section, WordLabels,
};
use std::collections::BTreeSet;

/// The page the keys are listed on: Settings' Privacy page.
pub const PRIVACY_PAGE: Page = Page::Privacy;

/// The word of a switch that is on: the program may use the power.
pub const ON: &str = "on";

/// The word of a switch that is off.
pub const OFF: &str = "off";

/// The word of a switch nobody has answered: the program is asked at its next use. It is a third
/// resting place of a two-position switch, never one Settings sets (that is "Ask Again").
pub const ASK: &str = "ask";

/// The most programs that get a switch for every power: the book's own cap on answers, so the
/// list is at most seven switches (and the answered programs' actions) per program the person's
/// book can hold, and an administrator's long floor cannot make it longer.
const FILLED_PROGRAMS: usize = MAX_GRANTS;

/// What a section says when nobody has asked.
pub const NOBODY_ASKED: &str = "No program has asked yet.";

/// What a row the administrator set says.
pub const SET_BY_ADMINISTRATOR: &str = "Set by your administrator";

/// One key: what `Describe` lists, what `Get` answers and what `Set` may do.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    /// The key as the schema lists it.
    pub spec: KeySpec,
    /// Its value now.
    pub value: toml::Value,
    /// What `Set` may do with it.
    pub write: Write,
}

/// The section name of a power in a key path (the same words the vocabulary stores).
fn slug(power: Power) -> &'static str {
    match power {
        Power::Screen => "screen",
        Power::Keys => "keys",
        Power::TypingHelp => "typing_help",
        Power::Clipboard => "clipboard",
        Power::Lock => "lock",
        Power::Displays => "displays",
        Power::Windows => "windows",
    }
}

/// FNV-1a over `bytes`: a hash that is the same on every run and every machine, so a key path
/// names the same program every time.
fn fnv1a(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The short stable name of a row inside its power: twelve hex digits of the hash of who set it
/// and which program it is about.
fn short(source: Source, subject: &Subject) -> String {
    let who = match source {
        Source::Person => 0_u8,
        Source::Administrator => 1,
    };
    let kind = match subject.kind() {
        SubjectKind::Program => 0_u8,
        _ => 1,
    };
    let engine = match subject {
        Subject::Sandboxed {
            engine: Engine::Flatpak,
            ..
        } => 1_u8,
        Subject::Sandboxed {
            engine: Engine::Snap,
            ..
        } => 2,
        _ => 0,
    };
    let bytes = [who, kind, engine]
        .into_iter()
        .chain(subject.text().bytes());
    format!("{:012x}", fnv1a(bytes) & 0xffff_ffff_ffff)
}

fn word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => ON,
        Verdict::Deny => OFF,
    }
}

fn spec(path: String, power: Power, label: &str, kind: KeyKind, default: toml::Value) -> KeySpec {
    KeySpec {
        path: KeyPath(path),
        kind,
        default,
        label: Label(label.to_owned()),
        help: Help(power.blurb().to_owned()),
        page: PRIVACY_PAGE,
        section: Section(power.title().to_owned()),
        exposure: Exposure::Basic,
        labels: WordLabels(
            [
                (ON.to_owned(), "Allowed".to_owned()),
                (OFF.to_owned(), "Not allowed".to_owned()),
                (ASK.to_owned(), "Ask".to_owned()),
            ]
            .into_iter()
            .collect(),
        ),
        unavailable: Default::default(),
        groups: Default::default(),
        places: Default::default(),
        note: None,
        agent: AgentSetting::HandsOff,
        control: None,
    }
}

/// The switch of `subject` for `power`, standing at `standing`; `default` is where "reset"
/// would put it.
fn switch_at(
    base: &str,
    (power, subject): (Power, &Subject),
    standing: &str,
    default: &str,
) -> Listed {
    Listed {
        spec: spec(
            base.to_owned(),
            power,
            subject.text(),
            KeyKind::Toggle {
                variants: [ON.to_owned(), OFF.to_owned()],
            },
            toml::Value::String(default.to_owned()),
        ),
        value: toml::Value::String(standing.to_owned()),
        write: Write::Choose(subject.clone(), power),
    }
}

fn switch(base: &str, row: &PrivacyRow) -> Listed {
    switch_at(base, (row.power, &row.subject), word(row.verdict), OFF)
}

/// The switch of a program nobody has answered for `power`: it stands at [`ASK`].
fn unanswered(base: &str, power: Power, subject: &Subject) -> Listed {
    switch_at(base, (power, subject), ASK, ASK)
}

fn read_out(base: &str, row: &PrivacyRow) -> Listed {
    let on = word(row.verdict);
    let mut spec = spec(
        base.to_owned(),
        row.power,
        row.subject.text(),
        KeyKind::Fixed {
            variant: on.to_owned(),
        },
        toml::Value::String(on.to_owned()),
    );
    spec.note = Some(Note(SET_BY_ADMINISTRATOR.to_owned()));
    Listed {
        spec,
        value: toml::Value::String(on.to_owned()),
        write: Write::Refused(Why::Administrator),
    }
}

fn forget(base: &str, row: &PrivacyRow) -> Listed {
    Listed {
        spec: spec(
            format!("{base}.forget"),
            row.power,
            "Ask Again",
            KeyKind::Live {
                action: LiveAction {
                    label: ActionLabel("Ask Again".to_owned()),
                    weight: ActionWeight::Plain,
                },
            },
            toml::Value::Boolean(false),
        ),
        value: toml::Value::Boolean(false),
        write: Write::Forget(row.subject.clone(), row.power),
    }
}

fn nobody(power: Power) -> Listed {
    Listed {
        spec: spec(
            format!("privacy.{}.none", slug(power)),
            power,
            NOBODY_ASKED,
            KeyKind::Fixed {
                variant: String::new(),
            },
            toml::Value::String(String::new()),
        ),
        value: toml::Value::String(String::new()),
        write: Write::Refused(Why::ReadOut),
    }
}

/// The programs that get a switch for every power: those the person answered first, then the
/// administrator's, at most [`FILLED_PROGRAMS`].
fn filled(rows: &[PrivacyRow]) -> BTreeSet<&Subject> {
    [Source::Person, Source::Administrator]
        .into_iter()
        .flat_map(|source| rows.iter().filter(move |row| row.source == source))
        .map(|row| &row.subject)
        .fold(BTreeSet::new(), |mut chosen, subject| {
            if chosen.len() < FILLED_PROGRAMS {
                chosen.insert(subject);
            }
            chosen
        })
}

/// The keys of the module for `rows`: for each power, in the vocabulary's order, its rows (the
/// person's with a switch and "Ask Again", the administrator's as read-outs), then a switch at
/// [`ASK`] for each program in [`filled`] that has no row for it, or the line that says nobody
/// has asked. Two rows that would share a name (a hash collision, which needs 2^24 programs)
/// list the first only, so the schema never repeats a key.
pub fn listed(rows: &[PrivacyRow]) -> Vec<Listed> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let programs = filled(rows);
    for &power in <Power as WireCode>::ALL {
        let before = out.len();
        let answered: BTreeSet<&Subject> = rows
            .iter()
            .filter(|row| row.power == power)
            .map(|row| &row.subject)
            .collect();
        for row in rows.iter().filter(|row| row.power == power) {
            let base = format!(
                "privacy.{}.{}",
                slug(power),
                short(row.source, &row.subject)
            );
            if !seen.insert(base.clone()) {
                continue;
            }
            match row.source {
                Source::Person => {
                    out.push(switch(&base, row));
                    out.push(forget(&base, row));
                }
                Source::Administrator => out.push(read_out(&base, row)),
            }
        }
        for subject in programs.difference(&answered).copied() {
            let base = format!("privacy.{}.{}", slug(power), short(Source::Person, subject));
            if seen.insert(base.clone()) {
                out.push(unanswered(&base, power, subject));
            }
        }
        if out.len() == before {
            out.push(nobody(power));
        }
    }
    out
}

/// What changed between two lists, as the `Changed` signals Settings needs: every key whose
/// value differs or is new with its value, and every key that is gone with `removed`.
pub fn changes(before: &[PrivacyRow], after: &[PrivacyRow]) -> Vec<(KeyPath, toml::Value)> {
    let old = listed(before);
    let new = listed(after);
    let value_in = |keys: &[Listed], path: &KeyPath| {
        keys.iter()
            .find(|key| key.spec.path == *path)
            .map(|key| key.value.clone())
    };
    let moved = new
        .iter()
        .filter(|key| value_in(&old, &key.spec.path).as_ref() != Some(&key.value))
        .map(|key| (key.spec.path.clone(), key.value.clone()));
    let gone = old
        .iter()
        .filter(|key| value_in(&new, &key.spec.path).is_none())
        .map(|key| {
            (
                key.spec.path.clone(),
                toml::Value::String("removed".to_owned()),
            )
        });
    moved.chain(gone).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::privilege::{ProgramPath, SandboxAppId, UnixSeconds};

    fn program(path: &str) -> Subject {
        Subject::Program(ProgramPath::parse(path).expect("a path"))
    }

    fn row(subject: Subject, power: Power, verdict: Verdict, source: Source) -> PrivacyRow {
        PrivacyRow {
            power,
            subject,
            verdict,
            source,
            at: match source {
                Source::Person => Some(UnixSeconds(1)),
                Source::Administrator => None,
            },
        }
    }

    #[test]
    fn every_power_has_a_section_and_an_empty_one_says_so() {
        let keys = listed(&[]);
        assert_eq!(keys.len(), <Power as WireCode>::ALL.len());
        assert!(keys.iter().all(|key| key.spec.label.0 == NOBODY_ASKED));
        assert!(
            keys.iter()
                .all(|key| key.write == Write::Refused(Why::ReadOut))
        );
    }

    #[test]
    fn every_key_is_on_the_privacy_page() {
        let rows = [row(
            program("/usr/bin/grim"),
            Power::Screen,
            Verdict::Allow,
            Source::Person,
        )];
        let keys = listed(&rows);
        assert!(keys.len() > rows.len());
        assert!(keys.iter().all(|key| key.spec.page == Page::Privacy));
    }

    /// The switches of `power` as (program, word) pairs.
    fn standing_of(keys: &[Listed], power: Power) -> Vec<(String, String)> {
        keys.iter()
            .filter(|key| key.spec.section.0 == power.title())
            .filter(|key| matches!(key.spec.kind, KeyKind::Toggle { .. }))
            .map(|key| {
                let word = match &key.value {
                    toml::Value::String(word) => word.clone(),
                    other => panic!("a word: {other:?}"),
                };
                (key.spec.label.0.clone(), word)
            })
            .collect()
    }

    #[test]
    fn a_program_has_a_switch_for_every_power_and_the_unanswered_ones_stand_at_ask() {
        let grim = program("/usr/bin/grim");
        let rows = [row(
            grim.clone(),
            Power::Screen,
            Verdict::Allow,
            Source::Person,
        )];
        let keys = listed(&rows);
        for &power in <Power as WireCode>::ALL {
            let want = match power {
                Power::Screen => ON,
                _ => ASK,
            };
            assert_eq!(
                standing_of(&keys, power),
                [("/usr/bin/grim".to_owned(), want.to_owned())],
                "{power:?}"
            );
        }
        // Only the answered one has an "Ask Again" action; no section says nobody asked.
        let forgets = keys
            .iter()
            .filter(|key| key.spec.path.0.ends_with(".forget"));
        assert_eq!(forgets.count(), 1);
        assert!(keys.iter().all(|key| key.spec.label.0 != NOBODY_ASKED));
        // A Set on an unanswered row is the person's first answer for that power.
        let keys_row = keys
            .iter()
            .find(|key| key.spec.section.0 == Power::Keys.title())
            .expect("a Keys switch");
        assert_eq!(keys_row.write, Write::Choose(grim, Power::Keys));
        assert_eq!(keys_row.spec.default, toml::Value::String(ASK.to_owned()));
        let paths: BTreeSet<&str> = keys.iter().map(|key| key.spec.path.0.as_str()).collect();
        assert_eq!(paths.len(), keys.len(), "no key twice");
    }

    #[test]
    fn answering_a_power_gives_the_row_the_path_it_had_while_unanswered() {
        let grim = program("/usr/bin/grim");
        let base = [row(
            grim.clone(),
            Power::Screen,
            Verdict::Allow,
            Source::Person,
        )];
        let before = listed(&base);
        let mut after_rows = base.to_vec();
        after_rows.push(row(grim, Power::Keys, Verdict::Deny, Source::Person));
        let after = listed(&after_rows);
        let path_in = |keys: &[Listed]| {
            keys.iter()
                .find(|key| key.spec.section.0 == Power::Keys.title())
                .map(|key| (key.spec.path.clone(), key.value.clone()))
                .expect("a Keys switch")
        };
        let (was, asking) = path_in(&before);
        let (is, answered) = path_in(&after);
        assert_eq!(was, is, "the same key, so Settings is told its value moved");
        assert_eq!(asking, toml::Value::String(ASK.to_owned()));
        assert_eq!(answered, toml::Value::String(OFF.to_owned()));
        assert_eq!(
            changes(&base, &after_rows).len(),
            2,
            "the value and the new action"
        );
    }

    #[test]
    fn the_programs_with_a_switch_for_every_power_are_capped_and_the_persons_come_first() {
        let many = FILLED_PROGRAMS + 40;
        let mut rows: Vec<PrivacyRow> = (0..many)
            .map(|n| {
                row(
                    program(&format!("/opt/admin/p{n:04}")),
                    Power::Screen,
                    Verdict::Deny,
                    Source::Administrator,
                )
            })
            .collect();
        // Sorts after every administrator program, so only "persons first" keeps it.
        let theirs = program("/usr/bin/zzz");
        rows.push(row(
            theirs.clone(),
            Power::Windows,
            Verdict::Allow,
            Source::Person,
        ));
        let keys = listed(&rows);
        let asking = standing_of(&keys, Power::Keys);
        assert_eq!(asking.len(), FILLED_PROGRAMS);
        assert!(asking.iter().any(|(program, _)| program == "/usr/bin/zzz"));
        // The answered rows are never dropped by the cap.
        let read_outs = keys
            .iter()
            .filter(|key| key.spec.section.0 == Power::Screen.title())
            .filter(|key| matches!(key.spec.kind, KeyKind::Fixed { .. }))
            .count();
        assert_eq!(read_outs, many);
    }

    #[test]
    fn a_persons_row_is_a_switch_with_ask_again_and_an_administrators_is_a_read_out() {
        let grim = program("/usr/bin/grim");
        let rows = [
            row(grim.clone(), Power::Screen, Verdict::Allow, Source::Person),
            row(
                program("/usr/bin/foo"),
                Power::Screen,
                Verdict::Deny,
                Source::Administrator,
            ),
        ];
        let keys = listed(&rows);
        let screen: Vec<&Listed> = keys
            .iter()
            .filter(|key| key.spec.section.0 == Power::Screen.title())
            .collect();
        assert_eq!(screen.len(), 3, "a switch, its action and a read-out");
        assert!(matches!(screen[0].spec.kind, KeyKind::Toggle { .. }));
        assert_eq!(screen[0].value, toml::Value::String(ON.to_owned()));
        assert_eq!(screen[0].write, Write::Choose(grim.clone(), Power::Screen));
        assert!(screen[1].spec.path.0.ends_with(".forget"));
        assert_eq!(screen[1].write, Write::Forget(grim, Power::Screen));
        assert!(matches!(screen[2].spec.kind, KeyKind::Fixed { .. }));
        assert_eq!(screen[2].write, Write::Refused(Why::Administrator));
        assert_eq!(
            screen[2].spec.note.as_ref().map(|note| note.0.as_str()),
            Some(SET_BY_ADMINISTRATOR)
        );
    }

    #[test]
    fn the_names_are_stable_distinct_and_never_repeat_in_a_schema() {
        let grim = program("/usr/bin/grim");
        let same_both_ways = [
            row(grim.clone(), Power::Screen, Verdict::Allow, Source::Person),
            row(
                grim.clone(),
                Power::Screen,
                Verdict::Deny,
                Source::Administrator,
            ),
            row(
                Subject::Sandboxed {
                    app: SandboxAppId::parse("org.gnome.Loupe").expect("an app"),
                    engine: Engine::Flatpak,
                },
                Power::Clipboard,
                Verdict::Allow,
                Source::Person,
            ),
        ];
        let keys = listed(&same_both_ways);
        let paths: BTreeSet<&str> = keys.iter().map(|key| key.spec.path.0.as_str()).collect();
        assert_eq!(paths.len(), keys.len(), "no key twice");
        // The hash is FNV-1a: the same on every run, so a name always means the same program.
        assert_eq!(fnv1a([]), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(*b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(short(Source::Person, &grim), short(Source::Person, &grim));
        assert_ne!(
            short(Source::Person, &grim),
            short(Source::Administrator, &grim)
        );
        assert_eq!(short(Source::Person, &grim).len(), 12);
    }

    #[test]
    fn what_changed_is_what_settings_is_told() {
        let grim = program("/usr/bin/grim");
        let allowed = [row(
            grim.clone(),
            Power::Screen,
            Verdict::Allow,
            Source::Person,
        )];
        let denied = [row(grim, Power::Screen, Verdict::Deny, Source::Person)];
        let moved = changes(&allowed, &denied);
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].1, toml::Value::String(OFF.to_owned()));
        let forgotten = changes(&allowed, &[]);
        assert!(
            forgotten
                .iter()
                .any(|(_, value)| *value == toml::Value::String("removed".to_owned()))
        );
        assert!(changes(&allowed, &allowed).is_empty());
    }
}
