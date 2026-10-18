//! The parametric creator (`wp_image_description_creator_params_v1`), image descriptions
//! (`wp_image_description_v1`) and their information (`wp_image_description_info_v1`).

use super::{ColorHandler, named};
use casement_core::output::{ColorTag, Primaries, TransferCurve};
use smithay::reexports::wayland_protocols::wp::color_management::v1::server::{
    wp_image_description_creator_icc_v1::{self, WpImageDescriptionCreatorIccV1},
    wp_image_description_creator_params_v1::{self, WpImageDescriptionCreatorParamsV1},
    wp_image_description_info_v1::WpImageDescriptionInfoV1,
    wp_image_description_v1::{self, Cause, WpImageDescriptionV1},
};
use smithay::reexports::wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, New, Resource,
};
use smithay::wayland::Dispatch2;
use std::sync::{Mutex, MutexGuard};

/// The luminance numbers every description reports: the sRGB defaults (0.2 / 80 / 80 cd/m2).
const MIN_LUMINANCE: u32 = 2000;
const MAX_LUMINANCE: u32 = 80;

/// Whether `get_information` is allowed on a description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InfoAllowed {
    /// The description came from an output or a surface's feedback.
    Yes,
    /// A client made it itself.
    No,
}

/// An image description's data: its tag, none when it failed.
#[derive(Debug, Clone, Copy)]
pub struct DescriptionData {
    tag: Option<ColorTag>,
    info: InfoAllowed,
}

impl DescriptionData {
    /// The tag of a ready description.
    pub fn tag(&self) -> Option<ColorTag> {
        self.tag
    }
}

/// Makes `id` a ready description of `tag` and says so.
pub(super) fn describe<D>(
    state: &mut D,
    data_init: &mut DataInit<'_, D>,
    id: New<WpImageDescriptionV1>,
    tag: ColorTag,
    info: InfoAllowed,
) where
    D: ColorHandler + Dispatch<WpImageDescriptionV1, DescriptionData>,
{
    let identity = state.color_part().identity_of(tag);
    let description = data_init.init(
        id,
        DescriptionData {
            tag: Some(tag),
            info,
        },
    );
    let low = u32::try_from(identity & 0xffff_ffff).unwrap_or(0);
    match description.version() {
        1 => description.ready(low),
        _ => description.ready2(u32::try_from(identity >> 32).unwrap_or(0), low),
    }
}

/// Makes `id` a description that failed for `cause`.
pub(super) fn refuse<D>(
    data_init: &mut DataInit<'_, D>,
    id: New<WpImageDescriptionV1>,
    cause: Cause,
    message: &str,
) where
    D: Dispatch<WpImageDescriptionV1, DescriptionData>,
{
    let description = data_init.init(
        id,
        DescriptionData {
            tag: None,
            info: InfoAllowed::No,
        },
    );
    description.failed(cause, message.to_owned());
}

/// The primaries a parametric creator has been given.
#[derive(Debug, Default, Clone, Copy)]
enum PrimariesGiven {
    /// None yet.
    #[default]
    Unset,
    /// Coordinates that cannot make a colour space (out of range, or on one line).
    Unusable,
    /// These.
    Set(Primaries),
}

/// What a parametric creator has been given so far.
#[derive(Debug, Default, Clone, Copy)]
struct Given {
    primaries: PrimariesGiven,
    transfer: Option<TransferCurve>,
}

/// A parametric creator's data.
#[derive(Debug, Default)]
pub struct CreatorData {
    given: Mutex<Given>,
}

impl CreatorData {
    fn given(&self) -> MutexGuard<'_, Given> {
        self.given
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The ICC creator is not served: the manager refuses it, and the object only absorbs requests.
#[derive(Debug, Default, Clone, Copy)]
pub struct IccCreatorData;

impl<D> Dispatch2<WpImageDescriptionCreatorIccV1, D> for IccCreatorData {
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &WpImageDescriptionCreatorIccV1,
        _request: wp_image_description_creator_icc_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
    }
}

