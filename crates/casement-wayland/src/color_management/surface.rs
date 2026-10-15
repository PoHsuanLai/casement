//! `wp_color_management_surface_v1` and its feedback object: a surface's committed colour tag
//! (double buffered with the surface) and its preferred description (always sRGB).

use super::ColorHandler;
use super::creator::{DescriptionData, InfoAllowed, describe};
use casement_core::output::ColorTag;
use smithay::reexports::wayland_protocols::wp::color_management::v1::server::{
    wp_color_management_surface_feedback_v1::{self, WpColorManagementSurfaceFeedbackV1},
    wp_color_management_surface_v1::{self, WpColorManagementSurfaceV1},
    wp_color_manager_v1::RenderIntent,
    wp_image_description_v1::WpImageDescriptionV1,
};
use smithay::reexports::wayland_server::backend::ClientId;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, Resource, WEnum,
};
use smithay::wayland::Dispatch2;
use smithay::wayland::compositor::{Cacheable, with_states};
use std::sync::atomic::{AtomicBool, Ordering};

/// What a surface has committed: its tag, none for an untagged one (handled as sRGB).
#[derive(Debug, Clone, Copy, Default)]
struct SurfaceColor {
    tag: Option<ColorTag>,
}

impl Cacheable for SurfaceColor {
    fn commit(&mut self, _dh: &DisplayHandle) -> Self {
        *self
    }

    fn merge_into(self, into: &mut Self, _dh: &DisplayHandle) {
        *into = self;
    }
}

/// Whether a colour-management object exists for the surface (one at a time).
#[derive(Debug, Default)]
pub(super) struct Attached(AtomicBool);

impl Attached {
    /// Marks the surface as having an object; false when it already had one.
    pub(super) fn take(surface: &WlSurface) -> bool {
        with_states(surface, |states| {
            let marker = states.data_map.get_or_insert_threadsafe(Attached::default);
            !marker.0.swap(true, Ordering::SeqCst)
        })
    }

    fn release(surface: &WlSurface) {
        with_states(surface, |states| {
            if let Some(marker) = states.data_map.get::<Attached>() {
                marker.0.store(false, Ordering::SeqCst);
            }
        });
    }
}

/// The tag `surface` committed (none: untagged, so sRGB).
///
/// ```no_run
/// use casement_wayland::color_management::surface_tag;
///
/// # fn demo(surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
/// assert_eq!(surface_tag(surface), None);
/// # }
/// ```
pub fn surface_tag(surface: &WlSurface) -> Option<ColorTag> {
    with_states(surface, |states| {
        states.cached_state.get::<SurfaceColor>().current().tag
    })
}

/// A colour-management surface's data: the surface it is for.
#[derive(Debug)]
pub struct SurfaceData {
    surface: WlSurface,
}

impl SurfaceData {
    pub(super) fn new(surface: WlSurface) -> SurfaceData {
        SurfaceData { surface }
    }
}

/// A feedback object's data: the surface it is for.
#[derive(Debug)]
pub struct FeedbackData {
    surface: WlSurface,
}

impl FeedbackData {
    pub(super) fn new(surface: WlSurface) -> FeedbackData {
        FeedbackData { surface }
    }
}

fn set_pending(surface: &WlSurface, tag: Option<ColorTag>) {
    with_states(surface, |states| {
        states.cached_state.get::<SurfaceColor>().pending().tag = tag;
    });
}

impl<D> Dispatch2<WpColorManagementSurfaceV1, D> for SurfaceData
where
    D: ColorHandler,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        resource: &WpColorManagementSurfaceV1,
        request: wp_color_management_surface_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        use wp_color_management_surface_v1::{Error, Request};
        let alive = self.surface.is_alive();
        match request {
            Request::Destroy => {}
            _ if !alive => resource.post_error(Error::Inert, "the surface is gone"),
            Request::SetImageDescription {
                image_description,
                render_intent,
            } => {
                if render_intent != WEnum::Value(RenderIntent::Perceptual) {
                    resource
                        .post_error(Error::RenderIntent, "only the perceptual intent is served");
                    return;
                }
                match ready_tag(&image_description) {
                    Some(tag) => set_pending(&self.surface, Some(tag)),
                    None => resource.post_error(
                        Error::ImageDescription,
                        "the image description is not ready",
                    ),
                }
            }
            Request::UnsetImageDescription => set_pending(&self.surface, None),
            _ => {}
        }
    }

    fn destroyed(&self, _state: &mut D, _client: ClientId, _resource: &WpColorManagementSurfaceV1) {
        if self.surface.is_alive() {
            set_pending(&self.surface, None);
            Attached::release(&self.surface);
        }
    }
}

/// The tag of a ready description; none for one that failed.
fn ready_tag(description: &WpImageDescriptionV1) -> Option<ColorTag> {
    description
        .data::<DescriptionData>()
        .and_then(DescriptionData::tag)
}

impl<D> Dispatch2<WpColorManagementSurfaceFeedbackV1, D> for FeedbackData
where
    D: ColorHandler + Dispatch<WpImageDescriptionV1, DescriptionData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &WpColorManagementSurfaceFeedbackV1,
        request: wp_color_management_surface_feedback_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        use wp_color_management_surface_feedback_v1::{Error, Request};
        match request {
            Request::GetPreferred { image_description }
            | Request::GetPreferredParametric { image_description } => {
                describe(
                    state,
                    data_init,
                    image_description,
                    ColorTag::SRGB,
                    InfoAllowed::Yes,
                );
                if !self.surface.is_alive() {
                    resource.post_error(Error::Inert, "the surface is gone");
                }
            }
            _ => {}
        }
    }
}
