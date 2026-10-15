//! `quire_shell_v1`, the overview part: the shell's `overview` request (its Mission Control key
//! or gesture) and the `overview_state` event that tells it when to draw its Spaces bar. The
//! model decides; this only carries.

use super::shell::QuireShellPart;
use casement_core::Event;
use casement_core::overview::Overview;
use casement_core::shell::ShellRequest;
use casement_protocols::server::quire_shell_v1::quire_shell_v1;
use casement_protocols::vocab::{OverviewMode, OverviewState, Toggle, WireCode};

/// The thousandths of progress an open overview reports.
const OPEN: u32 = 1000;

/// `overview`: open (on) or close (off) the overview `mode`. A value the XML does not name
/// asks for nothing.
///
/// ```
/// use casement_wayland::quire::shell_overview::on_overview;
/// assert_eq!(on_overview(Some(0), Some(1)).len(), 1);
/// assert!(on_overview(Some(9), Some(1)).is_empty());
/// assert!(on_overview(None, Some(1)).is_empty());
/// ```
pub fn on_overview(mode: Option<u32>, toggle: Option<u32>) -> Vec<Event> {
    mode.and_then(|code| OverviewMode::from_code(code).ok())
        .zip(toggle.and_then(|code| Toggle::from_code(code).ok()))
        .map(|(mode, toggle)| Event::Shell(ShellRequest::Overview { mode, toggle }))
        .into_iter()
        .collect()
}

/// `overview_state` for Mission Control, to every bound shell object: the state, and 0 or 1000
/// thousandths for the windows at rest on the desktop or in their slots.
pub fn send_state(part: &QuireShellPart, overview: Overview) {
    let (state, progress) = match overview {
        Overview::Entering => (OverviewState::Entering, 0),
        Overview::Open => (OverviewState::Open, OPEN),
        Overview::Leaving => (OverviewState::Leaving, OPEN),
        _ => (OverviewState::Closed, 0),
    };
    let wire = quire_shell_v1::OverviewMode::try_from(OverviewMode::MissionControl.code())
        .ok()
        .zip(quire_shell_v1::OverviewState::try_from(state.code()).ok());
    if let Some((mode, state)) = wire {
        for shell in &part.bound {
            shell.overview_state(mode, state, progress);
        }
    }
}
