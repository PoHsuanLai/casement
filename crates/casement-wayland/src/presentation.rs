//! `wp_presentation`: when a frame reached the screen. Timestamps are the clock's `Mono`
//! (virtual in tests), never the wall clock.

use crate::states::{GlobalFilter, WaylandFault};
use casement_core::time::{Millihertz, Mono, VblankSeq};
use casement_core::{Event, OutputKey};
use smithay::output::{Output, PhysicalProperties, Subpixel};
use smithay::reexports::wayland_protocols::wp::presentation_time::server::{
    wp_presentation::WpPresentation,
    wp_presentation_feedback::{Kind, WpPresentationFeedback},
};
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Dispatch, DisplayHandle, GlobalDispatch, Weak};
use smithay::wayland::GlobalData;
use smithay::wayland::compositor::with_states;
use smithay::wayland::presentation::{
    PresentationData, PresentationFeedbackCachedState, PresentationFeedbackCallback,
    PresentationState, Refresh,
};
use std::collections::BTreeMap;

/// The `clk_id` casement advertises: `CLOCK_MONOTONIC` (the session's `Mono` counts from a
/// monotonic epoch; the virtual clock stands in for it in tests).
pub const CLOCK_MONOTONIC: u32 = 1;

/// `wp_presentation`.
#[derive(Debug)]
pub struct PresentationPart {
    /// smithay's state.
    pub state: PresentationState,
    /// The outputs the presenting side told us about ([`learn_output`]): feedback names the
    /// output it was shown on, and smithay's feedback needs the `Output` to do it.
    outputs: BTreeMap<OutputKey, Output>,
    /// Feedback collected at commit, waiting for the next frame.
    waiting: Vec<Waiting>,
}

/// One feedback object and the surface whose content update it asked about.
#[derive(Debug)]
struct Waiting {
    surface: Weak<WlSurface>,
    callback: PresentationFeedbackCallback,
}

impl PresentationPart {
    /// Creates the global (version 2). Every connection sees it.
    pub fn new<D>(display: &DisplayHandle, role_filter: GlobalFilter) -> PresentationPart
    where
        D: GlobalDispatch<WpPresentation, PresentationData>
            + Dispatch<WpPresentation, PresentationData>
            + Dispatch<WpPresentationFeedback, GlobalData>
            + 'static,
    {
        let _ = role_filter;
        PresentationPart {
            state: PresentationState::new::<D>(display, CLOCK_MONOTONIC),
            outputs: BTreeMap::new(),
            waiting: Vec::new(),
        }
    }
}

/// One presented frame on one output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Presented {
    /// The vblank it was shown at.
    pub seq: VblankSeq,
    /// When.
    pub at: Mono,
    /// The output's refresh then.
    pub refresh: Millihertz,
}

/// Tells the part which smithay `Output` stands for `key`, so feedback can say where the frame
/// was shown (`sync_output`). Called when an output is plugged, with the `Output` that
/// `output::apply_plug` made; without it feedback is still sent, without `sync_output`.
pub fn learn_output(part: &mut PresentationPart, key: OutputKey, output: Output) {
    part.outputs.insert(key, output);
}

/// Forgets an unplugged output.
pub fn forget_output(part: &mut PresentationPart, key: &OutputKey) {
    part.outputs.remove(key);
}

/// A committed surface asked for presentation feedback: the callbacks it committed are
/// collected for the next frame. Callbacks of an earlier commit of the same surface that was
/// never shown are discarded (superseded, as the protocol says). Nothing reaches the model.
pub fn on_feedback(part: &mut PresentationPart, surface: &WlSurface) -> Vec<Event> {
    let fresh = with_states(surface, |states| {
        std::mem::take(
            &mut states
                .cached_state
                .get::<PresentationFeedbackCachedState>()
                .current()
                .callbacks,
        )
    });
    if fresh.is_empty() {
        return Vec::new();
    }
    let (superseded, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut part.waiting)
        .into_iter()
        .partition(|waiting| waiting.surface == surface.downgrade());
    part.waiting = kept;
    superseded
        .into_iter()
        .for_each(|waiting| waiting.callback.discarded());
    part.waiting
        .extend(fresh.into_iter().map(|callback| Waiting {
            surface: surface.downgrade(),
            callback,
        }));
    Vec::new()
}

