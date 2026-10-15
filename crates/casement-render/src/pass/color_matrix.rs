//! 3x3 colour matrices: the linear-light conversion from one set of primaries to another. Floats
//! are allowed here (a render pass); the primaries arrive as integers from `casement-core`.

use casement_core::output::{Chromaticity, D65, Gamut, Primaries, SRGB};

/// A row-major 3x3 matrix over linear RGB (or XYZ).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorMatrix(pub [[f64; 3]; 3]);

impl ColorMatrix {
    /// Leaves colours as they are.
    pub const IDENTITY: ColorMatrix =
        ColorMatrix([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    /// `self` applied after `first`.
    pub fn after(self, first: ColorMatrix) -> ColorMatrix {
        let (a, b) = (self.0, first.0);
        ColorMatrix(std::array::from_fn(|row| {
            std::array::from_fn(|col| (0..3).map(|k| a[row][k] * b[k][col]).sum())
        }))
    }

    /// The matrix applied to a colour.
    pub fn apply(self, rgb: [f64; 3]) -> [f64; 3] {
        self.0
            .map(|row| row.iter().zip(rgb).map(|(m, c)| m * c).sum())
    }

    /// The inverse; none for a matrix with no inverse (primaries on one line).
    pub fn inverse(self) -> Option<ColorMatrix> {
        let m = self.0;
        let cofactor = |r: usize, c: usize| -> f64 {
            let (r1, r2) = ((r + 1) % 3, (r + 2) % 3);
            let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
            m[r1][c1] * m[r2][c2] - m[r1][c2] * m[r2][c1]
        };
        let det: f64 = (0..3).map(|c| m[0][c] * cofactor(0, c)).sum();
        (det.abs() > 1e-12).then(|| {
            ColorMatrix(std::array::from_fn(|row| {
                std::array::from_fn(|col| cofactor(col, row) / det)
            }))
        })
    }

    /// Column-major `f32`s, as a GLSL `mat3` uniform takes them.
    pub fn column_major(self) -> [f32; 9] {
        std::array::from_fn(|i| self.0[i % 3][i / 3] as f32)
    }
}

/// XYZ of a chromaticity at luminance 1.
fn xyz(point: Chromaticity) -> [f64; 3] {
    let (x, y) = (f64::from(point.x) / 1e4, f64::from(point.y) / 1e4);
    [x / y, 1.0, (1.0 - x - y) / y]
}

/// Linear RGB of `primaries` (white D65) to XYZ.
pub fn rgb_to_xyz(primaries: Primaries) -> Option<ColorMatrix> {
    let columns = [primaries.red, primaries.green, primaries.blue].map(xyz);
    let raw = ColorMatrix(std::array::from_fn(|row| {
        std::array::from_fn(|col| columns[col][row])
    }));
    let scale = raw.inverse()?.apply(xyz(D65));
    Some(ColorMatrix(std::array::from_fn(|row| {
        std::array::from_fn(|col| raw.0[row][col] * scale[col])
    })))
}

/// Linear sRGB to linear RGB of the panel: the identity for an sRGB panel, so it costs nothing.
///
/// ```
/// use casement_core::output::{DISPLAY_P3, Gamut};
/// use casement_render::pass::{srgb_to_panel, ColorMatrix};
///
/// assert_eq!(srgb_to_panel(Gamut::Srgb), ColorMatrix::IDENTITY);
/// let to_p3 = srgb_to_panel(Gamut::Wide(DISPLAY_P3));
/// assert!((to_p3.0[0][0] - 0.8225).abs() < 2e-3);
/// ```
pub fn srgb_to_panel(gamut: Gamut) -> ColorMatrix {
    match gamut {
        Gamut::Srgb => ColorMatrix::IDENTITY,
        Gamut::Wide(panel) => rgb_to_xyz(panel)
            .and_then(ColorMatrix::inverse)
            .zip(rgb_to_xyz(SRGB))
            .map(|(from_xyz, srgb)| from_xyz.after(srgb))
            .unwrap_or(ColorMatrix::IDENTITY),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::output::DISPLAY_P3;

    fn close(a: [[f64; 3]; 3], b: [[f64; 3]; 3], tolerance: f64) {
        for (row, (ra, rb)) in a.iter().zip(&b).enumerate() {
            for (col, (x, y)) in ra.iter().zip(rb).enumerate() {
                assert!((x - y).abs() < tolerance, "[{row}][{col}] {x} vs {y}");
            }
        }
    }

    #[test]
    fn srgb_to_display_p3_matches_the_published_matrix() {
        let want = [
            [0.8225, 0.1774, 0.0000],
            [0.0332, 0.9669, 0.0000],
            [0.0171, 0.0724, 0.9108],
        ];
        close(srgb_to_panel(Gamut::Wide(DISPLAY_P3)).0, want, 2e-3);
    }

    #[test]
    fn white_stays_white() {
        let white = srgb_to_panel(Gamut::Wide(DISPLAY_P3)).apply([1.0; 3]);
        assert!(white.iter().all(|c| (c - 1.0).abs() < 2e-3), "{white:?}");
    }

    #[test]
    fn the_inverse_undoes_the_matrix() {
        let to = srgb_to_panel(Gamut::Wide(DISPLAY_P3));
        let back = to.inverse().expect("inverse").after(to);
        close(back.0, ColorMatrix::IDENTITY.0, 1e-9);
    }

    #[test]
    fn primaries_on_one_line_have_no_matrix() {
        let flat = Chromaticity { x: 3000, y: 3000 };
        let primaries = Primaries {
            red: flat,
            green: flat,
            blue: flat,
        };
        assert_eq!(rgb_to_xyz(primaries), None);
        assert_eq!(srgb_to_panel(Gamut::Wide(primaries)), ColorMatrix::IDENTITY);
    }

    #[test]
    fn column_major_puts_columns_first() {
        let m = ColorMatrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
        assert_eq!(
            m.column_major(),
            [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]
        );
    }
}
