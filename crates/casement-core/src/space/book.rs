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

    /// The Space `output` shows.
    pub fn shown_on(&self, output: &OutputKey) -> Option<&Space> {
        let _ = (output, &self.spaces, &self.shown);
        todo!("P0-F1-window: the shown id, looked up")
    }

    /// `output`'s Spaces in row order.
    pub fn row(&self, output: &OutputKey) -> Vec<&Space> {
        let _ = output;
        todo!("P0-F1-window: the output's Spaces sorted by position")
    }

    /// The Space with this id.
    pub fn get(&self, id: &SpaceId) -> Option<&Space> {
        let _ = id;
        todo!("P0-F1-window: lookup")
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
    #[ignore = "P0-F1-window"]
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
}
