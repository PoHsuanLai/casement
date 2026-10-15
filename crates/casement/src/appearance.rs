//! The person's light or dark scheme, read from quire's `appearance.toml` through ds-settings
//! and followed while casement runs. The server-side window frame is drawn in it
//! (`core_loop::frames::styles` reads `state.render.scheme` every frame, so a change shows at
//! the next vblank).
//!
//! `appearance.theme = "system"` follows the desktop: the scheme is read from the settings
//! portal once, when the theme first is `system` (at start or after the file changes to it), and
//! the portal's changes are followed after that (`casement_ipc::system_scheme`, a thread with its
//! own runtime; the answers come back over a calloop channel like the file's). Until the portal
//! has answered, and on a desktop without one, `system` paints Light. Where the portal is
//! reached is the [`ConfigHome`] seam: the real config tree uses the session bus, a stand-in
//! directory (a test, a jail) uses a fixed answer and never touches a bus.

use crate::State;
use crate::cli::ConfigHome;
use casement_ipc::system_scheme::{DesktopScheme, SchemeWatch};
use casement_layout::{Scheme, Theme, scheme_of};
use ds_core::spawner::Spawner;
use ds_settings::{AppName, AppearanceFile, ConfigRoot, Store, SystemPrefsSource};
use smithay::reexports::calloop::channel::{self, Event as ChannelEvent};
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Wake, Waker};
use std::thread::Thread;

/// The appearance store of `home`: the real config tree, or a stand-in directory (a test or a
/// jail never reaches the machine's own file).
fn store_for(home: &ConfigHome) -> Store {
    let root = match home {
        ConfigHome::Xdg => ConfigRoot::Xdg,
        ConfigHome::At(dir) => ConfigRoot::Scratch(dir.clone()),
    };
    Store::new(root, AppName::QUIRE)
}

/// The scheme `home`'s appearance file paints now (the light scheme when there is no file, and
/// for `system`, which [`attach`] resolves against the desktop).
///
/// ```
/// use casement::appearance::scheme_in;
/// use casement::cli::ConfigHome;
/// use casement_layout::Scheme;
///
/// let nowhere = ConfigHome::At("/nonexistent/casement-doc-home".into());
/// assert_eq!(scheme_in(&nowhere), Scheme::Light);
/// ```
pub fn scheme_in(home: &ConfigHome) -> Scheme {
    scheme_of(theme_in(home))
}

/// The theme `home`'s appearance file stores (the default when there is no file or the key is bad).
fn theme_in(home: &ConfigHome) -> Theme {
    let loaded = store_for(home).load::<AppearanceFile>();
    for key in &loaded.invalid {
        tracing::warn!("appearance.toml: {key}");
    }
    loaded.value.appearance.theme
}

/// Where the desktop's scheme is read from for `home`: the session bus for the real config tree,
/// a fixed answer (the defaults) for a stand-in directory, so a test never reaches a bus.
///
/// ```
/// use casement::appearance::system_source;
/// use casement::cli::ConfigHome;
/// use ds_settings::SystemPrefsSource;
///
/// assert_eq!(system_source(&ConfigHome::Xdg), SystemPrefsSource::Portal);
/// ```
pub fn system_source(home: &ConfigHome) -> SystemPrefsSource {
    match home {
        ConfigHome::Xdg => SystemPrefsSource::Portal,
        ConfigHome::At(_) => SystemPrefsSource::Fixed(Default::default()),
    }
}

/// The scheme a `theme` paints given the desktop's `system` scheme: only `system` follows it.
///
/// ```
/// use casement::appearance::resolve;
/// use casement_layout::{Scheme, Theme};
///
/// assert_eq!(resolve(Theme::System, Scheme::Dark), Scheme::Dark);
/// assert_eq!(resolve(Theme::Light, Scheme::Dark), Scheme::Light);
/// ```
pub fn resolve(theme: Theme, system: Scheme) -> Scheme {
    match theme {
        Theme::System => system,
        Theme::Dark => Scheme::Dark,
        _ => Scheme::Light,
    }
}