/// Discards every feedback collected for `surface` (it was destroyed or covered).
pub fn discard_surface(part: &mut PresentationPart, surface: &WlSurface) {
    let (gone, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut part.waiting)
        .into_iter()
        .partition(|waiting| waiting.surface == surface.downgrade());
    part.waiting = kept;
    gone.into_iter()
        .for_each(|waiting| waiting.callback.discarded());
}

/// The refresh as smithay's feedback takes it: a fixed period, unknown when the output reports
/// none.
fn refresh_of(rate: Millihertz) -> Refresh {
    match rate.0 {
        0 => Refresh::Unknown,
        _ => Refresh::Fixed(rate.period()),
    }
}

/// The output new surfaces live on, and so the one whose frames carry their feedback (P0:
/// every window is on the first output, A28): the top-left learned output. With none learned,
/// any presenting output serves.
fn primary(outputs: &BTreeMap<OutputKey, Output>) -> Option<&OutputKey> {
    outputs
        .iter()
        .min_by_key(|(key, output)| {
            let at = output.current_location();
            (at.x, at.y, *key)
        })
        .map(|(key, _)| key)
}

/// An `Output` nobody advertises, for feedback when the presenting side never said which
/// output it was (no `sync_output` reaches the client).
fn detached() -> Output {
    Output::new(
        "detached".to_owned(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: String::new(),
            model: String::new(),
            serial_number: String::new(),
        },
    )
}

/// Sends `presented` to every feedback collected for `output`'s last frame; feedback whose
/// surface is gone is discarded.
pub fn apply_presented(
    part: &mut PresentationPart,
    output: &OutputKey,
    presented: Presented,
) -> Result<(), WaylandFault> {
    if primary(&part.outputs).is_some_and(|primary| primary != output) {
        return Ok(());
    }
    let shown_on = part.outputs.get(output).cloned().unwrap_or_else(detached);
    let refresh = refresh_of(presented.refresh);
    for waiting in std::mem::take(&mut part.waiting) {
        if waiting.surface.upgrade().is_err() {
            waiting.callback.discarded();
        } else {
            waiting.callback.presented(
                &shown_on,
                presented.at.0,
                refresh,
                presented.seq.0,
                Kind::Vsync,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refresh_is_fixed_unless_the_output_reports_none() {
        assert_eq!(refresh_of(Millihertz(0)), Refresh::Unknown);
        assert_eq!(
            refresh_of(Millihertz(60_000)),
            Refresh::Fixed(Millihertz(60_000).period())
        );
    }

    #[test]
    fn the_top_left_learned_output_is_primary() {
        let at = |name: &str, x: i32| {
            let output = Output::new(
                name.to_owned(),
                PhysicalProperties {
                    size: (0, 0).into(),
                    subpixel: Subpixel::Unknown,
                    make: String::new(),
                    model: String::new(),
                    serial_number: String::new(),
                },
            );
            output.change_current_state(None, None, None, Some((x, 0).into()));
            output
        };
        let key = |name: &str| {
            use casement_core::output::{EdidFacts, EdidSerial};
            let edid = EdidFacts {
                make: String::new(),
                model: String::new(),
                serial: EdidSerial::Unknown,
            };
            OutputKey::of(
                &edid,
                &casement_core::OutputName::parse(name).expect("a name"),
            )
        };
        assert_eq!(primary(&BTreeMap::new()), None);
        let outputs: BTreeMap<_, _> = [
            (key("HEADLESS-1"), at("HEADLESS-1", 1920)),
            (key("HEADLESS-2"), at("HEADLESS-2", 0)),
        ]
        .into_iter()
        .collect();
        assert_eq!(primary(&outputs), Some(&key("HEADLESS-2")));
    }
}
