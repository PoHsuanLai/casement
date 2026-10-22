//! `quire_shell_v1` version 7: what the bar does in full screen (`set_full_screen_bar`), how much
//! motion the person allows (`set_motion`), and the edge reveal that names its output
//! (`reveal_on_output`, sent in place of `reveal` to a shell that bound version 7). The model
//! decides; this only carries.

use super::shell::QuireShellPart;
use casement_core::shell::ShellRequest;
use casement_core::{Event, OutputKey};
use casement_protocols::server::quire_shell_v1::quire_shell_v1;
use casement_protocols::vocab::{FullScreenBar, Motion, ScreenEdge, Shown, WireCode};
use smithay::output::Output;
use smithay::reexports::wayland_server::Resource;
use std::collections::BTreeMap;

/// The first version that hears `reveal_on_output` in place of `reveal`.
const NAMES_OUTPUT: u32 = 7;

/// `set_full_screen_bar`: what the bar does in full screen. A value the XML does not name asks
/// for nothing.
///
/// ```
/// use casement_wayland::quire::shell_screen::on_set_full_screen_bar;
/// assert_eq!(on_set_full_screen_bar(Some(1)).len(), 1);
/// assert!(on_set_full_screen_bar(Some(9)).is_empty());
/// assert!(on_set_full_screen_bar(None).is_empty());
/// ```
pub fn on_set_full_screen_bar(mode: Option<u32>) -> Vec<Event> {
    mode.and_then(|code| FullScreenBar::from_code(code).ok())
        .map(|mode| Event::Shell(ShellRequest::FullScreenBar(mode)))
        .into_iter()
        .collect()
}

/// `set_motion`: how much motion the person allows. A value the XML does not name asks for
/// nothing.
///
/// ```
/// use casement_wayland::quire::shell_screen::on_set_motion;
/// assert_eq!(on_set_motion(Some(1)).len(), 1);
/// assert!(on_set_motion(Some(9)).is_empty());
/// ```
pub fn on_set_motion(motion: Option<u32>) -> Vec<Event> {
    motion
        .and_then(|code| Motion::from_code(code).ok())
        .map(|motion| Event::Shell(ShellRequest::Motion(motion)))
        .into_iter()
        .collect()
}

/// An edge reveal of `output` changed: `reveal_on_output` (naming the shell's `wl_output` for
/// it) to every bound object of version 7 or later, `reveal` to the older ones.
pub fn send_reveal(
    part: &QuireShellPart,
    outputs: &BTreeMap<OutputKey, Output>,
    output: &OutputKey,
    edge: ScreenEdge,
    shown: Shown,
) {
    let wire = quire_shell_v1::ScreenEdge::try_from(edge.code())
        .ok()
        .zip(quire_shell_v1::Shown::try_from(shown.code()).ok());
    let Some((edge, shown)) = wire else {
        return;
    };
    for shell in part
        .bound
        .iter()
        .filter(|shell| shell.version() < NAMES_OUTPUT)
    {
        shell.reveal(edge, shown);
    }
    let Some(output) = outputs.get(output) else {
        return;
    };
    for shell in part
        .bound
        .iter()
        .filter(|shell| shell.version() >= NAMES_OUTPUT)
    {
        let Some(client) = shell.client() else {
            continue;
        };
        for wl_output in output.client_outputs(&client) {
            shell.reveal_on_output(&wl_output, edge, shown);
        }
    }
}
