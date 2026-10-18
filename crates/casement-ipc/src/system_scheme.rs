//! Following the desktop's colour scheme over the settings portal: a thread at the binary's edge
//! that reads the scheme once and then tells the compositor each time it changes, as `sleep` and
//! `signals` do for their buses. The compositor names no bus type: the one D-Bus crate is this
//! one (A4).
//!
//! The source is a seam: [`SystemPrefsSource::Portal`] reaches the session bus; a test passes
//! [`SystemPrefsSource::Fixed`] and never does.
//!
//! ```
//! use casement_ipc::system_scheme::{DesktopScheme, SchemeWatch};
//! use ds_settings::SystemPrefsSource;
//! use std::ops::ControlFlow;
//! use std::sync::mpsc;
//!
//! let (tx, rx) = mpsc::channel();
//! let _watch = SchemeWatch::start(SystemPrefsSource::Fixed(Default::default()), move |scheme| {
//!     match tx.send(scheme) {
//!         Ok(()) => ControlFlow::Continue(()),
//!         Err(_) => ControlFlow::Break(()),
//!     }
//! })
//! .expect("a thread starts");
//! assert_eq!(rx.recv(), Ok(DesktopScheme::Light));
//! ```

use ds_core::spawner::Spawner;
use ds_settings::{SystemPrefsSource, SystemPrefsWatch, scheme_from_portal};
use std::future::Future;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::thread::JoinHandle;
use tokio::sync::oneshot;

/// Why the watcher could not start.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SchemeError {
    /// The runtime or its thread could not be started.
    #[error("the colour scheme watcher: {0}")]
    Thread(#[source] std::io::Error),
}

/// The desktop's colour scheme, as the portal's `color-scheme` says it (no preference is light).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DesktopScheme {
    /// Light, or no preference.
    Light,
    /// The person prefers dark.
    Dark,
}

impl DesktopScheme {
    /// The scheme the portal's `color-scheme` value `code` means: 1 is dark, anything else light.
    ///
    /// ```
    /// use casement_ipc::system_scheme::DesktopScheme;
    ///
    /// assert_eq!(DesktopScheme::of_portal(1), DesktopScheme::Dark);
    /// assert_eq!(DesktopScheme::of_portal(0), DesktopScheme::Light);
    /// ```
    pub fn of_portal(code: u32) -> DesktopScheme {
        pick(&scheme_from_portal(code), &scheme_from_portal(1))
    }
}

/// Runs the portal's task on the watcher's own runtime.
struct InRuntime(tokio::runtime::Handle);

impl Spawner for InRuntime {
    fn spawn(&self, task: Pin<Box<dyn Future<Output = ()> + Send>>) {
        drop(self.0.spawn(task));
    }
}

/// The running watcher; dropping it stops the thread.
#[derive(Debug)]
pub struct SchemeWatch {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl SchemeWatch {
    /// Reads `source`'s colour scheme and passes it to `tell` (from the watcher's thread), then
    /// passes each change. `tell` returning `Break` ends the watch. A desktop without the portal
    /// is the default scheme once and never a change.
    pub fn start(
        source: SystemPrefsSource,
        tell: impl FnMut(DesktopScheme) -> ControlFlow<()> + Send + 'static,
    ) -> Result<SchemeWatch, SchemeError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(SchemeError::Thread)?;
        let (stop, stopped) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("casement-scheme".to_owned())
            .spawn(move || runtime.block_on(follow(source, tell, stopped)))
            .map_err(SchemeError::Thread)?;
        Ok(SchemeWatch {
            stop: Some(stop),
            thread: Some(thread),
        })
    }
}

impl Drop for SchemeWatch {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// `scheme` as ours, given the portal's dark scheme to compare with (this crate names no
/// ds-style type, so the value is compared, never matched).
fn pick<S: PartialEq>(scheme: &S, dark: &S) -> DesktopScheme {
    if scheme == dark {
        DesktopScheme::Dark
    } else {
        DesktopScheme::Light
    }
}

/// The first answer, then every change, until `tell` breaks, the portal goes or `stopped` fires.
async fn follow(
    source: SystemPrefsSource,
    mut tell: impl FnMut(DesktopScheme) -> ControlFlow<()>,
    mut stopped: oneshot::Receiver<()>,
) {
    let spawner = InRuntime(tokio::runtime::Handle::current());
    let mut watch = SystemPrefsWatch::start(&source, &spawner).await;
    if tell(pick(&watch.current().scheme, &scheme_from_portal(1))).is_break() {
        return;
    }
    loop {
        tokio::select! {
            _ = &mut stopped => return,
            next = watch.changed() => match next {
                Some(prefs) => {
                    if tell(pick(&prefs.scheme, &scheme_from_portal(1))).is_break() {
                        return;
                    }
                }
                None => return,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    /// A fixed source preferring the portal's `color-scheme` `code`.
    fn fixed(code: u32) -> SystemPrefsSource {
        let mut source = SystemPrefsSource::Fixed(Default::default());
        if let SystemPrefsSource::Fixed(prefs) = &mut source {
            prefs.scheme = scheme_from_portal(code);
        }
        source
    }

    fn watching(source: SystemPrefsSource) -> (SchemeWatch, mpsc::Receiver<DesktopScheme>) {
        let (tx, rx) = mpsc::channel();
        let watch = SchemeWatch::start(source, move |scheme| match tx.send(scheme) {
            Ok(()) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(()),
        })
        .expect("a thread starts");
        (watch, rx)
    }

    #[test]
    fn the_portal_codes_map_to_schemes() {
        const CASES: &[(u32, DesktopScheme)] = &[
            (0, DesktopScheme::Light),
            (1, DesktopScheme::Dark),
            (2, DesktopScheme::Light),
            (99, DesktopScheme::Light),
        ];
        for (code, want) in CASES {
            assert_eq!(
                DesktopScheme::of_portal(*code),
                *want,
                "color-scheme {code}"
            );
        }
    }

    #[test]
    fn a_fixed_source_answers_its_scheme_and_never_touches_the_bus() {
        for code in [0, 1, 2] {
            let (_watch, rx) = watching(fixed(code));
            assert_eq!(rx.recv(), Ok(DesktopScheme::of_portal(code)), "code {code}");
        }
    }

    #[test]
    fn a_fixed_source_never_changes() {
        let (watch, rx) = watching(fixed(1));
        assert_eq!(rx.recv(), Ok(DesktopScheme::Dark));
        drop(watch);
        assert!(rx.recv().is_err(), "nothing more, and the thread is gone");
    }
}
