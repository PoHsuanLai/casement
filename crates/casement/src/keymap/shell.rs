//! What the compositor does for each standard action, and the bindings a keymap gives them
//! (pure).
//!
//! An action the table does not name is an app's (copy, save, find, ...) and never becomes a
//! binding. The chords are not here: they come from the keymap (conventions, machine, person).

use super::chord::chord_of;
use casement_core::input::{Argv, Binding, BindingAction, Bindings, DesktopCommand, SpaceStep};
use casement_protocols::vocab::PlaceTarget;
use chordkit::{Action, Keymap, StandardAction};
use shell_verbs::ShellVerb;

/// How the compositor carries out one action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Does {
    /// A command casement runs itself.
    Itself(DesktopCommand),
    /// A verb for sill, the shell (its launcher, switcher, screenshots, power menu, Force Quit
    /// panel), sent as its command line.
    Shell(ShellVerb),
}

type Row = (StandardAction, Does);

/// The standard actions the compositor binds. The Space numbers are not rows (nine actions
/// with one meaning each): see [`shell_action`].
const ROWS: &[Row] = &[
    // sill draws the panel from the desktop manager's app list (quire_shell_v1 has no event for
    // the panel yet).
    (
        StandardAction::ForceQuit,
        Does::Shell(ShellVerb::ForceQuitToggle),
    ),
    (
        StandardAction::MissionControl,
        Does::Itself(DesktopCommand::MissionControl),
    ),
    // View > Enter Full Screen: the compositor's, so it works in an app that has no such item.
    (
        StandardAction::FullScreen,
        Does::Itself(DesktopCommand::FullScreenKey),
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
        Does::Shell(ShellVerb::LauncherToggle),
    ),
    (
        StandardAction::NextApp,
        Does::Shell(ShellVerb::SwitcherNext),
    ),
    // The front app's next window (the Mac's Command+`).
    (
        StandardAction::NextWindow,
        Does::Shell(ShellVerb::NextWindow),
    ),
    // Flips the dock's auto-hide (the Mac's Option+Command+D).
    (
        StandardAction::ToggleDock,
        Does::Shell(ShellVerb::DockToggleHide),
    ),
    (StandardAction::LockScreen, Does::Shell(ShellVerb::Lock)),
    (
        StandardAction::PreviousApp,
        Does::Shell(ShellVerb::SwitcherPrev),
    ),
    (
        StandardAction::ScreenshotScreen,
        Does::Shell(ShellVerb::ScreenshotScreen),
    ),
    (
        StandardAction::ScreenshotSelection,
        Does::Shell(ShellVerb::ScreenshotArea),
    ),
    (
        StandardAction::ScreenshotTools,
        Does::Shell(ShellVerb::ScreenshotArea),
    ),
    (
        StandardAction::LogOut,
        Does::Shell(ShellVerb::PowerMenuOpen),
    ),
    // Window > Move & Resize (macOS Sequoia's tiling keys).
    (
        StandardAction::TileLeft,
        Does::Itself(DesktopCommand::Place(PlaceTarget::Left)),
    ),
    (
        StandardAction::TileRight,
        Does::Itself(DesktopCommand::Place(PlaceTarget::Right)),
    ),
    (
        StandardAction::TileTop,
        Does::Itself(DesktopCommand::Place(PlaceTarget::Top)),
    ),
    (
        StandardAction::TileBottom,
        Does::Itself(DesktopCommand::Place(PlaceTarget::Bottom)),
    ),
    (
        StandardAction::TileTopLeft,
        Does::Itself(DesktopCommand::Place(PlaceTarget::TopLeft)),
    ),
    (
        StandardAction::TileTopRight,
        Does::Itself(DesktopCommand::Place(PlaceTarget::TopRight)),
    ),
    (
        StandardAction::TileBottomLeft,
        Does::Itself(DesktopCommand::Place(PlaceTarget::BottomLeft)),
    ),
    (
        StandardAction::TileBottomRight,
        Does::Itself(DesktopCommand::Place(PlaceTarget::BottomRight)),
    ),
    (
        StandardAction::Fill,
        Does::Itself(DesktopCommand::Place(PlaceTarget::Fill)),
    ),
    (
        StandardAction::Center,
        Does::Itself(DesktopCommand::Place(PlaceTarget::Centre)),
    ),
    (
        StandardAction::ReturnToPreviousSize,
        Does::Itself(DesktopCommand::Place(PlaceTarget::Restore)),
    ),
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
        Does::Shell(verb) => spawn_of(verb),
    }
}

