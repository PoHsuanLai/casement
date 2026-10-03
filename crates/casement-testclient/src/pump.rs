//! Pumping a client.

/// Whether a pump moved anything (the fixture pumps until every party is quiet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pumped {
    /// Something was read, dispatched or sent.
    Moved,
    /// Nothing happened.
    Quiet,
}

impl Pumped {
    /// `Moved` if either side moved: the fixture folds every party's pump into one answer.
    pub fn or(self, other: Pumped) -> Pumped {
        match (self, other) {
            (Pumped::Quiet, Pumped::Quiet) => Pumped::Quiet,
            _ => Pumped::Moved,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_party_moving_is_the_round_moving() {
        assert_eq!(Pumped::Quiet.or(Pumped::Quiet), Pumped::Quiet);
        assert_eq!(Pumped::Quiet.or(Pumped::Moved), Pumped::Moved);
        assert_eq!(Pumped::Moved.or(Pumped::Quiet), Pumped::Moved);
    }
}
