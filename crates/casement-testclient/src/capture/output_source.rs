//! `ext_output_image_capture_source_manager_v1` as a client: the `wl_output`s a source can be
//! made from (named by their connector) and the source objects themselves. Filled by P1-C1.

use super::copy_session::Capturing;
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_image_capture_source_v1::{self, ExtImageCaptureSourceV1},
    ext_output_image_capture_source_manager_v1::{self, ExtOutputImageCaptureSourceManagerV1},
};

/// A bound `wl_output` and the connector name it announced (version 4).
#[derive(Debug)]
pub(super) struct NamedOutput {
    pub(super) global: u32,
    pub(super) output: WlOutput,
    pub(super) name: Option<String>,
}

impl Dispatch<WlOutput, u32> for Capturing {
    fn event(
        state: &mut Self,
        _: &WlOutput,
        event: wl_output::Event,
        global: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event
            && let Some(entry) = state.outputs.iter_mut().find(|o| o.global == *global)
        {
            entry.name = Some(name);
        }
    }
}

impl Dispatch<ExtOutputImageCaptureSourceManagerV1, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &ExtOutputImageCaptureSourceManagerV1,
        _: ext_output_image_capture_source_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtImageCaptureSourceV1, ()> for Capturing {
    fn event(
        _: &mut Self,
        _: &ExtImageCaptureSourceV1,
        _: ext_image_capture_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
