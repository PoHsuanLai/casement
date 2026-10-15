//! A binding's action as the document spells it: one list of words, a single value to ds-settings'
//! key-by-key reader (a table would be read, and refused, a key at a time).
//!
//! ```text
//! ["spawn", "foot", "--title", "x"]                       a program and its arguments
//! ["call", SERVICE, PATH, INTERFACE, METHOD, ARG...]      a session-bus method call
//! ["desktop", "close_key"]                                 close_key, minimise_key, hide_active,
//!                                                          hide_others, quit_active, force_quit_panel,
//!                                                          mission_control
//! ["desktop", "switch_space", "next"]                      previous, next or the Space's number
//! ["desktop", "place", "left"]                             a `PlaceTarget` word
//! ```

use casement_core::CoreError;
use casement_core::input::{Argv, BindingAction, DbusCall, DesktopCommand, SpaceStep};
use casement_protocols::vocab::PlaceTarget;
use serde::{Deserialize, Serialize};

/// The words of one action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionWords(pub Vec<String>);

/// The place targets and the word each is written as.
const PLACE_TARGETS: [(&str, PlaceTarget); 11] = [
    ("left", PlaceTarget::Left),
    ("right", PlaceTarget::Right),
    ("top", PlaceTarget::Top),
    ("bottom", PlaceTarget::Bottom),
    ("top_left", PlaceTarget::TopLeft),
    ("top_right", PlaceTarget::TopRight),
    ("bottom_left", PlaceTarget::BottomLeft),
    ("bottom_right", PlaceTarget::BottomRight),
    ("fill", PlaceTarget::Fill),
    ("centre", PlaceTarget::Centre),
    ("restore", PlaceTarget::Restore),
];

/// The unit commands and the word each is written as.
const UNIT_COMMANDS: [(&str, DesktopCommand); 7] = [
    ("close_key", DesktopCommand::CloseKey),
    ("minimise_key", DesktopCommand::MinimiseKey),
    ("hide_active", DesktopCommand::HideActive),
    ("hide_others", DesktopCommand::HideOthers),
    ("quit_active", DesktopCommand::QuitActive),
    ("force_quit_panel", DesktopCommand::ForceQuitPanel),
    ("mission_control", DesktopCommand::MissionControl),
];

impl ActionWords {
    /// The action these words name.
    pub fn parse(&self) -> Result<BindingAction, CoreError> {
        let words: Vec<&str> = self.0.iter().map(String::as_str).collect();
        match words.as_slice() {
            ["spawn", argv @ ..] if !argv.is_empty() => {
                Some(BindingAction::Spawn(Argv(owned(argv))))
            }
            ["call", service, path, interface, method, args @ ..] => {
                Some(BindingAction::Call(DbusCall {
                    service: (*service).to_owned(),
                    path: (*path).to_owned(),
                    interface: (*interface).to_owned(),
                    method: (*method).to_owned(),
                    args: owned(args),
                }))
            }
            ["desktop", command @ ..] => desktop(command).map(BindingAction::Desktop),
            _ => None,
        }
        .ok_or_else(|| CoreError::Malformed {
            what: "binding action",
            text: self.0.join(" "),
        })
    }

    /// The words that name `action`.
    pub fn of(action: &BindingAction) -> ActionWords {
        let words: Vec<String> = match action {
            BindingAction::Spawn(Argv(argv)) => ["spawn".to_owned()]
                .into_iter()
                .chain(argv.iter().cloned())
                .collect(),
            BindingAction::Call(call) => [
                "call",
                &call.service,
                &call.path,
                &call.interface,
                &call.method,
            ]
            .into_iter()
            .map(str::to_owned)
            .chain(call.args.iter().cloned())
            .collect(),
            BindingAction::Desktop(command) => ["desktop".to_owned()]
                .into_iter()
                .chain(command_words(*command))
                .collect(),
        };
        ActionWords(words)
    }
}

fn owned(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

fn desktop(words: &[&str]) -> Option<DesktopCommand> {
    match words {
        ["switch_space", step] => step_of(step).map(DesktopCommand::SwitchSpace),
        ["place", word] => PLACE_TARGETS
            .iter()
            .find(|(name, _)| name == word)
            .map(|(_, target)| DesktopCommand::Place(*target)),
        [word] => UNIT_COMMANDS
            .iter()
            .find(|(name, _)| name == word)
            .map(|(_, command)| *command),
        _ => None,
    }
}

fn step_of(word: &str) -> Option<SpaceStep> {
    match word {
        "previous" => Some(SpaceStep::Previous),
        "next" => Some(SpaceStep::Next),
        number => number.parse().ok().map(SpaceStep::Number),
    }
}

fn command_words(command: DesktopCommand) -> Vec<String> {
    let unit = |wanted: DesktopCommand| {
        UNIT_COMMANDS
            .iter()
            .find(|(_, command)| *command == wanted)
            .map(|(name, _)| vec![(*name).to_owned()])
    };
    match command {
        DesktopCommand::SwitchSpace(step) => vec!["switch_space".to_owned(), step_word(step)],
        DesktopCommand::Place(target) => vec!["place".to_owned(), target_word(target)],
        other => unit(other).unwrap_or_default(),
    }
}

fn step_word(step: SpaceStep) -> String {
    match step {
        SpaceStep::Previous => "previous".to_owned(),
        SpaceStep::Next => "next".to_owned(),
        SpaceStep::Number(n) => n.to_string(),
    }
}

fn target_word(target: PlaceTarget) -> String {
    PLACE_TARGETS
        .iter()
        .find(|(_, known)| *known == target)
        .map(|(name, _)| (*name).to_owned())
        .unwrap_or_default()
}
