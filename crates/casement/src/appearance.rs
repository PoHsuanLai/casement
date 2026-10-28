//! The person's light or dark scheme and their Reduce transparency choice, read from quire's
//! `appearance.toml` through ds-settings and followed while casement runs. The transparency is
//! quire's `appearance.transparency` (`system` or `reduced`; a file without it is `system`), the
//! one desktop-wide setting the shell reads too: `reduced` draws every blur as its mostly opaque
//! stand-in in the scheme's paper ([`blur_finish`]), at the next frame. The server-side window frame is drawn in it
//! (`core_loop::frames::styles` reads `state.render.scheme` every frame, so a change shows at
//! the next vblank).
//!
//! `appearance.theme = "system"` follows the desktop: the scheme is read from the settings
//! portal once, when the theme first is `system` (at start or after the file changes to it), and
//! the portal's changes are followed after that (`casement_ipc::system_scheme`, a thread with its
//! own runtime; the answers come back over a calloop channel like the file's). Until the portal
//! has answered, and on a desktop without one, `system` paints Light. Where the portal is
//! reached is the config home seam: the real config tree uses the session bus, a stand-in
//! directory (a test, a jail) uses a fixed answer and never touches a bus.

use crate::State;
use casement_ipc::system_scheme::{DesktopScheme, SchemeWatch};
use casement_layout::{Scheme, Theme, Transparency, reduced_stand_in, scheme_of};
use casement_render::{BlurFinish, Rgba8};
use ds_core::spawner::Spawner;
use ds_settings::{AppName, AppearanceFile, ConfigRoot, Store, SystemPrefsSource};
use smithay::reexports::calloop::channel::{self, Event as ChannelEvent};
use std::future::Future;
use std::ops::ControlFlow;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Wake, Waker};
use std::thread::Thread;

/// The appearance store of `home`: the real config tree, or a stand-in directory (a test or a
/// jail never reaches the machine's own file).
fn store_for(home: Option<&Path>) -> Store {
    let root = home.map_or(ConfigRoot::Xdg, |dir| {
        ConfigRoot::Scratch(dir.to_path_buf())
    });
    Store::new(root, AppName::QUIRE)
}

/// The scheme `home`'s appearance file paints now (the light scheme when there is no file, and
/// for `system`, which [`attach`] resolves against the desktop).
///
/// ```
/// use casement::appearance::scheme_in;
/// use casement_layout::Scheme;
/// use std::path::Path;
///
/// let nowhere = Path::new("/nonexistent/casement-doc-home");
/// assert_eq!(scheme_in(Some(nowhere)), Scheme::Light);
/// ```
pub fn scheme_in(home: Option<&Path>) -> Scheme {
    scheme_of(stored_in(home).theme)
}

/// What `home`'s appearance file stores that casement follows: the defaults when there is no file
/// or a key is bad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stored {
    theme: Theme,
    transparency: Transparency,
}

fn stored_in(home: Option<&Path>) -> Stored {
    let loaded = store_for(home).load::<AppearanceFile>();
    for key in &loaded.invalid {
        tracing::warn!("appearance.toml: {key}");
    }
    Stored {
        theme: loaded.value.appearance.theme,
        transparency: loaded.value.appearance.transparency,
    }
}

/// How a blur is drawn given the person's `transparency` choice and the `scheme` in force: only
/// `reduced` replaces it, with the scheme's solid stand-in.
///
/// ```
/// use casement::appearance::blur_finish;
/// use casement_layout::{Scheme, Transparency};
/// use casement_render::BlurFinish;
///
/// assert_eq!(blur_finish(Transparency::System, Scheme::Dark), BlurFinish::Frosted);
/// assert!(matches!(
///     blur_finish(Transparency::Reduced, Scheme::Dark),
///     BlurFinish::Solid(_)
/// ));
/// ```
pub fn blur_finish(transparency: Transparency, scheme: Scheme) -> BlurFinish {
    match transparency {
        Transparency::Reduced => BlurFinish::Solid(Rgba8::from(reduced_stand_in(scheme))),
        _ => BlurFinish::Frosted,
    }
}

/// Where the desktop's scheme is read from for `home`: the session bus for the real config tree,
/// a fixed answer (the defaults) for a stand-in directory, so a test never reaches a bus.
///
/// ```
/// use casement::appearance::system_source;
/// use ds_settings::SystemPrefsSource;
///
/// assert_eq!(system_source(None), SystemPrefsSource::Portal);
/// ```
pub fn system_source(home: Option<&Path>) -> SystemPrefsSource {
    match home {
        None => SystemPrefsSource::Portal,
        Some(_) => SystemPrefsSource::Fixed(Default::default()),
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

/// What reaches the loop: the stored theme or transparency changed, or the desktop's scheme did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Input {
    Theme(Theme),
    Transparency(Transparency),
    System(Scheme),
}

/// The things the scheme and the transparency are made of, as last heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Heard {
    theme: Theme,
    transparency: Transparency,
    system: Scheme,
}

