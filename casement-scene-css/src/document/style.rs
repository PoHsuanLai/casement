//! The custom properties a box carries: the inline `style` that hands the spec and the knobs
//! to `scene.css`.

use casement_core::geometry::{LogicalRect, LogicalSize};
use casement_core::window::SizeLimits;

/// A list of custom properties, written as one `style` attribute.
#[derive(Debug, Default)]
pub(super) struct Props(String);

impl Props {
    /// No properties yet.
    pub(super) fn new() -> Props {
        Props::default()
    }

    /// `--name` as `value` logical pixels.
    pub(super) fn px(mut self, name: &str, value: impl Into<i64>) -> Props {
        self.0.push_str(&format!("--{name}:{}px;", value.into()));
        self
    }

    /// `--name` as the plain number `value`.
    pub(super) fn num(mut self, name: &str, value: impl Into<i64>) -> Props {
        self.0.push_str(&format!("--{name}:{};", value.into()));
        self
    }

    /// `rect` as `--{prefix}x`, `--{prefix}y`, `--{prefix}w` and `--{prefix}h`.
    pub(super) fn rect(self, prefix: &str, rect: LogicalRect) -> Props {
        self.px(&format!("{prefix}x"), rect.origin.x)
            .px(&format!("{prefix}y"), rect.origin.y)
            .px(&format!("{prefix}w"), rect.size.w)
            .px(&format!("{prefix}h"), rect.size.h)
    }

    /// `size` as `--w` and `--h`.
    pub(super) fn size(self, size: LogicalSize) -> Props {
        self.px("w", size.w).px("h", size.h)
    }

    /// `limits` as `--min-w`, `--min-h` and, when bounded, `--max-w` and `--max-h`.
    pub(super) fn limits(self, limits: SizeLimits) -> Props {
        let with_min = self.px("min-w", limits.min.w).px("min-h", limits.min.h);
        match limits.max {
            None => with_min,
            Some(max) => with_min.px("max-w", max.w).px("max-h", max.h),
        }
    }

    /// The `style` attribute's text.
    pub(super) fn text(self) -> String {
        self.0
    }
}
