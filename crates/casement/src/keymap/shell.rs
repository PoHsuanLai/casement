//! What the compositor does for each standard action, and the bindings a keymap gives them
//! (pure).
//!
//! An action the table does not name is an app's (copy, save, find, ...) and never becomes a
//! binding. The chords are not here: they come from the keymap (conventions, machine, person).

use super::chord::chord_of;
use casement_core::input::{Argv, Binding, BindingAction, Bindings, DesktopCommand, SpaceStep};
use chordkit::{Action, Keymap, StandardAction};

/// How the compositor carries out one action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Does {
    /// A command casement runs itself.
    Itself(DesktopCommand),
    /// A command line of sill, the shell (its launcher, switcher, screenshots, power menu).
    Sill(&'static [&'static str]),
}

type Row = (StandardAction, Does);

/// The standard actions the compositor binds. The Space numbers are not rows (nine actions
/// with one meaning each): see [`shell_action`].
const ROWS: &[Row] = &[
    (
        StandardAction::Close,
        Does::Itself(DesktopCommand::CloseKey),
    ),
    (
        StandardAction::Minimize,
        Does::Itself(DesktopCommand::MinimiseKey),
    ),
    (
        StandardAction::FullScreen,
        Does::Itself(DesktopCommand::ToggleFullScreenKey),
    ),
    (
        StandardAction::Hide,
        Does::Itself(DesktopCommand::HideActive),
    ),
    (
        StandardAction::HideOthers,
        Does::Itself(DesktopCommand::HideOthers),
    ),
    (
        StandardAction::Quit,
        Does::Itself(DesktopCommand::QuitActive),
    ),
    (
        StandardAction::ForceQuit,
        Does::Itself(DesktopCommand::ForceQuitPanel),
    ),
    (
        StandardAction::SpaceLeft,
        Does::Itself(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
    ),
    (
        StandardAction::SpaceRight,
        Does::Itself(DesktopCommand::SwitchSpace(SpaceStep::Next)),
    ),
    (
        StandardAction::Launcher,
        Does::Sill(&["launcher", "toggle"]),
    ),
    (StandardAction::NextApp, Does::Sill(&["switcher", "next"])),
    (
        StandardAction::PreviousApp,
        Does::Sill(&["switcher", "prev"]),
    ),
    (
        StandardAction::ScreenshotScreen,
        Does::Sill(&["screenshot", "screen"]),
    ),
    (
        StandardAction::ScreenshotSelection,
        Does::Sill(&["screenshot", "area"]),
    ),
    (
        StandardAction::ScreenshotTools,
        Does::Sill(&["screenshot", "area"]),
    ),
    (StandardAction::LogOut, Does::Sill(&["power-menu", "open"])),
];

/// The program sill's commands run under.
const SILL: &str = "sill";

/// What the compositor does for `action`, or `None` when the action belongs to apps.
pub fn shell_action(action: StandardAction) -> Option<BindingAction> {
    match action {
        StandardAction::SwitchToSpace(number) => Some(BindingAction::Desktop(
            DesktopCommand::SwitchSpace(SpaceStep::Number(number.get())),
        )),
        other => ROWS
            .iter()
            .find(|(named, _)| *named == other)
            .map(|(_, does)| binding_action(*does)),
    }
}

fn binding_action(does: Does) -> BindingAction {
    match does {
        Does::Itself(command) => BindingAction::Desktop(command),
        Does::Sill(words) => BindingAction::Spawn(Argv(
            std::iter::once(SILL)
                .chain(words.iter().copied())
                .map(str::to_owned)
                .collect(),
        )),
    }
}

/// Every binding `keymap` gives the compositor's actions, one per chord, actions in
/// `StandardAction::all` order and each action's chords best first. A chord casement cannot
/// see as a key press is left out.
pub fn bindings_of(keymap: &Keymap) -> Bindings {
    Bindings(
        StandardAction::all()
            .filter_map(|action| shell_action(action).map(|does| (action, does)))
            .flat_map(|(action, does)| {
                keymap
                    .chords_of(&Action::Standard(action))
                    .into_iter()
                    .filter_map(chord_of)
                    .map(move |chord| Binding {
                        chord,
                        action: does.clone(),
                    })
            })
            .collect(),
    )
}

/// `explicit` (casement's own `[bindings]`) then `from_keymap`: the first match wins, so a
/// chord casement's document binds is not taken from it by the keymap.
pub fn merge(explicit: &Bindings, from_keymap: &Bindings) -> Bindings {
    Bindings(explicit.0.iter().chain(&from_keymap.0).cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::input::Chord;
    use chordkit::{Desktop, Overrides, Platform};

    const OURS: Platform = Platform::Linux {
        desktop: Desktop::Ours,
    };

    fn itself(command: DesktopCommand) -> Option<BindingAction> {
        Some(BindingAction::Desktop(command))
    }

    fn sill(words: &[&str]) -> Option<BindingAction> {
        Some(BindingAction::Spawn(Argv(
            std::iter::once(SILL)
                .chain(words.iter().copied())
                .map(str::to_owned)
                .collect(),
        )))
    }

    fn space(n: u8) -> Option<BindingAction> {
        itself(DesktopCommand::SwitchSpace(SpaceStep::Number(n)))
    }

    fn what(bindings: &Bindings, chord: &str) -> Option<BindingAction> {
        let chord = Chord::parse(chord).expect("a chord");
        bindings
            .0
            .iter()
            .find(|binding| binding.chord == chord)
            .map(|binding| binding.action.clone())
    }

    #[test]
    fn the_conventions_of_our_desktop_bind_the_shell() {
        let bindings = bindings_of(&Keymap::conventional(OURS));
        let cases: Vec<(&str, &str, Option<BindingAction>)> = vec![
            ("close", "cmd+w", itself(DesktopCommand::CloseKey)),
            ("minimise", "cmd+m", itself(DesktopCommand::MinimiseKey)),
            (
                "full screen",
                "ctrl+cmd+f",
                itself(DesktopCommand::ToggleFullScreenKey),
            ),
            ("hide", "cmd+h", itself(DesktopCommand::HideActive)),
            (
                "hide others",
                "option+cmd+h",
                itself(DesktopCommand::HideOthers),
            ),
            ("quit", "cmd+q", itself(DesktopCommand::QuitActive)),
            (
                "force quit",
                "option+cmd+Escape",
                itself(DesktopCommand::ForceQuitPanel),
            ),
            ("Space 1", "ctrl+1", space(1)),
            ("Space 9", "ctrl+9", space(9)),
            (
                "Space left",
                "ctrl+Left",
                itself(DesktopCommand::SwitchSpace(SpaceStep::Previous)),
            ),
            (
                "Space right",
                "ctrl+Right",
                itself(DesktopCommand::SwitchSpace(SpaceStep::Next)),
            ),
            ("launcher", "cmd+space", sill(&["launcher", "toggle"])),
            ("next app", "cmd+Tab", sill(&["switcher", "next"])),
            ("previous app", "shift+cmd+Tab", sill(&["switcher", "prev"])),
            (
                "screen shot",
                "shift+cmd+3",
                sill(&["screenshot", "screen"]),
            ),
            ("area shot", "shift+cmd+4", sill(&["screenshot", "area"])),
            ("shot tools", "shift+cmd+5", sill(&["screenshot", "area"])),
            ("log out", "shift+cmd+q", sill(&["power-menu", "open"])),
            ("copy is an app's", "cmd+c", None),
            ("the input language is fcitx's", "ctrl+space", None),
            ("an unbound chord", "cmd+F13", None),
        ];
        for (case, chord, want) in cases {
            assert_eq!(what(&bindings, chord), want, "{case}");
        }
    }

    #[test]
    fn every_action_the_table_names_has_a_chord_on_our_desktop() {
        let keymap = Keymap::conventional(OURS);
        let silent: Vec<String> = StandardAction::all()
            .filter(|action| shell_action(*action).is_some())
            .filter(|action| {
                keymap
                    .chords_of(&Action::Standard(*action))
                    .into_iter()
                    .filter_map(chord_of)
                    .next()
                    .is_none()
            })
            .map(StandardAction::slug)
            .collect();
        assert_eq!(silent, Vec::<String>::new());
    }

    #[test]
    fn no_chord_is_bound_twice_by_the_conventions() {
        let bindings = bindings_of(&Keymap::conventional(OURS));
        let mut chords: Vec<_> = bindings
            .0
            .iter()
            .map(|binding| binding.chord.text())
            .collect();
        chords.sort();
        let before = chords.len();
        chords.dedup();
        assert_eq!(chords.len(), before);
    }

    #[test]
    fn the_persons_keymap_moves_a_binding() {
        let (overrides, bad) = Overrides::parse("close = Primary+Shift+W\n");
        assert!(bad.is_empty());
        let (keymap, refused) = Keymap::conventional(OURS).with_overrides(&overrides);
        assert!(refused.is_empty());
        let bindings = bindings_of(&keymap);
        assert_eq!(
            what(&bindings, "shift+cmd+w"),
            itself(DesktopCommand::CloseKey)
        );
        assert_eq!(what(&bindings, "cmd+w"), None);
    }

    #[test]
    fn the_persons_unbinding_removes_a_binding() {
        let (overrides, _) = Overrides::parse("minimize = none\n");
        let (keymap, _) = Keymap::conventional(OURS).with_overrides(&overrides);
        assert_eq!(what(&bindings_of(&keymap), "cmd+m"), None);
    }

    #[test]
    fn the_documents_own_bindings_win_a_shared_chord() {
        let own = Bindings(vec![Binding {
            chord: Chord::parse("cmd+w").expect("a chord"),
            action: BindingAction::Desktop(DesktopCommand::HideActive),
        }]);
        let merged = merge(&own, &bindings_of(&Keymap::conventional(OURS)));
        assert_eq!(
            casement_core::input::resolve(
                &merged,
                &Chord::parse("cmd+w").expect("a chord"),
                &casement_core::input::Origin::Test,
            ),
            casement_core::input::Resolved::Action(BindingAction::Desktop(
                DesktopCommand::HideActive
            ))
        );
        assert!(merged.0.len() > own.0.len());
    }
}