impl Heard {
    /// The state before the desktop has answered: its scheme is Light.
    fn start(stored: Stored) -> Heard {
        Heard {
            theme: stored.theme,
            transparency: stored.transparency,
            system: Scheme::Light,
        }
    }

    fn after(self, input: Input) -> Heard {
        match input {
            Input::Theme(theme) => Heard { theme, ..self },
            Input::Transparency(transparency) => Heard {
                transparency,
                ..self
            },
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

/// Reads the scheme and the transparency into `state` now, and again whenever the file (or, for
/// `theme = "system"`, the desktop) changes. A file that cannot be watched is logged and the scheme read once is kept.
pub(crate) fn attach(state: &mut State, home: Option<&Path>) -> Result<(), crate::StartError> {
    let stored = stored_in(home);
    let theme = stored.theme;
    let mut heard = Heard::start(stored);
    state.render.scheme = heard.scheme();
    state.render.transparency = heard.transparency;
    let (sender, receiver) = channel::channel::<Input>();
    let source = system_source(home);
    let mut following = (theme == Theme::System)
        .then(|| follow_desktop(source.clone(), sender.clone()))
        .flatten();
    let desktop_sender = sender.clone();
    let mut watch = store_for(home).watch::<AppearanceFile>(&ThreadSpawner);
    ThreadSpawner.spawn(Box::pin(async move {
        while let Some(loaded) = watch.changed().await {
            let changes = [
                Input::Theme(loaded.value.appearance.theme),
                Input::Transparency(loaded.value.appearance.transparency),
            ];
            if changes.into_iter().any(|input| sender.send(input).is_err()) {
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
                state.render.transparency = heard.transparency;
            }
        })
        .map(|_| ())
        .map_err(|error| crate::StartError::Loop(error.to_string()))
}

/// Runs each task on a thread of its own, parked between wake-ups: ds-settings' watch is an
/// async task and casement runs no async runtime.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThreadSpawner;

impl Spawner for ThreadSpawner {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) {
        let started = std::thread::Builder::new()
            .name("casement-settings-watch".to_owned())
            .spawn(move || block_on(task));
        if let Err(error) = started {
            tracing::warn!(%error, "a settings file is read once: it could not be watched");
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
        assert_eq!(scheme_in(Some(home.path())), Scheme::Light);
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
            assert_eq!(scheme_in(Some(home.path())), *want, "{name}");
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
        let mut heard = Heard::start(Stored {
            theme: Theme::System,
            transparency: Transparency::System,
        });
        assert_eq!(heard.scheme(), Scheme::Light, "before the desktop answers");
        for (input, want) in steps {
            heard = heard.after(input);
            assert_eq!(heard.scheme(), want, "{input:?}");
        }
    }

    #[test]
    fn the_file_picks_the_transparency_and_a_missing_key_is_system() {
        const CASES: &[(&str, &str, Transparency)] = &[
            (
                "reduced",
                "[appearance]\ntransparency = \"reduced\"\n",
                Transparency::Reduced,
            ),
            (
                "system",
                "[appearance]\ntransparency = \"system\"\n",
                Transparency::System,
            ),
            (
                "no key",
                "[appearance]\ntheme = \"dark\"\n",
                Transparency::System,
            ),
        ];
        for (name, text, want) in CASES {
            let home = tempfile::tempdir().expect("a scratch directory");
            let dir = home.path().join("quire");
            std::fs::create_dir_all(&dir).expect("a directory");
            std::fs::write(dir.join("appearance.toml"), text).expect("a file");
            let stored = stored_in(Some(home.path()));
            assert_eq!(stored.transparency, *want, "{name}");
        }
    }

    #[test]
    fn transparency_follows_the_file_whatever_the_scheme_does() {
        let mut heard = Heard::start(Stored {
            theme: Theme::Light,
            transparency: Transparency::System,
        });
        heard = heard.after(Input::Transparency(Transparency::Reduced));
        heard = heard.after(Input::System(Scheme::Dark));
        assert_eq!(heard.transparency, Transparency::Reduced);
        heard = heard.after(Input::Transparency(Transparency::System));
        assert_eq!(heard.transparency, Transparency::System);
    }

    #[test]
    fn only_reduced_replaces_the_blur_and_in_the_scheme_s_paper() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            assert_eq!(
                blur_finish(Transparency::System, scheme),
                BlurFinish::Frosted
            );
        }
        let light = blur_finish(Transparency::Reduced, Scheme::Light);
        let dark = blur_finish(Transparency::Reduced, Scheme::Dark);
        assert!(
            matches!((light, dark), (BlurFinish::Solid(l), BlurFinish::Solid(d)) if l != d && l.a == 245 && d.a == 245)
        );
    }

    #[test]
    fn a_stand_in_home_never_reaches_a_bus() {
        let jail = Path::new("/nonexistent/casement-jail");
        assert_eq!(
            system_source(Some(jail)),
            SystemPrefsSource::Fixed(Default::default())
        );
        assert_eq!(system_source(None), SystemPrefsSource::Portal);
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
