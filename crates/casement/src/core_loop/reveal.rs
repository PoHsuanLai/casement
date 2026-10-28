//! Edge reveal in the loop: in full screen the bar and the dock come over the window when the
//! pointer rests at their edge (`casement_core::reveal`). The loop tells each machine where the
//! pointer is, whether the output shows a full-screen Space and whether a pop-up of the surface
//! is open; it carries out the timer and the `reveal` note the machine asks for; and it says
//! where each raised surface is drawn and hit now. The full-screen window is never told by the
//! reveal: the surfaces overlay it. With the shell's `keep` (`FullScreenBar::Keep`) the top edge
//! never reveals: layout sizes the window below the bar, and the bar is raised above the window
//! at its place for as long as the output shows a full-screen Space.

use super::scene::{Hit, hit_test};
use crate::exec;
use crate::state::State;
use casement_core::geometry::{LogicalPoint, LogicalRect};
use casement_core::reveal::{Place, RevealEdge, RevealIn, RevealOut, RevealTiming, slid};
use casement_core::shell::ShellNote;
use casement_core::space::SpaceContent;
use casement_core::time::{Due, Mono, TimerKey};
use casement_core::{Effect, Event, LayerKey, Model, OutputKey};
use casement_render::RaisedDraw;
use casement_wayland::seat::Candidate;
use ds_core::vocab::Fraction;
use smithay::desktop::PopupManager;

/// The surface of `output` that holds the role `edge` reveals: each output has its own bar and
/// its own dock.
fn holder(model: &Model, output: &OutputKey, edge: RevealEdge) -> Option<LayerKey> {
    model.layers().role_holder_on(edge.role(), output)
}

/// Whether `output` shows a full-screen Space and has a surface to reveal, whatever the bar does
/// in full screen.
fn full_screen_cover(model: &Model, output: &OutputKey, surface: Option<LayerKey>) -> bool {
    let full_screen = model
        .spaces()
        .shown_on(output)
        .is_some_and(|space| matches!(space.content, SpaceContent::FullScreen(_)));
    full_screen && surface.is_some()
}

/// Whether `edge` of `output` reveals now: the output shows a full-screen Space with a surface
/// to reveal, and the shell has not chosen to keep the bar up.
fn cover_of(
    model: &Model,
    output: &OutputKey,
    edge: RevealEdge,
    surface: Option<LayerKey>,
) -> bool {
    edge.covered(
        model.full_screen_bar(),
        full_screen_cover(model, output, surface),
    )
}

/// Whether the surface of `edge` of `output` stays up above the full-screen window (the shell's
/// `keep`): the output shows a full-screen Space and the edge does not reveal.
fn kept_on(model: &Model, output: &OutputKey, edge: RevealEdge, surface: Option<LayerKey>) -> bool {
    let shown = full_screen_cover(model, output, surface);
    shown && !cover_of(model, output, edge, surface)
}

/// Whether `surface` has a pop-up open.
fn popup_of(state: &State, surface: Option<LayerKey>) -> bool {
    surface
        .and_then(|layer| state.render.textures.layer(&layer))
        .is_some_and(|entry| {
            PopupManager::popups_for_surface(&entry.root)
                .next()
                .is_some()
        })
}

/// The machine of `edge` of `output` hears `input`; what it asks for is carried out.
fn step(state: &mut State, output: &OutputKey, edge: RevealEdge, input: RevealIn) {
    let now = state.clock.now();
    let timing = RevealTiming::of(edge, state.core.model.reduced_motion());
    let book = std::mem::take(&mut state.render.reveal);
    let (book, outs) = book.step(output, edge, timing, input, now);
    state.render.reveal = book;
    let key = TimerKey::Reveal(output.clone(), edge);
    let effects: Vec<Effect> = outs
        .into_iter()
        .map(|out| match out {
            RevealOut::Arm(at) => Effect::ArmTimer(Due {
                at,
                key: key.clone(),
            }),
            RevealOut::Cancel => Effect::CancelTimer(key.clone()),
            RevealOut::Tell(shown) => {
                Effect::Shell(ShellNote::Reveal(output.clone(), edge.screen_edge(), shown))
            }
        })
        .collect();
    exec::exec(state, effects);
}

