//! `zwlr_screencopy_manager_v1`: `casement_wayland::screencopy` serves the wire; this file
//! answers the copies it queued from the frame each output last presented, the one scanout
//! shows and `ext_image_copy_capture` serves ([`super::image_capture`]).
//!
//! A copy is answered when it is queued and again after every presented frame
//! ([`serve_captures`], which also wakes the `ext_image_copy_capture` frames that waited): one
//! that finds no frame yet, or a `copy_with_damage` that finds nothing newer than the
//! client's last, stays queued.

use super::image_capture;
use crate::state::State;
use casement_core::OutputKey;
use casement_render::pass::CapturedFrame;
use casement_wayland::image_capture::CaptureFailure;
use casement_wayland::screencopy::{
    self, Damage, PendingCopy, ScreencopyHandler, ScreencopyOutput, ScreencopyPart,
};
use smithay::output::Output;
use smithay::reexports::wayland_server::Resource;
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;

impl ScreencopyHandler for State {
    fn screencopy_part(&mut self) -> &mut ScreencopyPart {
        &mut self.wl.screencopy
    }

    fn screencopy_output(&self, output: &WlOutput) -> Option<ScreencopyOutput> {
        let output = &Output::from_resource(output)?;
        let key = self
            .wl
            .output
            .outputs
            .iter()
            .find(|(_, advertised)| *advertised == output)
            .map(|(key, _)| key.clone())?;
        let config = self.core.model.outputs().0.get(&key)?.config;
        Some(ScreencopyOutput {
            key,
            mode: config.mode.size,
            scale: config.scale,
            transform: config.transform,
            dmabuf: image_capture::fourccs(self),
        })
    }

    fn screencopy_wake(&mut self) {
        serve(self);
    }
}

/// Answers what waited for a frame: the `wlr` copies and the `ext` frames, after noting what
/// the frame `output` presented changed and telling the cursor sessions where the cursor is.
/// Run after every frame the loop presents; cheap when nothing waits.
pub(crate) fn serve_captures(state: &mut State, output: &OutputKey) {
    image_capture::note_presented(state, output);
    image_capture::sync_cursors(state, output);
    serve(state);
    image_capture::release_waiting(state);
}

/// Whether a copy is over or waits for a later frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Done,
    Wait,
}

/// Answers each queued copy that can be; the rest stay queued.
fn serve(state: &mut State) {
    for copy in state.wl.screencopy.take_pending() {
        match answer(state, &copy) {
            Outcome::Done => {}
            Outcome::Wait => state.wl.screencopy.keep(copy),
        }
    }
}

/// Writes the frame into `copy`'s buffer and tells the client, or says why not.
fn answer(state: &mut State, copy: &PendingCopy) -> Outcome {
    if !copy.frame.is_alive() {
        return Outcome::Done;
    }
    let (captured, at) = match image_capture::last_frame(state, &copy.target.output, copy.cursor) {
        Ok(Some(found)) => found,
        Ok(None) => return Outcome::Wait,
        Err(why) => return failed(copy, why),
    };
    // `copy_with_damage` answers once something changed in what it copies since the client's
    // last frame (the whole area for its first); a plain `copy` answers at once.
    let damage = match copy.damage {
        Damage::Fresh => {
            let last = state.wl.screencopy.last_delivery(copy);
            let since = state
                .wl
                .image_capture
                .damage
                .since(&copy.target.output, last);
            since.within(copy.target.area)
        }
        _ => Vec::new(),
    };
    if copy.damage == Damage::Fresh && damage.is_empty() {
        return Outcome::Wait;
    }
    match write(state, copy, &captured) {
        Ok(()) => {
            screencopy::apply_ready(&mut state.wl.screencopy, copy, at, &damage);
            Outcome::Done
        }
        Err(why) => failed(copy, why),
    }
}

/// The part of `captured` that `copy` names, written into its buffer.
fn write(
    state: &mut State,
    copy: &PendingCopy,
    captured: &CapturedFrame,
) -> Result<(), CaptureFailure> {
    let part = captured
        .cropped(copy.target.area)
        .ok_or(CaptureFailure::BufferMismatch)?;
    image_capture::deliver(state, &copy.buffer, &part).map_err(|error| {
        tracing::warn!(%error, "a screencopy frame was not written");
        CaptureFailure::BufferMismatch
    })
}

/// Tells the client `failed`; the copy is over.
fn failed(copy: &PendingCopy, why: CaptureFailure) -> Outcome {
    tracing::debug!(?why, "a screencopy frame failed");
    screencopy::apply_failed(copy);
    Outcome::Done
}
