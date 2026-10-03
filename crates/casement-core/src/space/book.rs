//! Every Space, and which one each output shows.

use super::Space;
use crate::ids::OutputKey;
use porter_core::SpaceId;
use std::collections::BTreeMap;

/// Every Space by id, and the Space each output shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceBook {
    spaces: BTreeMap<SpaceId, Space>,
    shown: BTreeMap<OutputKey, SpaceId>,
}

impl SpaceBook {
    /// No Spaces.
    pub fn empty() -> SpaceBook {
        SpaceBook {
            spaces: BTreeMap::new(),
            shown: BTreeMap::new(),
        }
    }

    /// The book with `space` added (or replaced, by id).
    pub fn with_space(self, space: Space) -> SpaceBook {
        let SpaceBook { mut spaces, shown } = self;
        spaces.insert(space.id.clone(), space);
        SpaceBook { spaces, shown }
    }

    /// The book with `output` showing `space`.
    pub fn with_shown(self, output: OutputKey, space: SpaceId) -> SpaceBook {
        let SpaceBook { spaces, mut shown } = self;
        shown.insert(output, space);
        SpaceBook { spaces, shown }
    }

    /// The book without the Space `id`, and without any output showing it.
    pub fn without_space(self, id: &SpaceId) -> SpaceBook {
        let SpaceBook { mut spaces, shown } = self;
        spaces.remove(id);
        let shown = shown.into_iter().filter(|(_, shown)| shown != id).collect();
        SpaceBook { spaces, shown }
    }

    /// The book with `output` showing nothing.
    pub fn without_shown(self, output: &OutputKey) -> SpaceBook {
        let SpaceBook { spaces, mut shown } = self;
        shown.remove(output);
        SpaceBook { spaces, shown }
    }

    /// Every Space, by id.
    pub fn iter(&self) -> impl Iterator<Item = &Space> {
        self.spaces.values()
    }

    /// The Space id each output shows, by output.
    pub fn shown(&self) -> impl Iterator<Item = (&OutputKey, &SpaceId)> {
        self.shown.iter()
    }

    /// The Space `output` shows.
    pub fn shown_on(&self, output: &OutputKey) -> Option<&Space> {
        self.shown.get(output).and_then(|id| self.get(id))
    }

    /// `output`'s Spaces in row order (by position; equal positions by id, so the order is
    /// total).
    pub fn row(&self, output: &OutputKey) -> Vec<&Space> {
        let mut row: Vec<&Space> = self
            .spaces
            .values()
            .filter(|space| space.output == *output)
            .collect();
        row.sort_by(|a, b| (a.position, &a.id).cmp(&(b.position, &b.id)));
        row
    }

    /// The Space with this id.
    pub fn get(&self, id: &SpaceId) -> Option<&Space> {
        self.spaces.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::OutputName;
    use crate::output::{EdidFacts, EdidSerial};
    use crate::space::{SpaceContent, SpaceIndex};

    fn output(name: &str) -> OutputKey {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse(name).expect("name"))
    }

    fn space(id: &str, on: &str, position: u32) -> Space {
        Space {
            id: SpaceId::parse(id).expect("id"),
            output: output(on),
            content: SpaceContent::Desktop,
            position: SpaceIndex(position),
        }
    }

    fn book() -> SpaceBook {
        SpaceBook::empty()
            .with_space(space("s-b", "HEADLESS-1", 1))
            .with_space(space("s-a", "HEADLESS-1", 0))
            .with_space(space("s-c", "HEADLESS-2", 0))
            .with_shown(output("HEADLESS-1"), SpaceId::parse("s-b").expect("id"))
    }

    #[test]
    fn queries_answer_from_the_book() {
        let book = book();
        let shown = book.shown_on(&output("HEADLESS-1")).map(|s| s.id.as_str());
        assert_eq!(shown, Some("s-b"));
        assert_eq!(book.shown_on(&output("HEADLESS-2")), None);
        let row: Vec<&str> = book
            .row(&output("HEADLESS-1"))
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(row, vec!["s-a", "s-b"]);
        assert_eq!(book.row(&output("HEADLESS-3")), Vec::<&Space>::new());
        let c = SpaceId::parse("s-c").expect("id");
        assert_eq!(book.get(&c), Some(&space("s-c", "HEADLESS-2", 0)));
        assert_eq!(book.get(&SpaceId::parse("s-z").expect("id")), None);
    }

    #[test]
    fn rows_break_ties_by_id_and_replacing_a_space_keeps_one_copy() {
        let book = SpaceBook::empty()
            .with_space(space("s-z", "DP-1", 0))
            .with_space(space("s-m", "DP-1", 0))
            .with_space(space("s-a", "DP-1", 0))
            .with_space(space("s-q", "DP-1", 2));
        let ids = |book: &SpaceBook| -> Vec<String> {
            book.row(&output("DP-1"))
                .iter()
                .map(|s| s.id.as_str().to_owned())
                .collect()
        };
        assert_eq!(ids(&book), ["s-a", "s-m", "s-z", "s-q"]);
        let moved = book.with_space(space("s-a", "DP-1", 3));
        assert_eq!(ids(&moved), ["s-m", "s-z", "s-q", "s-a"]);
    }

    #[test]
    fn removing_a_space_stops_it_being_shown() {
        let book = book().without_space(&SpaceId::parse("s-b").expect("id"));
        assert_eq!(book.shown_on(&output("HEADLESS-1")), None);
        assert_eq!(book.shown().count(), 0);
        let ids: Vec<&str> = book.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["s-a", "s-c"]);
        let book = self::book().without_shown(&output("HEADLESS-1"));
        assert_eq!(book.shown_on(&output("HEADLESS-1")), None);
        assert_eq!(book.iter().count(), 3);
    }

    #[test]
    fn a_shown_id_with_no_space_shows_nothing() {
        let book = SpaceBook::empty()
            .with_shown(output("DP-1"), SpaceId::parse("s-gone").expect("id"))
            .with_space(space("s-a", "DP-2", 0));
        assert_eq!(book.shown_on(&output("DP-1")), None);
        assert_eq!(
            book.shown_on(&output("DP-2")),
            None,
            "a Space exists but nothing says the output shows it"
        );
    }
}