/// What reaches the loop: the stored theme changed, or the desktop's scheme did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Input {
    Theme(Theme),
    System(Scheme),
}

/// The two things the scheme is made of, as last heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Heard {
    theme: Theme,
    system: Scheme,
}

impl Heard {
    /// The state before the desktop has answered: its scheme is Light.
    fn start(theme: Theme) -> Heard {
        Heard {
            theme,
            system: Scheme::Light,
        }
    }

    fn after(self, input: Input) -> Heard {
        match input {
            Input::Theme(theme) => Heard { theme, ..self },
            Input::System(system) => Heard { system, ..self },
        }
    }

    fn scheme(self) -> Scheme {
        resolve(self.theme, self.system)
    }
}

/// The desktop's scheme as a `Scheme`.
fn scheme_of_desktop(scheme: DesktopScheme) -> Scheme {
    match scheme {
        DesktopScheme::Dark => Scheme::Dark,
        DesktopScheme::Light => Scheme::Light,
    }
}

/// Starts following the desktop's scheme from `source`, its answers sent as `Input::System`. A
/// watcher that cannot start is logged and `system` stays Light.
fn follow_desktop(
    source: SystemPrefsSource,
    sender: channel::Sender<Input>,
) -> Option<SchemeWatch> {
    let tell =
        move |scheme: DesktopScheme| match sender.send(Input::System(scheme_of_desktop(scheme))) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        };
    match SchemeWatch::start(source, tell) {
        Ok(watch) => Some(watch),
        Err(error) => {
            tracing::warn!(%error, "the desktop's colour scheme is not followed: system paints light");
            None
        }
    }
}

/// Reads the scheme into `state` now, and again whenever the file (or, for `theme = "system"`,
/// the desktop) changes. A file that cannot be watched is logged and the scheme read once is kept.
pub(crate) fn attach(state: &mut State, home: &ConfigHome) -> Result<(), crate::StartError> {
    let theme = theme_in(home);
    let mut heard = Heard::start(theme);
    state.render.scheme = heard.scheme();
    let (sender, receiver) = channel::channel::<Input>();
    let source = system_source(home);
    let mut following = (theme == Theme::System)
        .then(|| follow_desktop(source.clone(), sender.clone()))
        .flatten();
    let desktop_sender = sender.clone();
    let mut watch = store_for(home).watch::<AppearanceFile>(&ThreadSpawner);
    ThreadSpawner.spawn(Box::pin(async move {
        while let Some(loaded) = watch.changed().await {
            if sender
                .send(Input::Theme(loaded.value.appearance.theme))
                .is_err()
            {
                return;
            }
        }
    }));
    state
        .event_loop
        .clone()
        .insert_source(receiver, move |event, (), state: &mut State| {
            if let ChannelEvent::Msg(input) = event {
                // The first time the theme is `system`, ask the desktop (once, at start or after
                // the file changes to it); the watcher stays for the run.
                if input == Input::Theme(Theme::System) && following.is_none() {
                    following = follow_desktop(source.clone(), desktop_sender.clone());
                }
                heard = heard.after(input);
                state.render.scheme = heard.scheme();
            }
        })
        .map(|_| ())
        .map_err(|error| crate::StartError::Loop(error.to_string()))
}

/// Runs each task on a thread of its own, parked between wake-ups: ds-settings' watch is an
/// async task and casement runs no async runtime.
#[derive(Debug, Clone, Copy)]
struct ThreadSpawner;

impl Spawner for ThreadSpawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let started = std::thread::Builder::new()
            .name("casement-appearance".to_owned())
            .spawn(move || block_on(task));
        if let Err(error) = started {
            tracing::warn!(%error, "the appearance file is read once: it could not be watched");
        }
    }
}

