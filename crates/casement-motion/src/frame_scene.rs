//! A sampled frame: `Eq` data in whole sub-pixels, what the renderer draws and what motion
//! goldens compare.

use casement_core::geometry::{LogicalRect, LogicalSize, SubPoint, SubPx, SubSize};
use casement_core::{OutputKey, WindowId};
use casement_protocols::vocab::Emphasis;
use ds_core::vocab::Fraction;

/// Every output's frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FrameScene {
    /// The outputs, by key order.
    pub outputs: Vec<OutputFrame>,
}

impl FrameScene {
    /// The frame as golden text: per output a line `output <key> strip <x>`, then per window
    /// `w<id> at <x> <y> size <w> <h> opacity <permille> committed <w>x<h> corner <r> shadow
    /// <level> emphasis <emphasis>`, sub-pixel integers throughout, one line each; a window in a
    /// minimise flight ends its line with ` flight <permille>`.
    pub fn text(&self) -> String {
        self.outputs.iter().map(OutputFrame::text).collect()
    }
}

impl OutputFrame {
    fn text(&self) -> String {
        let head = format!("output {} strip {}\n", self.output.as_str(), self.strip.0.0);
        self.windows.iter().fold(head, |text, w| text + &w.text())
    }
}

impl PlacedWindow {
    fn text(&self) -> String {
        let flight = self.flight.map_or(String::new(), |flight| {
            format!(" flight {}", flight.progress.0)
        });
        format!(
            "w{} at {} {} size {} {} opacity {} committed {}x{} corner {} shadow {} emphasis {}{}\n",
            self.window.0,
            self.at.x.0,
            self.at.y.0,
            self.size.w.0,
            self.size.h.0,
            self.opacity.0,
            self.committed.w,
            self.committed.h,
            self.corner.0.0,
            self.shadow.word(),
            emphasis_word(self.emphasis),
            flight,
        )
    }
}

impl ShadowLevel {
    fn word(self) -> &'static str {
        match self {
            ShadowLevel::None => "none",
            ShadowLevel::Inactive => "inactive",
            ShadowLevel::Key => "key",
        }
    }
}

/// The emphasis as its XML entry name.
fn emphasis_word(emphasis: Emphasis) -> &'static str {
    match emphasis {
        Emphasis::Key => "key",
        Emphasis::Main => "main",
        Emphasis::Inactive => "inactive",
    }
}

/// One output's frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFrame {
    /// The output.
    pub output: OutputKey,
    /// How far its Space strip is shifted.
    pub strip: StripOffset,
    /// Its windows, bottom of the stack first.
    pub windows: Vec<PlacedWindow>,
}

/// A Space strip's horizontal shift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StripOffset(pub SubPx);

/// One window in a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedWindow {
    /// The window.
    pub window: WindowId,
    /// Its committed buffer size (scaled over `size` until it commits the new one).
    pub committed: LogicalSize,
    /// Its top-left, in global sub-pixels.
    pub at: SubPoint,
    /// Its drawn size.
    pub size: SubSize,
    /// Its opacity.
    pub opacity: Fraction,
    /// Its corner radius.
    pub corner: CornerRadius,
    /// Which shadow it casts.
    pub shadow: ShadowLevel,
    /// Key, main or inactive.
    pub emphasis: Emphasis,
    /// Set while it flies into or out of a dock tile.
    pub flight: Option<Flight>,
}

/// A window flying into a dock tile (minimising) or out of one (coming back): the two ends and
/// how far along it is. The renderer draws the window shrunk, or as a genie, by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Flight {
    /// The window's end of the flight, in global logical pixels: where it left from, or where
    /// it comes back to.
    pub window: LogicalRect,
    /// The dock tile, in global logical pixels.
    pub tile: LogicalRect,
    /// How far the window is from its end to the tile's, in thousandths (1000 is in the tile).
    pub progress: Fraction,
}

impl Flight {
    /// A flight between `window` and `tile`, `progress` of the way to the tile.
    ///
    /// ```
    /// use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
    /// use casement_motion::Flight;
    /// use ds_core::vocab::Fraction;
    ///
    /// let rect = |x| LogicalRect::new(LogicalPoint { x, y: 0 }, LogicalSize { w: 10, h: 10 });
    /// assert_eq!(Flight::new(rect(0), rect(90), Fraction(500)).progress, Fraction(500));
    /// ```
    pub fn new(window: LogicalRect, tile: LogicalRect, progress: Fraction) -> Flight {
        Flight {
            window,
            tile,
            progress,
        }
    }
}

/// A corner radius in sub-pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CornerRadius(pub SubPx);

/// Which compositor shadow a window casts (values from ds-style's elevation tokens at render
/// time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadowLevel {
    /// None (the client draws its own frame and shadow).
    None,
    /// An inactive window's shadow.
    Inactive,
    /// The key window's deeper shadow.
    Key,
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::output::{EdidFacts, EdidSerial};

    #[test]
    fn a_frame_reads_as_one_line_per_output_and_window() {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        let output = OutputKey::of(&edid, &OutputName::parse("HEADLESS-1").expect("name"));
        let frame = FrameScene {
            outputs: vec![OutputFrame {
                output,
                strip: StripOffset(SubPx(-240)),
                windows: vec![PlacedWindow {
                    window: WindowId(3),
                    committed: LogicalSize { w: 400, h: 300 },
                    at: SubPoint {
                        x: SubPx(12_000),
                        y: SubPx(-6),
                    },
                    size: SubSize {
                        w: SubPx(48_000),
                        h: SubPx(36_000),
                    },
                    opacity: Fraction(350),
                    corner: CornerRadius(SubPx(1_200)),
                    shadow: ShadowLevel::Key,
                    emphasis: Emphasis::Main,
                    flight: None,
                }],
            }],
        };
        assert_eq!(
            frame.text(),
            "output connector:HEADLESS-1 strip -240\n\
             w3 at 12000 -6 size 48000 36000 opacity 350 committed 400x300 corner 1200 shadow key emphasis main\n"
        );
        assert_eq!(FrameScene::default().text(), "");
    }
}
