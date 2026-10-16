//! The administrator's floor: what an unlisted program gets, programs that are never
//! remembered, and pre-approvals or refusals by program. Read-only for casement; the text
//! codec is in `floor_codec.rs`.

use super::book::Verdict;
use super::limits::{MAX_FLOOR_RULES, MAX_NEVER_REMEMBER};
use super::subject::{ProgramPath, Subject};
use casement_protocols::vocab::{Memory, Power};
use std::collections::{BTreeMap, BTreeSet};

/// What a program nobody has answered for gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    /// It is asked.
    Ask,
    /// It is refused without being asked. (There is no "allow": a floor cannot let in a
    /// program nobody named.)
    Deny,
}

/// One pre-approval or refusal by the administrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorRule {
    /// The power.
    pub power: Power,
    /// The program or app.
    pub subject: Subject,
    /// What the administrator decided.
    pub verdict: Verdict,
}

/// How a program's file name is matched against the built-in list of interpreters.
#[derive(Debug, Clone, Copy)]
enum Name {
    /// The whole name.
    Exact(&'static str),
    /// A name that starts so (`python3.13`).
    Prefix(&'static str),
}

/// Programs that run whatever script they are given: allowing one allows every script, so an
/// answer about one is never kept. An administrator adds to this list in the floor.
const INTERPRETERS: [Name; 13] = [
    Name::Prefix("python"),
    Name::Exact("perl"),
    Name::Exact("ruby"),
    Name::Exact("node"),
    Name::Exact("deno"),
    Name::Exact("bun"),
    Name::Exact("java"),
    Name::Prefix("lua"),
    Name::Exact("sh"),
    Name::Exact("bash"),
    Name::Exact("zsh"),
    Name::Exact("fish"),
    Name::Prefix("wine"),
];

fn is_interpreter(path: &ProgramPath) -> bool {
    let file = path.file_name();
    INTERPRETERS.iter().any(|name| match name {
        Name::Exact(exact) => file == *exact,
        Name::Prefix(prefix) => file.starts_with(*prefix),
    })
}

/// The administrator's floor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Floor {
    fallbacks: BTreeMap<Power, Fallback>,
    never: BTreeSet<ProgramPath>,
    rules: Vec<FloorRule>,
}

impl Floor {
    /// No floor: everything unlisted is asked.
    pub fn empty() -> Floor {
        Floor::default()
    }

    /// The floor with `fallback` for `power`.
    pub fn with_fallback(self, power: Power, fallback: Fallback) -> Floor {
        let mut fallbacks = self.fallbacks;
        fallbacks.insert(power, fallback);
        Floor { fallbacks, ..self }
    }

    /// The floor with `rule` (ignored when the floor already holds [`MAX_FLOOR_RULES`]).
    pub fn with_rule(self, rule: FloorRule) -> Floor {
        if self.rules.len() >= MAX_FLOOR_RULES || self.rules.contains(&rule) {
            return self;
        }
        let rules = self.rules.into_iter().chain([rule]).collect();
        Floor { rules, ..self }
    }

    /// The floor that never remembers answers about `program` (ignored when it already names
    /// [`MAX_NEVER_REMEMBER`]).
    pub fn with_never_remembered(self, program: ProgramPath) -> Floor {
        if self.never.len() >= MAX_NEVER_REMEMBER {
            return self;
        }
        let mut never = self.never;
        never.insert(program);
        Floor { never, ..self }
    }

    /// What an unlisted program gets for `power`.
    pub fn fallback(&self, power: Power) -> Fallback {
        self.fallbacks.get(&power).copied().unwrap_or(Fallback::Ask)
    }

    /// What the administrator decided about `subject` and `power`: a refusal wins over a
    /// pre-approval when the floor holds both.
    pub fn rule(&self, subject: &Subject, power: Power) -> Option<Verdict> {
        self.rules
            .iter()
            .filter(|rule| rule.power == power && rule.subject == *subject)
            .map(|rule| rule.verdict)
            .fold(None, |found, verdict| match (found, verdict) {
                (Some(Verdict::Deny), _) | (_, Verdict::Deny) => Some(Verdict::Deny),
                _ => Some(Verdict::Allow),
            })
    }

    /// Whether an answer about `subject` is kept ([`Memory::Remembered`]) or lasts for one
    /// connection ([`Memory::ThisTimeOnly`]): never for an interpreter or a program the floor
    /// names.
    pub fn remembers(&self, subject: &Subject) -> Memory {
        match subject {
            Subject::Program(path) if is_interpreter(path) || self.never.contains(path) => {
                Memory::ThisTimeOnly
            }
            Subject::Program(_) | Subject::Sandboxed { .. } => Memory::Remembered,
        }
    }

    /// Every pre-approval and refusal.
    pub fn rules(&self) -> &[FloorRule] {
        &self.rules
    }

    /// The floor of both: a refusing default wins, the programs never remembered and the rules
    /// of both are kept (up to the caps).
    pub fn merge(self, other: Floor) -> Floor {
        let fallbacks =
            other
                .fallbacks
                .into_iter()
                .fold(self.fallbacks.clone(), |all, (power, fallback)| {
                    let kept = match (all.get(&power), fallback) {
                        (Some(Fallback::Deny), _) | (_, Fallback::Deny) => Fallback::Deny,
                        _ => Fallback::Ask,
                    };
                    let mut all = all;
                    all.insert(power, kept);
                    all
                });
        let merged = Floor { fallbacks, ..self };
        let merged = other
            .never
            .into_iter()
            .fold(merged, Floor::with_never_remembered);
        other.rules.into_iter().fold(merged, Floor::with_rule)
    }
}
