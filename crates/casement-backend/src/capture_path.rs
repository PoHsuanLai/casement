//! Which way a capture frame reaches the client's buffer (wave-2026-10-15/dmabuf): drawn by
//! the GPU straight into a dma-buf, or read back and copied. One pure function, so the choice
//! is a table and not a set of `if`s at each of the places that answer a frame.
//!
//! ```
//! use casement_backend::capture_path::direct;
//!
//! // A dma-buf on the drawing GPU, whose renderer binds the format: drawn straight into it.
//! assert!(direct(Some(0xE280), Some(0xE280), true));
//! // The renderer does not bind the format: read back.
//! assert!(!direct(Some(0xE280), Some(0xE280), false));
//! ```

/// Whether a capture is drawn straight into a dma-buf (`true`) or read back and copied (`false`).
///
/// Direct needs a renderer that binds the buffer's format and modifier (`binds_format`: a GL
/// renderer that lists the pair; a CPU renderer or none never does) and a buffer on the device
/// that draws: `wanted` is the device clients are told to allocate on, `drawing` the one the
/// renderer draws on. A buffer is on another device only when both are known and differ;
/// casement has no renderer that draws across two GPUs for a capture, so such a buffer is
/// filled from a read-back (FINDINGS `wave-2026-10-15/dmabuf`). A failed direct draw always
/// falls back to the read-back, which every buffer has.
pub fn direct(wanted: Option<u64>, drawing: Option<u64>, binds_format: bool) -> bool {
    let foreign = matches!((wanted, drawing), (Some(wanted), Some(drawing)) if wanted != drawing);
    binds_format && !foreign
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_follows_the_buffer_and_the_renderer() {
        // name, wanted device, drawing device, renderer binds the format, direct
        let table = [
            ("dma-buf the gpu lists", Some(1), Some(1), true, true),
            ("devices unknown", None, None, true, true),
            ("renderer device unknown", Some(1), None, true, true),
            ("main device unknown", None, Some(1), true, true),
            ("dma-buf the gpu refuses", Some(1), Some(1), false, false),
            ("software renderer", Some(1), Some(1), false, false),
            ("dma-buf on another gpu", Some(1), Some(2), true, false),
        ];
        for (name, wanted, drawing, binds, expected) in table {
            assert_eq!(direct(wanted, drawing, binds), expected, "{name}");
        }
    }
}
