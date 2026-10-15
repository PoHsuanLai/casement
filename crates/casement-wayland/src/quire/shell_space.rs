//! `quire_shell_v1` version 2, the Spaces bar's part: which outputs Mission Control is up on
//! (`overview_output`), the rectangles a dragged window can be dropped on (`space_target_clear`,
//! `space_target`) and the Space under it (`drop_target`). The picture of a Space is
//! [`super::space_shot`]. The model and the loop decide; this only carries.

use super::shell::{QuireShellHandler, QuireShellPart};
use casement_core::OutputKey;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::overview_drag::SpaceTarget;
use casement_protocols::server::quire_shell_v1::quire_shell_v1;
use porter_core::SpaceId;
use smithay::output::Output;
use smithay::reexports::wayland_server::{DataInit, Resource};
use smithay::wayland::image_capture_source::{ImageCaptureSource, ImageCaptureSourceData};
use std::collections::BTreeMap;

/// Why a Space target was refused (a protocol error to the client).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SpaceTargetRefusal {
    /// The rectangle has no width or no height (`bad_target`).
    Empty,
}

/// `space_target_clear`: forget the targets of `output` (an output the compositor does not
/// have clears nothing).
///
/// ```
/// use casement_wayland::quire::shell_space::on_space_target_clear;
/// # fn demo(part: &mut casement_wayland::quire::shell::QuireShellPart) {
/// on_space_target_clear(part, None);
/// # }
/// ```
pub fn on_space_target_clear(part: &mut QuireShellPart, output: Option<OutputKey>) {
    if let Some(output) = output {
        part.space_targets.clear(&output);
    }
}

/// `space_target`: the rectangle `x`, `y`, `width`, `height` (global logical pixels) moves a
/// dropped window to `space`. An empty rectangle is refused; an output or a Space the
/// compositor does not know is kept out (`None`) without an error, since either may be gone by
/// the time the request arrives.
///
/// ```
/// use casement_wayland::quire::shell_space::{SpaceTargetRefusal, target_rect};
/// assert!(target_rect(0, 0, 160, 90).is_ok());
/// assert_eq!(target_rect(0, 0, 0, 90), Err(SpaceTargetRefusal::Empty));
/// ```
pub fn on_space_target(
    part: &mut QuireShellPart,
    output: Option<OutputKey>,
    space: Option<SpaceId>,
    rect: LogicalRect,
) {
    if let Some((output, space)) = output.zip(space) {
        part.space_targets
            .add(output, SpaceTarget::new(space, rect));
    }
}

/// The rectangle of a `space_target`, or why it is refused.
pub fn target_rect(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<LogicalRect, SpaceTargetRefusal> {
    match width == 0 || height == 0 {
        true => Err(SpaceTargetRefusal::Empty),
        false => Ok(LogicalRect::new(
            LogicalPoint { x, y },
            LogicalSize {
                w: width,
                h: height,
            },
        )),
    }
}

/// The shell objects are gone: nothing is left to drop on.
pub fn forget_targets(part: &mut QuireShellPart) {
    if part.bound.is_empty() {
        part.space_targets.clear_all();
    }
}

/// `overview_output` for every output, to every bound object of version 2 or later.
pub fn send_overview_outputs(
    part: &QuireShellPart,
    outputs: &BTreeMap<OutputKey, Output>,
    shown: quire_shell_v1::Shown,
) {
    for shell in part.bound.iter().filter(|shell| shell.version() >= 2) {
        let Some(client) = shell.client() else {
            continue;
        };
        for output in outputs.values() {
            for wl_output in output.client_outputs(&client) {
                shell.overview_output(&wl_output, shown);
            }
        }
    }
}

/// `drop_target`: the Space under the dragged window on `output`, or none.
pub fn send_drop_target(
    part: &QuireShellPart,
    outputs: &BTreeMap<OutputKey, Output>,
    output: &OutputKey,
    space: Option<&SpaceId>,
) {
    let Some(output) = outputs.get(output) else {
        return;
    };
    for shell in part.bound.iter().filter(|shell| shell.version() >= 2) {
        let Some(client) = shell.client() else {
            continue;
        };
        for wl_output in output.client_outputs(&client) {
            shell.drop_target(&wl_output, space.map(|space| space.as_str().to_owned()));
        }
    }
}

/// A version 2 request of the shell; every other request is ignored here.
pub fn on_request<D: QuireShellHandler>(
    state: &mut D,
    resource: &quire_shell_v1::QuireShellV1,
    request: quire_shell_v1::Request,
    data_init: &mut DataInit<'_, D>,
) {
    use quire_shell_v1::Request;
    match request {
        Request::SpaceTargetClear { output } => {
            let output = state.output_key_of(&output);
            on_space_target_clear(state.quire_shell_part(), output);
        }
        Request::SpaceTarget {
            output,
            space_id,
            x,
            y,
            width,
            height,
        } => match target_rect(x, y, width, height) {
            Ok(rect) => {
                let output = state.output_key_of(&output);
                let space = SpaceId::parse(&space_id).ok();
                on_space_target(state.quire_shell_part(), output, space, rect);
            }
            Err(SpaceTargetRefusal::Empty) => resource.post_error(
                quire_shell_v1::Error::BadTarget,
                "a Space target needs a width and a height",
            ),
        },
        Request::GetSpaceCaptureSource {
            source,
            output,
            space_id,
            width,
            height,
        } => {
            let output = state.output_key_of(&output);
            let space = SpaceId::parse(&space_id)
                .ok()
                .filter(|space| state.space_known(space));
            let capture = ImageCaptureSource::new();
            if let Some((output, space)) = output.zip(space) {
                let id = state
                    .quire_shell_part()
                    .shots
                    .book
                    .on_source(output, space, width, height);
                capture.user_data().insert_if_missing_threadsafe(|| id);
            }
            let made = data_init.init(
                source,
                ImageCaptureSourceData {
                    source: capture.clone(),
                },
            );
            capture.add_instance(&made);
        }
        _ => {}
    }
}
