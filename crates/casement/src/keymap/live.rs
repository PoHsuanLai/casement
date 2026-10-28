//! The keymap source for a config home, the first load and the reload when the keymap
//! changes.

use super::shell::{bindings_of, merge};
use crate::State;
use casement_core::Event;
use casement_core::input::Bindings;
use chordkit::{Desktop, KeymapSource, Platform, Watch};
use keycap_client::KeycapSource;
use smithay::reexports::calloop::channel::{self, Event as ChannelEvent};
use std::path::Path;
use std::sync::Mutex;

/// The platform whose keymap the compositor reads: our desktop.
const OURS: Platform = Platform::Linux {
    desktop: Desktop::Ours,
};

/// Where the person's keymap and the machine's sit under a stand-in config home (the layout
/// of a real one: `keycap/keymap` for the person, and a sibling directory for the machine).
const PERSON_DIR: &str = "keycap";
const MACHINE_DIR: &str = "keycap-machine";
const FILE: &str = "keymap";
const SOCKET: &str = "keycap.sock";

/// The keymap source of `home`. The real home reads the environment (`$KEYCAP_SOCKET`,
/// `$KEYCAP_CONFIG_DIR`, `$XDG_CONFIG_HOME`); a stand-in directory reads only inside itself, and
/// no daemon answers there, so a test or a jail never reaches the real machine's keymap.
pub fn source_for(home: Option<&Path>) -> KeycapSource {
    match home {
        None => KeycapSource::standard(),
        Some(dir) => stand_in(dir),
    }
}

fn stand_in(dir: &Path) -> KeycapSource {
    KeycapSource::new(
        dir.join(PERSON_DIR).join(SOCKET),
        dir.join(MACHINE_DIR).join(FILE),
        dir.join(PERSON_DIR).join(FILE),
    )
}

/// The shell bindings `source` gives now, and every problem it reported in plain words.
pub fn load_bindings(source: &KeycapSource) -> (Bindings, Vec<String>) {
    let (keymap, problems) = source.load_reporting(OURS);
    (bindings_of(&keymap), problems)
}

/// What a reload needs: the source, casement's own bindings (which stay first) and the watch
/// that tells the loop the keymap changed.
#[derive(Debug)]
struct Live {
    source: KeycapSource,
    explicit: Bindings,
    _watch: Watch,
}

impl Live {
    /// The bindings the model runs with: casement's own, then the keymap's.
    fn bindings(&self) -> Bindings {
        let (from_keymap, problems) = load_bindings(&self.source);
        for problem in problems {
            tracing::warn!(%problem, "the keymap has a line casement could not use");
        }
        merge(&self.explicit, &from_keymap)
    }
}

/// Reads the keymap into the model's settings now, and again whenever it changes. The bindings
/// already in the settings (casement's own `[bindings]`) stay first. A watch that cannot start
/// is logged and the keymap read once is kept.
pub fn attach(state: &mut State, home: Option<&Path>) -> Result<(), crate::StartError> {
    let source = source_for(home);
    let (sender, receiver) = channel::channel::<()>();
    let sender = Mutex::new(sender);
    let watch = source
        .watch(Box::new(move || {
            if let Ok(sender) = sender.lock() {
                let _ = sender.send(());
            }
        }))
        .unwrap_or_else(|error| {
            tracing::warn!(%error, "the keymap is read once: it could not be watched");
            Watch::inert()
        });
    let live = Live {
        source,
        explicit: state.settings().bindings.clone(),
        _watch: watch,
    };
    apply(state, &live);
    state
        .event_loop
        .clone()
        .insert_source(receiver, move |event, (), state: &mut State| {
            if matches!(event, ChannelEvent::Msg(())) {
                apply(state, &live);
            }
        })
        .map(|_| ())
        .map_err(|error| crate::StartError::Loop(error.to_string()))
}

/// Puts the keymap's bindings into the model when they differ from the ones it has.
fn apply(state: &mut State, live: &Live) {
    let bindings = live.bindings();
    if state.settings().bindings == bindings {
        return;
    }
    let mut settings = state.settings().clone();
    settings.bindings = bindings;
    crate::core_loop::feed(state, vec![Event::SettingsChanged(Box::new(settings))]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::input::{Argv, BindingAction, Chord};

    fn what(bindings: &Bindings, chord: &str) -> Option<BindingAction> {
        let chord = Chord::parse(chord).expect("a chord");
        bindings
            .0
            .iter()
            .find(|binding| binding.chord == chord)
            .map(|binding| binding.action.clone())
    }

    fn launcher() -> Option<BindingAction> {
        Some(BindingAction::Spawn(Argv(
            ["sill", "launcher", "toggle"].map(str::to_owned).to_vec(),
        )))
    }

    #[test]
    fn a_home_with_no_keymap_files_gives_the_conventions() {
        let home = tempfile::tempdir().expect("a scratch directory");
        let (bindings, problems) = load_bindings(&source_for(Some(home.path())));
        assert_eq!(problems, Vec::<String>::new());
        assert_eq!(what(&bindings, "cmd+space"), launcher());
    }

    #[test]
    fn the_persons_file_comes_after_the_conventions_and_the_machines_file() {
        let home = tempfile::tempdir().expect("a scratch directory");
        for (dir, text) in [
            (MACHINE_DIR, "launcher = Primary+Shift+Space\n"),
            (PERSON_DIR, "launcher = Ctrl+Shift+Primary+Space\n"),
        ] {
            let dir = home.path().join(dir);
            std::fs::create_dir_all(&dir).expect("a directory");
            std::fs::write(dir.join(FILE), text).expect("a keymap file");
        }
        let (bindings, _) = load_bindings(&source_for(Some(home.path())));
        assert_eq!(what(&bindings, "ctrl+shift+cmd+space"), launcher());
        assert_eq!(what(&bindings, "shift+cmd+space"), None);
        assert_eq!(what(&bindings, "cmd+space"), None);
    }

    #[test]
    fn a_line_that_does_not_read_is_a_problem_and_the_rest_still_apply() {
        let home = tempfile::tempdir().expect("a scratch directory");
        let dir = home.path().join(PERSON_DIR);
        std::fs::create_dir_all(&dir).expect("a directory");
        std::fs::write(
            dir.join(FILE),
            "nonsense\nlauncher = Ctrl+Shift+Primary+Space\n",
        )
        .expect("a file");
        let (bindings, problems) = load_bindings(&source_for(Some(home.path())));
        assert_eq!(problems.len(), 1);
        assert_eq!(what(&bindings, "ctrl+shift+cmd+space"), launcher());
    }
}
