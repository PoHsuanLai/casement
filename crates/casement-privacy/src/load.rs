//! What reading and keeping the two files comes to: the policy over [`PrivacyStore`].
//!
//! A damaged book is kept under another name and an empty one is used, never a permissive one:
//! an answer lost is a question asked again. A floor file that is there but cannot be believed
//! (it does not parse, or is not the administrator's alone) leaves every power at "ask" at the
//! most, so a floor that fails is never more permissive than no floor at all; the other floor
//! files are still read, and the compositor keeps running.

use super::store::{FloorText, PrivacyStore};
use casement_core::privilege::{Book, Fallback, Floor, MAX_GRANTS, Subject, UnixSeconds};
use casement_protocols::vocab::{Power, WireCode};

/// The person's book as it is stored. `now` names the damaged book's new name.
pub fn load_book(store: &mut dyn PrivacyStore, now: UnixSeconds) -> Book {
    let text = match store.read_book() {
        Ok(None) => return Book::empty(),
        Ok(Some(text)) => text,
        Err(error) => {
            tracing::warn!(%error, "the privacy book could not be read; starting with an empty one");
            set_aside(store, now);
            return Book::empty();
        }
    };
    match Book::from_toml(&text) {
        Ok(loaded) => {
            if loaded.dropped > 0 {
                tracing::warn!(
                    dropped = loaded.dropped,
                    "some answers in the privacy book could not be read; those programs will be asked again"
                );
            }
            forgetting_gone(store, loaded.value)
        }
        Err(error) => {
            tracing::warn!(%error, "the privacy book is damaged; starting with an empty one");
            set_aside(store, now);
            Book::empty()
        }
    }
}

/// The book `book` is when it is full: without the answers about programs that are no longer
/// installed. A book with room keeps everything.
fn forgetting_gone(store: &dyn PrivacyStore, book: Book) -> Book {
    if book.len() < MAX_GRANTS {
        return book;
    }
    book.retained(|grant| match &grant.subject {
        Subject::Program(path) => store.program_exists(path.as_str()),
        Subject::Sandboxed { .. } => true,
    })
}

fn set_aside(store: &mut dyn PrivacyStore, now: UnixSeconds) {
    if let Err(error) = store.set_book_aside(now) {
        tracing::warn!(%error, "the damaged privacy book could not be kept aside");
    }
}

/// Keeps `book`.
pub fn save_book(store: &mut dyn PrivacyStore, book: &Book) {
    match book.to_toml() {
        Ok(text) => {
            if let Err(error) = store.write_book(&text) {
                tracing::warn!(%error, "the privacy book could not be saved; the answers last until casement stops");
            }
        }
        Err(error) => tracing::warn!(%error, "the privacy book could not be written out"),
    }
}

/// The administrator's floor as it is stored: every file that is believed, merged. If any file
/// is not believed, one line says which and why, and every power is at "ask" at the most.
pub fn load_floor(store: &mut dyn PrivacyStore) -> Floor {
    let mut refused: Vec<String> = Vec::new();
    let floor = store
        .read_floor()
        .into_iter()
        .fold(Floor::empty(), |floor, file| match floored(&file) {
            Ok(next) => floor.merge(next),
            Err(why) => {
                refused.push(why);
                floor
            }
        });
    if refused.is_empty() {
        return floor;
    }
    tracing::error!(
        files = %refused.join("; "),
        "the administrator's privacy settings could not be used; every program is asked about every power until they can"
    );
    floor.merge(asking_about_everything())
}

/// The floor one file says, or why it is not believed.
fn floored(file: &FloorText) -> Result<Floor, String> {
    let text = file.text.as_ref().map_err(|error| error.to_string())?;
    Floor::from_toml(text).map_err(|error| format!("{}: {error}", file.path.display()))
}

