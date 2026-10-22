//! Every output's reveal machines.

use super::{Reveal, RevealEdge, RevealIn, RevealOut, RevealTiming};
use crate::ids::OutputKey;
use crate::time::Mono;
use std::collections::BTreeMap;

/// The machines of every output and edge. A machine that was never told anything is behind the
/// window and is not held.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RevealBook(BTreeMap<(OutputKey, RevealEdge), Reveal>);

impl RevealBook {
    /// No machines.
    pub fn new() -> RevealBook {
        RevealBook::default()
    }

    /// The machine of `edge` of `output`; a machine never told anything when there is none.
    pub fn get(&self, output: &OutputKey, edge: RevealEdge) -> Reveal {
        self.0
            .get(&(output.clone(), edge))
            .copied()
            .unwrap_or_default()
    }

    /// The book with the machine of `edge` of `output` stepped by `input` at `now`, and what the
    /// edge does about it.
    pub fn step(
        self,
        output: &OutputKey,
        edge: RevealEdge,
        timing: RevealTiming,
        input: RevealIn,
        now: Mono,
    ) -> (RevealBook, Vec<RevealOut>) {
        let current = self.get(output, edge);
        let RevealBook(mut machines) = self;
        let (next, out) = current.step(timing, input, now);
        machines.insert((output.clone(), edge), next);
        (RevealBook(machines), out)
    }

    /// The book without the machines of outputs `keep` refuses (an output that was unplugged).
    pub fn retain_outputs(self, keep: impl Fn(&OutputKey) -> bool) -> RevealBook {
        let RevealBook(mut machines) = self;
        machines.retain(|(output, _), _| keep(output));
        RevealBook(machines)
    }
}
