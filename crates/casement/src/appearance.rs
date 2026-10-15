//! The person's light or dark scheme, read from quire's `appearance.toml` through ds-settings
//! and followed while casement runs. The server-side window frame is drawn in it
//! (`core_loop::frames::styles` reads `state.render.scheme` every frame, so a change shows at
//! the next vblank).
//!
//! `appearance.theme = "system"` would follow the desktop's preference over the settings
//! portal, which casement does not build in (ds-settings' `quire-desktop` feature is off for
//! it), so `system` paints the light scheme; see [`casement_layout::scheme_of`].

use crate::State;
use crate::cli::ConfigHome;
use casement_layout::{Scheme, scheme_of};
use ds_core::spawner::Spawner;
use ds_settings::{AppName, AppearanceFile, ConfigRoot, Store};
use smithay::reexports::calloop::channel::{self, Event as ChannelEvent};
use std::future::Future;
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

/// The scheme `home`'s appearance file paints now (the light scheme when there is no file).
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
    let loaded = store_for(home).load::<AppearanceFile>();
    for key in &loaded.invalid {
        tracing::warn!("appearance.toml: {key}");
    }
    scheme_of(loaded.value.appearance.theme)
}

/// Reads the scheme into `state` now, and again whenever the file changes. A file that cannot
/// be watched is logged and the scheme read once is kept.
pub(crate) fn attach(state: &mut State, home: &ConfigHome) -> Result<(), crate::StartError> {
    state.render.scheme = scheme_in(home);
    let (sender, receiver) = channel::channel::<Scheme>();
    let mut watch = store_for(home).watch::<AppearanceFile>(&ThreadSpawner);
    ThreadSpawner.spawn(Box::pin(async move {
        while let Some(loaded) = watch.changed().await {
            if sender
                .send(scheme_of(loaded.value.appearance.theme))
                .is_err()
            {
                return;
            }
        }
    }));
    state
        .event_loop
        .clone()
        .insert_source(receiver, |event, (), state: &mut State| {
            if let ChannelEvent::Msg(scheme) = event {
                state.render.scheme = scheme;
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
    fn block_on_finishes_a_ready_task() {
        let (sender, receiver) = std::sync::mpsc::channel();
        block_on(Box::pin(async move {
            let _ = sender.send(7);
        }));
        assert_eq!(receiver.recv(), Ok(7));
    }
}