/// A floor that says "ask" for every power: the least a failed floor may leave.
fn asking_about_everything() -> Floor {
    <Power as WireCode>::ALL
        .iter()
        .copied()
        .fold(Floor::empty(), |floor, power| {
            floor.with_fallback(power, Fallback::Ask)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StoreError;
    use casement_core::privilege::{Grant, ProgramPath, Verdict};
    use std::path::PathBuf;

    /// A store held in memory.
    #[derive(Debug, Default)]
    struct Memory {
        book: Option<String>,
        floor: Vec<String>,
        aside: Vec<UnixSeconds>,
        gone: Vec<String>,
    }

    impl PrivacyStore for Memory {
        fn read_book(&mut self) -> Result<Option<String>, StoreError> {
            Ok(self.book.clone())
        }
        fn set_book_aside(&mut self, at: UnixSeconds) -> Result<(), StoreError> {
            self.aside.push(at);
            self.book = None;
            Ok(())
        }
        fn write_book(&mut self, text: &str) -> Result<(), StoreError> {
            self.book = Some(text.to_owned());
            Ok(())
        }
        fn read_floor(&mut self) -> Vec<FloorText> {
            self.floor
                .iter()
                .enumerate()
                .map(|(n, text)| FloorText {
                    path: PathBuf::from(format!("/etc/quire/{n}.toml")),
                    text: Ok(text.clone()),
                })
                .collect()
        }
        fn program_exists(&self, path: &str) -> bool {
            !self.gone.iter().any(|gone| gone == path)
        }
    }

    fn grant(path: &str, power: Power, verdict: Verdict, at: u64) -> Grant {
        Grant {
            power,
            subject: Subject::Program(ProgramPath::parse(path).expect("a path")),
            verdict,
            at: UnixSeconds(at),
        }
    }

    #[test]
    fn a_saved_book_is_read_back_and_a_damaged_one_is_set_aside() {
        let mut store = Memory::default();
        let book = Book::empty().with(grant("/usr/bin/grim", Power::Screen, Verdict::Allow, 7));
        save_book(&mut store, &book);
        assert_eq!(load_book(&mut store, UnixSeconds(9)), book);
        assert!(store.aside.is_empty());

        store.book = Some("version = 99\n".to_owned());
        assert_eq!(load_book(&mut store, UnixSeconds(9)), Book::empty());
        assert_eq!(store.aside, [UnixSeconds(9)]);
        store.book = Some("this is [not toml".to_owned());
        assert_eq!(load_book(&mut store, UnixSeconds(10)), Book::empty());
        assert_eq!(store.aside, [UnixSeconds(9), UnixSeconds(10)]);
    }

    #[test]
    fn a_full_book_forgets_programs_that_are_gone_and_a_roomy_one_forgets_nothing() {
        let rows = (0..MAX_GRANTS).map(|n| {
            grant(
                &format!("/usr/bin/tool{n}"),
                Power::Screen,
                Verdict::Allow,
                u64::try_from(n).unwrap_or(0),
            )
        });
        let full = rows.fold(Book::empty(), Book::with);
        let mut store = Memory::default();
        save_book(&mut store, &full);
        store.gone = vec!["/usr/bin/tool3".to_owned()];
        let loaded = load_book(&mut store, UnixSeconds(1));
        assert_eq!(loaded.len(), MAX_GRANTS - 1);
        let few = Book::empty().with(grant("/usr/bin/tool3", Power::Screen, Verdict::Allow, 1));
        save_book(&mut store, &few);
        assert_eq!(load_book(&mut store, UnixSeconds(1)), few);
    }

    #[test]
    fn a_floor_that_fails_is_never_more_permissive_than_none_and_the_others_still_count() {
        let denies = "[default]\nlock = \"deny\"\n";
        let mut store = Memory {
            floor: vec![
                denies.to_owned(),
                "[default]\nscreen = \"allow\"\n".to_owned(),
            ],
            ..Memory::default()
        };
        let floor = load_floor(&mut store);
        assert_eq!(
            floor.fallback(Power::Lock),
            Fallback::Deny,
            "the good file counts"
        );
        assert_eq!(
            floor.fallback(Power::Screen),
            Fallback::Ask,
            "the bad file does not"
        );
        let mut none = Memory::default();
        assert_eq!(load_floor(&mut none), Floor::empty());
    }
}
