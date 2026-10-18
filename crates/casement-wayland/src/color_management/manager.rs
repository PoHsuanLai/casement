//! The manager global and the per-output object.

use super::creator::{CreatorData, DescriptionData, IccCreatorData, InfoAllowed, describe, refuse};
use super::surface::{Attached, FeedbackData, SurfaceData};
use super::{ColorHandler, named};
use crate::states::GlobalFilter;
use casement_core::OutputKey;
use casement_core::output::TransferCurve;
use casement_core::policy::GlobalKind;
use smithay::reexports::wayland_protocols::wp::color_management::v1::server::{
    wp_color_management_output_v1::{self, WpColorManagementOutputV1},
    wp_color_management_surface_feedback_v1::WpColorManagementSurfaceFeedbackV1,
    wp_color_management_surface_v1::WpColorManagementSurfaceV1,
    wp_color_manager_v1::{self, Feature, Primaries, RenderIntent, WpColorManagerV1},
    wp_image_description_creator_icc_v1::WpImageDescriptionCreatorIccV1,
    wp_image_description_creator_params_v1::WpImageDescriptionCreatorParamsV1,
    wp_image_description_v1::{Cause, WpImageDescriptionV1},
};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, New, Resource,
};
use smithay::wayland::{Dispatch2, GlobalDispatch2};

/// The global's data: the filter its `can_view` asks.
#[derive(Debug, Clone, Copy)]
pub struct ManagerGlobal {
    filter: GlobalFilter,
}

impl ManagerGlobal {
    pub(super) fn new(filter: GlobalFilter) -> ManagerGlobal {
        ManagerGlobal { filter }
    }
}

/// A bound manager's data.
#[derive(Debug, Clone, Copy, Default)]
pub struct ManagerData;

/// An output object's data: the output it describes, none for one that is gone.
#[derive(Debug, Clone)]
pub struct OutputData {
    key: Option<OutputKey>,
}

/// Everything the manager serves, sent once at bind (`done` last).
fn announce(manager: &WpColorManagerV1) {
    manager.supported_intent(RenderIntent::Perceptual);
    manager.supported_feature(Feature::Parametric);
    manager.supported_feature(Feature::SetPrimaries);
    for curve in [
        TransferCurve::Srgb,
        TransferCurve::Gamma22,
        TransferCurve::Linear,
    ] {
        manager.supported_tf_named(named::name_of_curve(curve, manager.version()));
    }
    manager.supported_primaries_named(Primaries::Srgb);
    manager.supported_primaries_named(Primaries::DisplayP3);
    manager.done();
}

impl<D> GlobalDispatch2<WpColorManagerV1, D> for ManagerGlobal
where
    D: Dispatch<WpColorManagerV1, ManagerData> + 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<WpColorManagerV1>,
        data_init: &mut DataInit<'_, D>,
    ) {
        let manager = data_init.init(resource, ManagerData);
        announce(&manager);
    }

    fn can_view(&self, client: &Client) -> bool {
        self.filter.sees(GlobalKind::ColorManagement, client)
    }
}

impl<D> Dispatch2<WpColorManagerV1, D> for ManagerData
where
    D: ColorHandler
        + Dispatch<WpColorManagementOutputV1, OutputData>
        + Dispatch<WpColorManagementSurfaceV1, SurfaceData>
        + Dispatch<WpColorManagementSurfaceFeedbackV1, FeedbackData>
        + Dispatch<WpImageDescriptionCreatorIccV1, IccCreatorData>
        + Dispatch<WpImageDescriptionCreatorParamsV1, CreatorData>
        + Dispatch<WpImageDescriptionV1, DescriptionData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &WpColorManagerV1,
        request: wp_color_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        use wp_color_manager_v1::{Error, Request};
        match request {
            Request::GetOutput { id, output } => {
                let key = state.output_key_of(&output);
                data_init.init(id, OutputData { key });
            }
            Request::GetSurface { id, surface } => {
                let fresh = Attached::take(&surface);
                data_init.init(id, SurfaceData::new(surface));
                if !fresh {
                    resource.post_error(
                        Error::SurfaceExists,
                        "the surface already has a colour management object",
                    );
                }
            }
            Request::GetSurfaceFeedback { id, surface } => {
                data_init.init(id, FeedbackData::new(surface));
            }
            Request::CreateIccCreator { obj } => {
                data_init.init(obj, IccCreatorData);
                resource.post_error(Error::UnsupportedFeature, "ICC profiles are not served");
            }
            Request::CreateParametricCreator { obj } => {
                data_init.init(obj, CreatorData::default());
            }
            Request::CreateWindowsScrgb { image_description } => {
                refuse(
                    data_init,
                    image_description,
                    Cause::Unsupported,
                    "scRGB is not served",
                );
                resource.post_error(Error::UnsupportedFeature, "scRGB is not served");
            }
            Request::GetImageDescription {
                image_description, ..
            } => refuse(
                data_init,
                image_description,
                Cause::Unsupported,
                "references are not served",
            ),
            _ => {}
        }
    }
}

impl<D> Dispatch2<WpColorManagementOutputV1, D> for OutputData
where
    D: ColorHandler + Dispatch<WpImageDescriptionV1, DescriptionData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        _resource: &WpColorManagementOutputV1,
        request: wp_color_management_output_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let wp_color_management_output_v1::Request::GetImageDescription { image_description } =
            request
        else {
            return;
        };
        match &self.key {
            Some(key) => {
                let tag = state.color_part().output_tag(key);
                describe(state, data_init, image_description, tag, InfoAllowed::Yes);
            }
            None => refuse(
                data_init,
                image_description,
                Cause::NoOutput,
                "the output is gone",
            ),
        }
    }
}