/// The machines hear what the model and the surfaces say now: which outputs show a full-screen
/// Space, which surfaces have a pop-up open, and which outputs are gone.
pub(super) fn sync(state: &mut State) {
    let outputs = state.core.model.output_rects();
    let book = std::mem::take(&mut state.render.reveal);
    let present = |key: &OutputKey| outputs.iter().any(|(output, _)| output == key);
    state.render.reveal = book.retain_outputs(present);
    for (output, _) in &outputs {
        for edge in RevealEdge::ALL {
            let surface = holder(&state.core.model, output, edge);
            let cover = cover_of(&state.core.model, output, edge, surface);
            let popup = popup_of(state, surface);
            let machine = state.render.reveal.get(output, edge);
            if machine.cover() != cover {
                step(state, output, edge, RevealIn::Cover(cover));
            }
            if machine.popup() != popup {
                step(state, output, edge, RevealIn::Popup(popup));
            }
        }
    }
}

/// The pointer is at `at`; `stacked` is what it can hit (`scene::stack`).
pub(super) fn moved(state: &mut State, stacked: &[(Hit, Candidate)], at: LogicalPoint) {
    let hit = hit_test(stacked, at).map(|(hit, _)| hit);
    for (output, area) in state.core.model.output_rects() {
        for edge in RevealEdge::ALL {
            let surface = holder(&state.core.model, &output, edge);
            let over_surface =
                matches!((&hit, surface), (Some(Hit::Layer(hit)), Some(layer)) if *hit == layer);
            let place = Place::of(edge, area, at, over_surface);
            if state.render.reveal.get(&output, edge).place() != place {
                step(state, &output, edge, RevealIn::Pointer(place));
            }
        }
    }
}

/// A reveal timer fired in `events`: the machine looks again.
pub(super) fn hear(state: &mut State, events: &[Event]) {
    for event in events {
        if let Event::TimerFired(TimerKey::Reveal(output, edge)) = event
            && state.core.model.outputs().0.contains_key(output)
        {
            step(state, output, *edge, RevealIn::Tick);
        }
    }
}

/// The surfaces raised over the full-screen windows at `now`, each where it is drawn and hit:
/// the one place the renderer and the hit test both read.
pub(super) fn raised(state: &State, now: Mono) -> Vec<RaisedDraw> {
    let mut found = Vec::new();
    for (output, area) in state.core.model.output_rects() {
        for edge in RevealEdge::ALL {
            found.extend(raised_on(state, &output, area, edge, now));
        }
    }
    found
}

/// The surface `edge` of `output` reveals, when it is above the full-screen window: sliding, shown
/// or fading, or (the bar when the shell keeps it) at its place.
fn raised_on(
    state: &State,
    output: &OutputKey,
    area: LogicalRect,
    edge: RevealEdge,
    now: Mono,
) -> Option<RaisedDraw> {
    let model = &state.core.model;
    let layer = holder(model, output, edge)?;
    let kept = kept_on(model, output, edge, Some(layer));
    let machine = state.render.reveal.get(output, edge);
    if !kept && !machine.raised() {
        return None;
    }
    let entry = state.render.textures.layer(&layer)?;
    let rests = model
        .layers()
        .arrange(output, area)
        .place(&layer, entry.committed)?;
    if kept {
        return Some(RaisedDraw {
            layer,
            area: rests,
            opacity: Fraction(1000),
        });
    }
    let timing = RevealTiming::of(edge, model.reduced_motion());
    let drawn = slid(edge, timing.look, area, rests, machine.visible(timing, now));
    Some(RaisedDraw {
        layer,
        area: drawn.area,
        opacity: drawn.opacity,
    })
}