/// The command line that sends `verb` to sill.
fn spawn_of(verb: ShellVerb) -> BindingAction {
    BindingAction::Spawn(Argv(
        std::iter::once(SILL)
            .chain(verb.argv().iter().copied())
            .map(str::to_owned)
            .collect(),
    ))
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

    fn shell(verb: ShellVerb) -> Option<BindingAction> {
        Some(spawn_of(verb))
    }

    #[test]
    fn a_shell_verb_spawns_sill_with_its_argv() {
        assert_eq!(
            spawn_of(ShellVerb::DockToggleHide),
            BindingAction::Spawn(Argv(vec![
                "sill".to_owned(),
                "dock".to_owned(),
                "toggle-hide".to_owned()
            ]))
        );
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
    fn the_rows_are_the_handled_table() {
        use keycap_client::Handler;
        for action in StandardAction::all() {
            let want = match shell_action(action) {
                None => None,
                Some(BindingAction::Desktop(_)) => Some(Handler::Compositor),
                Some(BindingAction::Spawn(argv)) => {
                    let args: Vec<&str> = argv.0.iter().skip(1).map(String::as_str).collect();
                    ShellVerb::parse(&args).map(Handler::Shell)
                }
                Some(_) => None,
            };
            assert_eq!(keycap_client::handler(action), want, "{}", action.slug());
        }
        assert!(
            keycap_client::handled_actions().all(|(action, _)| shell_action(action).is_some())
        );
    }

    #[test]
    fn the_conventions_of_our_desktop_bind_the_shell() {
        let bindings = bindings_of(&Keymap::conventional(OURS));
        let cases: Vec<(&str, &str, Option<BindingAction>)> = vec![
            (
                "force quit",
                "option+cmd+Escape",
                shell(ShellVerb::ForceQuitToggle),
            ),
            (
                "Mission Control",
                "ctrl+Up",
                itself(DesktopCommand::MissionControl),
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
            ("launcher", "cmd+space", shell(ShellVerb::LauncherToggle)),
            ("next app", "cmd+Tab", shell(ShellVerb::SwitcherNext)),
            (
                "previous app",
                "shift+cmd+Tab",
                shell(ShellVerb::SwitcherPrev),
            ),
            (
                "screen shot",
                "shift+cmd+3",
                shell(ShellVerb::ScreenshotScreen),
            ),
            ("area shot", "shift+cmd+4", shell(ShellVerb::ScreenshotArea)),
            (
                "shot tools",
                "shift+cmd+5",
                shell(ShellVerb::ScreenshotArea),
            ),
            ("log out", "shift+cmd+q", shell(ShellVerb::PowerMenuOpen)),
            (
                "tile left",
                "ctrl+option+Left",
                itself(DesktopCommand::Place(PlaceTarget::Left)),
            ),
            (
                "tile bottom",
                "ctrl+option+Down",
                itself(DesktopCommand::Place(PlaceTarget::Bottom)),
            ),
            (
                "top left quarter",
                "ctrl+option+u",
                itself(DesktopCommand::Place(PlaceTarget::TopLeft)),
            ),
            (
                "fill",
                "ctrl+option+f",
                itself(DesktopCommand::Place(PlaceTarget::Fill)),
            ),
            (
                "centre",
                "ctrl+option+c",
                itself(DesktopCommand::Place(PlaceTarget::Centre)),
            ),
            (
                "return to previous size",
                "ctrl+option+r",
                itself(DesktopCommand::Place(PlaceTarget::Restore)),
            ),
            ("copy is an app's", "cmd+c", None),
            ("close is the app's menu", "cmd+w", None),
            ("minimise is the app's menu", "cmd+m", None),
            (
                "full screen",
                "ctrl+cmd+f",
                itself(DesktopCommand::FullScreenKey),
            ),
            ("hide is the app's menu", "cmd+h", None),
            ("hide others is the app's menu", "option+cmd+h", None),
            ("quit is the app's menu", "cmd+q", None),
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
        let (overrides, bad) = Overrides::parse("launcher = Ctrl+Alt+Space\n");
        assert!(bad.is_empty());
        let (keymap, refused) = Keymap::conventional(OURS).with_overrides(&overrides);
        assert!(refused.is_empty());
        let bindings = bindings_of(&keymap);
        assert_eq!(
            what(&bindings, "ctrl+option+space"),
            shell(ShellVerb::LauncherToggle)
        );
        assert_eq!(what(&bindings, "cmd+space"), None);
    }

    #[test]
    fn the_persons_unbinding_removes_a_binding() {
        let (overrides, _) = Overrides::parse("space-left = none\n");
        let (keymap, _) = Keymap::conventional(OURS).with_overrides(&overrides);
        assert_eq!(what(&bindings_of(&keymap), "ctrl+Left"), None);
    }

    #[test]
    fn the_documents_own_bindings_win_a_shared_chord() {
        let own = Bindings(vec![Binding {
            chord: Chord::parse("cmd+space").expect("a chord"),
            action: BindingAction::Desktop(DesktopCommand::HideActive),
        }]);
        let merged = merge(&own, &bindings_of(&Keymap::conventional(OURS)));
        assert_eq!(
            casement_core::input::resolve(
                &merged,
                &Chord::parse("cmd+space").expect("a chord"),
                &casement_core::input::Origin::Test,
            ),
            casement_core::input::Resolved::Action(BindingAction::Desktop(
                DesktopCommand::HideActive
            ))
        );
        assert!(merged.0.len() > own.0.len());
    }
}