/// Wakes the thread that polls a task.
struct Unpark(Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Polls `task` on this thread until it finishes, parking while it waits.
fn block_on(mut task: Pin<Box<dyn Future<Output = ()> + Send>>) {
    let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    while task.as_mut().poll(&mut context).is_pending() {
        std::thread::park();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_home_with_no_file_paints_light() {
        let home = tempfile::tempdir().expect("a scratch directory");
        assert_eq!(
            scheme_in(&ConfigHome::At(home.path().into())),
            Scheme::Light
        );
    }

    #[test]
    fn the_file_picks_the_scheme() {
        const CASES: &[(&str, &str, Scheme)] = &[
            ("dark", "[appearance]\ntheme = \"dark\"\n", Scheme::Dark),
            ("light", "[appearance]\ntheme = \"light\"\n", Scheme::Light),
            (
                "system",
                "[appearance]\ntheme = \"system\"\n",
                Scheme::Light,
            ),
            (
                "a bad value costs only itself",
                "[appearance]\ntheme = \"purple\"\n",
                Scheme::Light,
            ),
        ];
        for (name, text, want) in CASES {
            let home = tempfile::tempdir().expect("a scratch directory");
            let dir = home.path().join("quire");
            std::fs::create_dir_all(&dir).expect("a directory");
            std::fs::write(dir.join("appearance.toml"), text).expect("a file");
            assert_eq!(
                scheme_in(&ConfigHome::At(home.path().into())),
                *want,
                "{name}"
            );
        }
    }

    #[test]
    fn only_system_follows_the_desktop() {
        const CASES: &[(&str, Theme, Scheme, Scheme)] = &[
            (
                "system follows dark",
                Theme::System,
                Scheme::Dark,
                Scheme::Dark,
            ),
            (
                "system follows light",
                Theme::System,
                Scheme::Light,
                Scheme::Light,
            ),
            (
                "light ignores dark",
                Theme::Light,
                Scheme::Dark,
                Scheme::Light,
            ),
            (
                "dark ignores light",
                Theme::Dark,
                Scheme::Light,
                Scheme::Dark,
            ),
        ];
        for (name, theme, system, want) in CASES {
            assert_eq!(resolve(*theme, *system), *want, "{name}");
        }
    }

    #[test]
    fn the_scheme_follows_the_file_and_the_desktop_in_any_order() {
        let steps = [
            (Input::System(Scheme::Dark), Scheme::Dark),
            (Input::Theme(Theme::Light), Scheme::Light),
            (Input::System(Scheme::Light), Scheme::Light),
            (Input::Theme(Theme::System), Scheme::Light),
            (Input::System(Scheme::Dark), Scheme::Dark),
            (Input::Theme(Theme::Dark), Scheme::Dark),
        ];
        let mut heard = Heard::start(Theme::System);
        assert_eq!(heard.scheme(), Scheme::Light, "before the desktop answers");
        for (input, want) in steps {
            heard = heard.after(input);
            assert_eq!(heard.scheme(), want, "{input:?}");
        }
    }

    #[test]
    fn a_stand_in_home_never_reaches_a_bus() {
        let jail = ConfigHome::At("/nonexistent/casement-jail".into());
        assert_eq!(
            system_source(&jail),
            SystemPrefsSource::Fixed(Default::default())
        );
        assert_eq!(system_source(&ConfigHome::Xdg), SystemPrefsSource::Portal);
    }

    #[test]
    fn the_desktop_scheme_maps_one_to_one() {
        assert_eq!(scheme_of_desktop(DesktopScheme::Dark), Scheme::Dark);
        assert_eq!(scheme_of_desktop(DesktopScheme::Light), Scheme::Light);
    }

    #[test]
    fn block_on_finishes_a_ready_task() {
        let (sender, receiver) = std::sync::mpsc::channel();
        block_on(Box::pin(async move {
            let _ = sender.send(7);
        }));
        assert_eq!(receiver.recv(), Ok(7));
    }
}
