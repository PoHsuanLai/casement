//! The shot grammar: what `casementctl shot` asks Debug1 to capture while ext-image-copy-capture
//! (P1-C1) is not built. The compositor writes the output's last presented frame (or a region of
//! it) as a PNG to a path the caller names, inside the jail.
//!
//! ```text
//! output | output:NAME | region:X,Y,W,H      (X, Y, W, H in global logical pixels)
//! ```

use crate::error::IpcError;
use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// What a shot covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotArea {
    /// A whole output: the named one, or the first.
    Output(Option<OutputName>),
    /// A rectangle of the global logical space, cut from the output that holds its origin.
    Region(LogicalRect),
}

/// The area `text` says.
pub fn parse_shot(text: &str) -> Result<ShotArea, IpcError> {
    let bad = || IpcError::BadLine(text.to_owned());
    match text.split_once(':') {
        None if text == "output" => Ok(ShotArea::Output(None)),
        Some(("output", name)) => Ok(ShotArea::Output(Some(
            OutputName::parse(name).map_err(|_| bad())?,
        ))),
        Some(("region", numbers)) => {
            let parts: Vec<&str> = numbers.split(',').collect();
            let [x, y, w, h] = parts.as_slice() else {
                return Err(bad());
            };
            match (x.parse(), y.parse(), w.parse::<u32>(), h.parse::<u32>()) {
                (Ok(x), Ok(y), Ok(w @ 1..), Ok(h @ 1..)) => Ok(ShotArea::Region(LogicalRect::new(
                    LogicalPoint { x, y },
                    LogicalSize { w, h },
                ))),
                _ => Err(bad()),
            }
        }
        _ => Err(bad()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grammar_table() {
        let rect = |x, y, w, h| {
            ShotArea::Region(LogicalRect::new(
                LogicalPoint { x, y },
                LogicalSize { w, h },
            ))
        };
        let named = OutputName::parse("HEADLESS-2").expect("a name");
        let cases = [
            ("output", Some(ShotArea::Output(None))),
            ("output:HEADLESS-2", Some(ShotArea::Output(Some(named)))),
            ("region:0,0,640,480", Some(rect(0, 0, 640, 480))),
            ("region:-10,20,5,5", Some(rect(-10, 20, 5, 5))),
            ("region:0,0,0,5", None),
            ("region:0,0,5", None),
            ("region:a,0,5,5", None),
            ("window:3", None),
            ("", None),
        ];
        for (text, want) in cases {
            assert_eq!(parse_shot(text).ok(), want, "{text}");
        }
    }
}
