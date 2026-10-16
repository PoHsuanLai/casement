//! A picture file or a buffer as premultiplied pixels at an exact edge: PNG and JPEG through
//! `image`, SVG through `resvg` drawn at the edge itself (so it is sharp at any scale).

use crate::{IconError, Pixels, RgbaBuffer};
use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;
use std::path::Path;
use std::sync::Arc;

/// The largest icon file read, in bytes.
const FILE_LIMIT: u64 = 16 * 1024 * 1024;

/// The largest edge drawn, in pixels.
const EDGE_LIMIT: u32 = 4096;

/// The picture at `path` (SVG by its extension, else PNG or JPEG by its bytes) as `edge` by
/// `edge` premultiplied pixels, kept in proportion and centred.
pub fn load(path: &Path, edge: u32) -> Result<Pixels, IconError> {
    checked(edge)?;
    let bytes = crate::read::plain_file(path, FILE_LIMIT)?;
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("svg") => svg(&bytes, edge),
        _ => {
            let image = image::load_from_memory(&bytes)
                .map_err(|error| IconError::Decode(error.to_string()))?;
            fitted(image, edge)
        }
    }
}

/// `buffer` (a toplevel's own pixels) as `edge` by `edge` premultiplied pixels.
pub fn scale_straight(buffer: &RgbaBuffer, edge: u32) -> Result<Pixels, IconError> {
    checked(edge)?;
    let image = RgbaImage::from_raw(buffer.edge, buffer.edge, buffer.rgba.clone())
        .ok_or_else(|| IconError::Decode("the buffer is not square RGBA".to_owned()))?;
    fitted(DynamicImage::ImageRgba8(image), edge)
}

fn checked(edge: u32) -> Result<(), IconError> {
    match edge {
        1..=EDGE_LIMIT => Ok(()),
        _ => Err(IconError::Edge(edge)),
    }
}

/// `image` shrunk or grown to fit `edge` by `edge`, centred on transparent pixels.
fn fitted(image: DynamicImage, edge: u32) -> Result<Pixels, IconError> {
    let scaled = match (image.width(), image.height()) {
        (w, h) if w == edge && h == edge => image.into_rgba8(),
        _ => image.resize(edge, edge, FilterType::Lanczos3).into_rgba8(),
    };
    let side = usize::try_from(edge).unwrap_or(0);
    let (w, h) = (
        usize::try_from(scaled.width()).unwrap_or(0),
        usize::try_from(scaled.height()).unwrap_or(0),
    );
    let (left, top) = ((side - w) / 2, (side - h) / 2);
    let mut straight = vec![0u8; side * side * 4];
    scaled
        .as_raw()
        .chunks_exact(w * 4)
        .enumerate()
        .for_each(|(row, line)| {
            let at = ((top + row) * side + left) * 4;
            straight[at..at + w * 4].copy_from_slice(line);
        });
    Pixels::from_straight(edge, &straight).ok_or(IconError::Edge(edge))
}

/// SVG options that load nothing from outside the SVG: an `<image href>` is a file path to
/// usvg's default resolver, which would read it whole, device or pipe included. Data URLs inside
/// the picture still work.
fn no_outside_files() -> usvg::Options<'static> {
    usvg::Options {
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_string: Box::new(|_, _| None),
            ..usvg::ImageHrefResolver::default()
        },
        ..usvg::Options::default()
    }
}

/// An SVG drawn to fill `edge` by `edge`.
fn svg(bytes: &[u8], edge: u32) -> Result<Pixels, IconError> {
    let tree = usvg::Tree::from_data(bytes, &no_outside_files())
        .map_err(|error| IconError::Decode(error.to_string()))?;
    let (w, h) = (tree.size().width(), tree.size().height());
    let side = edge as f32;
    let scale = side / w.max(h);
    let place = Transform::from_row(
        scale,
        0.0,
        0.0,
        scale,
        (side - w * scale) / 2.0,
        (side - h * scale) / 2.0,
    );
    let mut pixmap = Pixmap::new(edge, edge).ok_or(IconError::Edge(edge))?;
    resvg::render(&tree, place, &mut pixmap.as_mut());
    Ok(Pixels {
        edge,
        rgba: Arc::from(pixmap.take()),
    })
}
