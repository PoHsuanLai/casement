//! The privacy list as `casementctl privacy list` prints it: the person's answers and the
//! administrator's, as the two files say them. It only reads, never sets a damaged book aside,
//! and says in plain words what it could not read.

use super::store::PrivacyStore;
use casement_core::privilege::{Book, Floor, PrivacyRow, Source, Subject, Verdict};
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use casement_core::time::Mono;
use casement_core::{Event, Model};
use casement_protocols::vocab::{Power, WireCode};
use std::fmt::Write;
use std::time::Duration;

/// What a power's section says when no program has an answer for it (Settings says the same).
pub const NOBODY_ASKED: &str = "No program has asked yet.";

/// What the two files say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    rows: Vec<PrivacyRow>,
    notes: Vec<String>,
}

impl Listing {
    /// The answers, by power, then program, with the person's before the administrator's.
    pub fn rows(&self) -> &[PrivacyRow] {
        &self.rows
    }

    /// What could not be read, one sentence each.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// The list as text: a heading per power and a line per program.
    pub fn render(&self) -> String {
        <Power as WireCode>::ALL
            .iter()
            .copied()
            .fold(String::new(), |mut text, power| {
                let _ = writeln!(text, "{}", power.title());
                let mine: Vec<&PrivacyRow> =
                    self.rows.iter().filter(|row| row.power == power).collect();
                if mine.is_empty() {
                    let _ = writeln!(text, "  {NOBODY_ASKED}");
                }
                for row in mine {
                    let _ = writeln!(
                        text,
                        "  {}: {} ({})",
                        subject_words(&row.subject),
                        answer_words(row.verdict),
                        source_words(row.source)
                    );
                }
                text
            })
    }
}

fn subject_words(subject: &Subject) -> String {
    match subject {
        Subject::Program(path) => path.as_str().to_owned(),
        Subject::Sandboxed { app, engine } => {
            format!("{} ({engine:?})", app.as_str())
        }
    }
}

fn answer_words(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "Allowed",
        Verdict::Deny => "Not allowed",
    }
}

fn source_words(source: Source) -> &'static str {
    match source {
        Source::Person => "you chose this",
        Source::Administrator => "set by your administrator",
    }
}

/// Reads `store` and lists what it holds.
pub fn list(store: &mut dyn PrivacyStore) -> Listing {
    let mut notes = Vec::new();
    let book = match store.read_book() {
        Ok(None) => Book::empty(),
        Ok(Some(text)) => match Book::from_toml(&text) {
            Ok(loaded) => {
                if loaded.dropped > 0 {
                    notes.push(format!(
                        "{} answers in your privacy file could not be read.",
                        loaded.dropped
                    ));
                }
                loaded.value
            }
            Err(error) => {
                notes.push(format!("Your privacy answers could not be read: {error}."));
                Book::empty()
            }
        },
        Err(error) => {
            notes.push(format!("Your privacy answers could not be read: {error}."));
            Book::empty()
        }
    };
    let floor = store
        .read_floor()
        .into_iter()
        .fold(Floor::empty(), |floor, file| {
            let parsed = file
                .text
                .map_err(|error| error.to_string())
                .and_then(|text| Floor::from_toml(&text).map_err(|error| error.to_string()));
            match parsed {
                Ok(next) => floor.merge(next),
                Err(why) => {
                    notes.push(format!(
                        "Your administrator's settings in {} are not used: {why}.",
                        file.path.display()
                    ));
                    floor
                }
            }
        });
    Listing {
        rows: rows_of(book, floor),
        notes,
    }
}

/// The rows the model lists for `book` under `floor` (the same ones Settings shows).
fn rows_of(book: Book, floor: Floor) -> Vec<PrivacyRow> {
    let now = Mono(Duration::ZERO);
    let model = Model::new(
        CasementSettings::defaults(),
        now,
        SpaceSeed::from_entropy([0; 16]),
    );
    let (model, _) = [
        Event::FloorLoaded(Box::new(floor)),
        Event::BookLoaded(Box::new(book)),
    ]
    .into_iter()
    .fold((model, Vec::new()), |(model, mut effects), event| {
        let (model, more) = model.step(event, now);
        effects.extend(more);
        (model, effects)
    });
    model.privilege().rows()
}
