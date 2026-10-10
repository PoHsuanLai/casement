//! `shot --output NAME OUT.png`: one frame of an output over `ext-image-copy-capture-v1`, on
//! `desktop_client`'s wire client, written as a PNG. The compositor's frame is the one scanout
//! shows, so this is the same picture the screen has.

use desktop_client::capture::{CaptureFailure, CaptureRequest, FrameError};
use desktop_client::client::{DesktopClient, WireError};
use desktop_client::{Desktop, DesktopCommand, OutputName, Ticket};
use std::path::Path;
use wayland_client::{ConnectError, Connection};

/// The exit code of a shot that could not be taken.
const FAILED: u8 = 1;

/// Why a shot was not taken.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ShotError {
    /// There is no compositor to connect to (`WAYLAND_DISPLAY`).
    #[error("connecting to the compositor: {0}")]
    Connect(#[from] ConnectError),
    /// The wire client failed or the compositor refused the request.
    #[error("capture: {0}")]
    Wire(#[from] WireError),
    /// The compositor failed the frame.
    #[error("the compositor failed the capture: {0}")]
    Failed(#[from] CaptureFailure),
    /// The frame is in a layout this tool cannot read.
    #[error("reading the frame: {0}")]
    Frame(#[from] FrameError),
    /// The compositor answered with something that is not a capture ticket.
    #[error("the compositor started no capture")]
    NoTicket,
    /// The PNG could not be encoded or written.
    #[error("writing the PNG: {0}")]
    Png(#[from] image::ImageError),
    /// The frame's pixels do not fill its size.
    #[error("the frame is not {0} by {1} pixels")]
    Size(u32, u32),
}

/// Takes the shot and says so on stdout; the exit code.
pub fn run(output: &str, out: &Path) -> u8 {
    match shoot(output, out) {
        Ok((w, h)) => {
            println!("{{\"output\":\"{output}\",\"width\":{w},\"height\":{h}}}");
            0
        }
        Err(error) => {
            eprintln!("casementctl: shot: {error}");
            FAILED
        }
    }
}

/// One frame of `output` into the PNG at `out`: its width and height.
fn shoot(output: &str, out: &Path) -> Result<(u32, u32), ShotError> {
    let connection = Connection::connect_to_env()?;
    let mut client = DesktopClient::new(&connection)?;
    let request = CaptureRequest::new(OutputName::new(output));
    let Ticket::Capture(id) = client.send(DesktopCommand::CaptureOutput(request))? else {
        return Err(ShotError::NoTicket);
    };
    let frame = loop {
        client.blocking_dispatch()?;
        if let Some(done) = client.take_capture(id) {
            break done?;
        }
    };
    let rgba = frame.to_rgba8()?;
    let (w, h) = (frame.size.w, frame.size.h);
    let image = image::RgbaImage::from_raw(w, h, rgba).ok_or(ShotError::Size(w, h))?;
    image.save_with_format(out, image::ImageFormat::Png)?;
    Ok((w, h))
}
