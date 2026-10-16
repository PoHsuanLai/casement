//! The floor as text: `/etc/quire/privacy.toml` and `/etc/quire/privacy.d/*.toml`. A pure
//! codec, strict because an administrator's file should fail loudly: a wrong word, a default
//! of "allow" or a bad subject is an error, not a dropped row.
//!
//! ```toml
//! [default]
//! screen = "ask"
//! lock   = "deny"
//! [never_remember]
//! programs = ["/usr/bin/python3.13"]
//! [[rule]]
//! power   = "clipboard"
//! subject = { program = "/usr/bin/cliphist" }
//! answer  = "allow"
//! ```

use super::book::Verdict;
use super::floor::{Fallback, Floor, FloorRule};
use super::limits::{MAX_FILE_BYTES, MAX_FLOOR_RULES, MAX_NEVER_REMEMBER};
use super::subject::ProgramPath;
use super::subject_dto::SubjectDto;
use casement_protocols::vocab::Power;
use serde::Deserialize;

/// Why the floor's text was refused.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FloorError {
    /// The text is larger than a floor file can be.
    #[error("the floor file is larger than 64 KiB")]
    TooLarge,
    /// The text is not this shape.
    #[error("the floor file: {0}")]
    Parse(#[from] toml::de::Error),
    /// A default says "allow": a floor cannot let in a program nobody named.
    #[error("the floor file: the default for {0:?} cannot be \"allow\"")]
    DefaultAllow(Power),
    /// A subject or a program path is not well formed.
    #[error("the floor file: a program or app is not well formed")]
    Subject,
    /// There are more rules or programs than a floor holds.
    #[error("the floor file has too many rules or programs")]
    TooMany,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Stated {
    Ask,
    Deny,
    Allow,
}

#[derive(Deserialize, Default)]
struct Defaults {
    screen: Option<Stated>,
    keys: Option<Stated>,
    typing_help: Option<Stated>,
    clipboard: Option<Stated>,
    lock: Option<Stated>,
    displays: Option<Stated>,
    windows: Option<Stated>,
}

impl Defaults {
    fn entries(self) -> [(Power, Option<Stated>); 7] {
        [
            (Power::Screen, self.screen),
            (Power::Keys, self.keys),
            (Power::TypingHelp, self.typing_help),
            (Power::Clipboard, self.clipboard),
            (Power::Lock, self.lock),
            (Power::Displays, self.displays),
            (Power::Windows, self.windows),
        ]
    }
}

#[derive(Deserialize, Default)]
struct Never {
    #[serde(default)]
    programs: Vec<String>,
}

#[derive(Deserialize)]
struct RuleDto {
    power: Power,
    answer: Verdict,
    subject: SubjectDto,
}

#[derive(Deserialize, Default)]
struct InFloor {
    #[serde(default, rename = "default")]
    defaults: Defaults,
    #[serde(default)]
    never_remember: Never,
    #[serde(default)]
    rule: Vec<RuleDto>,
}

impl Floor {
    /// The floor `text` holds; an empty text is the empty floor, keys it does not know are
    /// ignored.
    pub fn from_toml(text: &str) -> Result<Floor, FloorError> {
        if text.len() > MAX_FILE_BYTES {
            return Err(FloorError::TooLarge);
        }
        let file: InFloor = toml::from_str(text)?;
        if file.rule.len() > MAX_FLOOR_RULES
            || file.never_remember.programs.len() > MAX_NEVER_REMEMBER
        {
            return Err(FloorError::TooMany);
        }
        let floor = file.defaults.entries().into_iter().try_fold(
            Floor::empty(),
            |floor, (power, stated)| match stated {
                None => Ok(floor),
                Some(Stated::Ask) => Ok(floor.with_fallback(power, Fallback::Ask)),
                Some(Stated::Deny) => Ok(floor.with_fallback(power, Fallback::Deny)),
                Some(Stated::Allow) => Err(FloorError::DefaultAllow(power)),
            },
        )?;
        let floor = file
            .never_remember
            .programs
            .iter()
            .try_fold(floor, |floor, program| {
                ProgramPath::parse(program)
                    .map(|path| floor.with_never_remembered(path))
                    .map_err(|_| FloorError::Subject)
            })?;
        file.rule.into_iter().try_fold(floor, |floor, rule| {
            let subject = rule.subject.subject().ok_or(FloorError::Subject)?;
            Ok(floor.with_rule(FloorRule {
                power: rule.power,
                subject,
                verdict: rule.answer,
            }))
        })
    }
}