impl<D> Dispatch2<WpImageDescriptionCreatorParamsV1, D> for CreatorData
where
    D: ColorHandler + Dispatch<WpImageDescriptionV1, DescriptionData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &WpImageDescriptionCreatorParamsV1,
        request: wp_image_description_creator_params_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        use wp_image_description_creator_params_v1::{Error, Request};
        match request {
            Request::SetTfNamed { tf } => match named::curve_of_name(tf) {
                None => {
                    resource.post_error(Error::InvalidTf, "that transfer function is not served")
                }
                Some(curve) => {
                    if self.given().transfer.replace(curve).is_some() {
                        resource.post_error(Error::AlreadySet, "the transfer function is set");
                    }
                }
            },
            Request::SetPrimariesNamed { primaries } => match named::primaries_of_name(primaries) {
                None => resource.post_error(
                    Error::InvalidPrimariesNamed,
                    "those primaries are not served",
                ),
                Some(set) => set_primaries(self, resource, PrimariesGiven::Set(set)),
            },
            Request::SetPrimaries {
                r_x,
                r_y,
                g_x,
                g_y,
                b_x,
                b_y,
                w_x,
                w_y,
            } => {
                let coordinates = [r_x, r_y, g_x, g_y, b_x, b_y, w_x, w_y];
                // Coordinates that cannot make a colour space do not end the connection: `create`
                // answers with a failed description.
                let given = match named::primaries_of_coordinates(coordinates) {
                    Some(set) if set.is_usable() => PrimariesGiven::Set(set),
                    _ => PrimariesGiven::Unusable,
                };
                set_primaries(self, resource, given);
            }
            Request::SetTfPower { .. }
            | Request::SetLuminances { .. }
            | Request::SetMasteringDisplayPrimaries { .. }
            | Request::SetMasteringLuminance { .. } => {
                resource.post_error(Error::UnsupportedFeature, "this feature is not served")
            }
            Request::SetMaxCll { .. } | Request::SetMaxFall { .. } => {}
            Request::Create { image_description } => {
                let given = *self.given();
                match (given.primaries, given.transfer) {
                    (PrimariesGiven::Set(primaries), Some(transfer)) => describe(
                        state,
                        data_init,
                        image_description,
                        ColorTag {
                            primaries,
                            transfer,
                        },
                        InfoAllowed::No,
                    ),
                    (PrimariesGiven::Unusable, Some(_)) => refuse(
                        data_init,
                        image_description,
                        Cause::Unsupported,
                        "those primaries cannot make a colour space",
                    ),
                    _ => {
                        refuse(
                            data_init,
                            image_description,
                            Cause::Unsupported,
                            "incomplete",
                        );
                        resource.post_error(
                            Error::IncompleteSet,
                            "primaries and transfer function are both needed",
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn set_primaries(
    data: &CreatorData,
    resource: &WpImageDescriptionCreatorParamsV1,
    set: PrimariesGiven,
) {
    let before = std::mem::replace(&mut data.given().primaries, set);
    if !matches!(before, PrimariesGiven::Unset) {
        resource.post_error(
            wp_image_description_creator_params_v1::Error::AlreadySet,
            "the primaries are set",
        );
    }
}

impl<D> Dispatch2<WpImageDescriptionV1, D> for DescriptionData
where
    D: ColorHandler + Dispatch<WpImageDescriptionInfoV1, InfoData>,
{
    fn request(
        &self,
        state: &mut D,
        _client: &Client,
        resource: &WpImageDescriptionV1,
        request: wp_image_description_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        use wp_image_description_v1::{Error, Request};
        let Request::GetInformation { information } = request else {
            return;
        };
        let info = data_init.init(information, InfoData);
        match (self.tag, self.info) {
            (None, _) => resource.post_error(Error::NotReady, "the image description is not ready"),
            (Some(_), InfoAllowed::No) => resource.post_error(
                Error::NoInformation,
                "this image description does not allow get_information",
            ),
            (Some(tag), InfoAllowed::Yes) => state.color_part().unsent.push((info, tag)),
        }
    }
}

/// Sends the information asked for since the last call. `done` is a destructor event: sent from
/// inside the `get_information` that created the object, it would destroy the object before the
/// request returns, and the backend then writes the new object's data into freed memory. So the
/// handler queues the object and the loop calls this after every dispatch of the clients.
pub fn send_unsent(part: &mut super::ColorPart) {
    for (info, tag) in std::mem::take(&mut part.unsent) {
        if info.is_alive() {
            send_information(&info, tag);
        }
    }
}

/// Everything a parametric description reports, then `done`.
fn send_information(info: &WpImageDescriptionInfoV1, tag: ColorTag) {
    let [rx, ry, gx, gy, bx, by, wx, wy] = named::coordinates_of(tag.primaries);
    info.primaries(rx, ry, gx, gy, bx, by, wx, wy);
    info.tf_named(named::name_of_curve(tag.transfer, info.version()));
    info.luminances(MIN_LUMINANCE, MAX_LUMINANCE, MAX_LUMINANCE);
    info.target_primaries(rx, ry, gx, gy, bx, by, wx, wy);
    info.target_luminance(MIN_LUMINANCE, MAX_LUMINANCE);
    info.done();
}

/// An information object's data: it only sends.
#[derive(Debug, Default, Clone, Copy)]
pub struct InfoData;

impl<D> Dispatch2<WpImageDescriptionInfoV1, D> for InfoData {
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _resource: &WpImageDescriptionInfoV1,
        _request: <WpImageDescriptionInfoV1 as Resource>::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
    }
}
