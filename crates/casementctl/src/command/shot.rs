//! `shot [--output NAME | --window ID | --region X,Y,W,H] [--masked] OUT.png`.

use super::plan::Call;
use super::{CommandError, Masked, ShotTarget};
use std::path::{Path, PathBuf};

fn bad(message: impl Into<String>) -> CommandError {
    CommandError::BadArgument(message.into())
}

/// The target, masking and output path `args` (after `shot`) name.
pub fn parse_shot(args: &[String]) -> Result<(ShotTarget, Masked, PathBuf), CommandError> {
    let mut target = None;
    let mut masked = Masked::No;
    let mut out = None;
    let mut rest = args.iter();
    while let Some(word) = rest.next() {
        let mut value = |flag: &str| {
            rest.next()
                .ok_or_else(|| bad(format!("{flag} needs a value")))
        };
        let named = match word.as_str() {
            "--masked" => {
                masked = Masked::Yes;
                continue;
            }
            "--output" => ShotTarget::Output(value("--output")?.clone()),
            "--window" => ShotTarget::Window(window_id(value("--window")?)?),
            "--region" => region(value("--region")?)?,
            flag if flag.starts_with("--") => return Err(bad(format!("unknown flag {flag}"))),
            path => {
                if out.replace(PathBuf::from(path)).is_some() {
                    return Err(bad("shot takes one output path"));
                }
                continue;
            }
        };
        if target.replace(named).is_some() {
            return Err(bad("shot takes one of --output, --window, --region"));
        }
    }
    let out = out.ok_or_else(|| bad("shot needs an output path"))?;
    Ok((target.unwrap_or(ShotTarget::Default), masked, out))
}

/// Why a shot cannot be taken yet (exit 2: could not run), or the Debug1 call that takes it.
/// A named output is captured over ext-image-copy-capture (`capture.rs`); the default output and
/// a region of one are still written by casement's debug object, from inside the compositor.
/// A window is drawn alone by the same debug object (P2-Q3); a masked shot (P4) waits for its
/// pass.
pub fn call(
    target: &ShotTarget,
    masked: Masked,
    out: &Path,
    cwd: &Path,
) -> Result<Call, &'static str> {
    let area = match (masked, target) {
        (Masked::Yes, _) => return Err("shot --masked needs the mask pass (P4)"),
        (Masked::No, ShotTarget::Window(id)) => format!("window:{id}"),
        (Masked::No, ShotTarget::Default) => "output".to_owned(),
        (Masked::No, ShotTarget::Output(name)) => format!("output:{name}"),
        (Masked::No, ShotTarget::Region { x, y, w, h }) => format!("region:{x},{y},{w},{h}"),
    };
    Ok(Call::Shot {
        area,
        path: cwd.join(out).to_string_lossy().into_owned(),
    })
}

fn window_id(text: &str) -> Result<u64, CommandError> {
    text.parse()
        .map_err(|_| bad(format!("--window: not a window id: {text}")))
}

/// `X,Y,W,H`: a point (may be negative) and a size (not empty).
fn region(text: &str) -> Result<ShotTarget, CommandError> {
    let parts: Vec<&str> = text.split(',').collect();
    let [x, y, w, h] = parts.as_slice() else {
        return Err(bad(format!("--region wants X,Y,W,H: {text}")));
    };
    let parsed = (x.parse(), y.parse(), w.parse::<u32>(), h.parse::<u32>());
    match parsed {
        (Ok(x), Ok(y), Ok(w), Ok(h)) if w > 0 && h > 0 => Ok(ShotTarget::Region { x, y, w, h }),
        _ => Err(bad(format!("--region wants X,Y,W,H with a size: {text}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_becomes_a_debug_call_with_an_absolute_path() {
        let cwd = Path::new("/run/work");
        let planned = |target, masked, out: &str| call(&target, masked, Path::new(out), cwd);
        assert_eq!(
            planned(ShotTarget::Default, Masked::No, "a.png"),
            Ok(Call::Shot {
                area: "output".to_owned(),
                path: "/run/work/a.png".to_owned()
            })
        );
        assert_eq!(
            planned(
                ShotTarget::Output("HEADLESS-2".into()),
                Masked::No,
                "/x/b.png"
            ),
            Ok(Call::Shot {
                area: "output:HEADLESS-2".to_owned(),
                path: "/x/b.png".to_owned()
            })
        );
        assert_eq!(
            planned(
                ShotTarget::Region {
                    x: -1,
                    y: 2,
                    w: 3,
                    h: 4
                },
                Masked::No,
                "c.png"
            ),
            Ok(Call::Shot {
                area: "region:-1,2,3,4".to_owned(),
                path: "/run/work/c.png".to_owned()
            })
        );
        assert_eq!(
            planned(ShotTarget::Window(7), Masked::No, "d.png"),
            Ok(Call::Shot {
                area: "window:7".to_owned(),
                path: "/run/work/d.png".to_owned()
            })
        );
        assert!(planned(ShotTarget::Default, Masked::Yes, "e.png").is_err());
    }

    fn shot(text: &str) -> Result<(ShotTarget, Masked, PathBuf), CommandError> {
        parse_shot(
            &text
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn a_shot_names_its_target_masking_and_path_in_any_order() {
        assert_eq!(
            shot("out.png").expect("default"),
            (ShotTarget::Default, Masked::No, PathBuf::from("out.png"))
        );
        assert_eq!(
            shot("--masked a/b.png --output HEADLESS-2").expect("output"),
            (
                ShotTarget::Output("HEADLESS-2".to_owned()),
                Masked::Yes,
                PathBuf::from("a/b.png")
            )
        );
        assert_eq!(
            shot("--window 17 w.png").expect("window").0,
            ShotTarget::Window(17)
        );
        assert_eq!(
            shot("--region -5,10,640,480 r.png").expect("region").0,
            ShotTarget::Region {
                x: -5,
                y: 10,
                w: 640,
                h: 480
            }
        );
    }

    #[test]
    fn a_malformed_shot_is_refused() {
        for text in [
            "",
            "--window x out.png",
            "--region 1,2,3 out.png",
            "--region 0,0,0,5 out.png",
            "--region 0,0,-1,5 out.png",
            "--output out.png",
            "--output A-1 --window 3 out.png",
            "--frobnicate out.png",
            "a.png b.png",
        ] {
            assert!(
                matches!(shot(text), Err(CommandError::BadArgument(_))),
                "{text}"
            );
        }
    }
}
