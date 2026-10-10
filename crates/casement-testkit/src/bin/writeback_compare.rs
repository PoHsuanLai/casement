//! `writeback-compare`: the last step of `dev/vkms/accept.sh`'s writeback check.
//!
//! ```text
//! writeback-compare RAW W H PITCH FORMAT GOLDEN_SCENE OUT_DIR [CHANNEL [PPM]]
//! ```
//!
//! Compares the raw writeback buffer in `RAW` with the golden `GOLDEN_SCENE.png` (path without
//! the extension) within `CHANNEL` steps per channel (default 3) on all but `PPM` pixels per
//! million (default 2000). Exit 0 on a match, 1 on a difference (the frame and a diff image are
//! in `OUT_DIR`), 64 on bad arguments. It reads two files and writes into `OUT_DIR` only.

use casement_render::golden::GoldenOutcome;
use casement_testkit::writeback::{PixelFormat, RawFrame, compare_raw, tolerance};
use std::path::PathBuf;
use std::process::ExitCode;

fn number<T: std::str::FromStr>(text: Option<&String>, what: &str) -> Result<T, String> {
    text.ok_or_else(|| format!("missing {what}"))?
        .parse()
        .map_err(|_| format!("{what} is not a number"))
}

fn run(args: &[String]) -> Result<ExitCode, String> {
    let raw = args.first().ok_or("missing RAW")?;
    let width = number(args.get(1), "W")?;
    let height = number(args.get(2), "H")?;
    let pitch = number(args.get(3), "PITCH")?;
    let format = args
        .get(4)
        .and_then(|name| PixelFormat::named(name))
        .ok_or("FORMAT is XR24 or AR24")?;
    let scene = args.get(5).ok_or("missing GOLDEN_SCENE")?;
    let out_dir = PathBuf::from(args.get(6).ok_or("missing OUT_DIR")?);
    let channel = args
        .get(7)
        .map_or(Ok(3), |_| number(args.get(7), "CHANNEL"))?;
    let ppm = args
        .get(8)
        .map_or(Ok(2000), |_| number(args.get(8), "PPM"))?;
    let bytes = std::fs::read(raw).map_err(|error| format!("{raw}: {error}"))?;
    let frame = RawFrame::new(width, height, pitch, format);
    match compare_raw(&frame, &bytes, scene, tolerance(channel, ppm), &out_dir) {
        Ok(GoldenOutcome::Match) => {
            println!("writeback-compare: match");
            Ok(ExitCode::SUCCESS)
        }
        Ok(outcome) => {
            println!("writeback-compare: differs: {outcome:?}");
            Ok(ExitCode::from(1))
        }
        Err(error) => Err(format!("{raw}: {error}")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("writeback-compare: {error}");
            ExitCode::from(64)
        }
    }
}
