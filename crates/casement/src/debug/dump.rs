//! What the debug object says about the session: the outputs, the windows, the scene, the
//! wait predicates, and the output and fault verbs that change the outputs. JSON is written by
//! hand (a few fixed shapes, escaped strings): casement's own dependencies carry no JSON crate.

use crate::{State, core_loop};
use casement_backend::{BackendKind, VirtualOutputSpec};
use casement_core::geometry::{LogicalPoint, Transform};
use casement_core::output::{Mode, OutputConfig, OutputPower};
use casement_core::time::Millihertz;
use casement_core::{OutputKey, OutputName, WindowId};
use casement_ipc::WaitPredicate;
use casement_ipc::dbus::DebugReply;
use ds_core::geometry::scale::Scale;

/// A JSON string literal.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Frames presented on `output`, or on the first output (by name) when none is named.
pub(super) fn frames_presented(state: &State, output: Option<&OutputName>) -> u64 {
    let BackendKind::Headless(backend) = &state.backend else {
        return 0;
    };
    let outputs = backend.outputs();
    match output {
        Some(name) => outputs.get(name).map_or(0, |o| o.presented),
        None => outputs.iter().next().map_or(0, |(_, o)| o.presented),
    }
}

/// Whether `predicate` holds now.
pub(super) fn met(state: &State, predicate: &WaitPredicate, frames_goal: u64) -> bool {
    let model = &state.core.model;
    let class_of = |id: &WindowId| {
        model
            .windows()
            .get(id)
            .map(|window| window.claimed_id.as_str().to_owned())
    };
    match predicate {
        WaitPredicate::Mapped(app) => model
            .windows()
            .values()
            .any(|window| window.claimed_id.as_str() == app),
        WaitPredicate::Focused(app) => model
            .focus()
            .key
            .and_then(|id| class_of(&id))
            .is_some_and(|class| class == *app),
        WaitPredicate::Gone(app) => model
            .windows()
            .values()
            .all(|window| window.claimed_id.as_str() != app),
        WaitPredicate::Frames { output, .. } => {
            frames_presented(state, output.as_ref()) >= frames_goal
        }
        WaitPredicate::Settled => !state.core.motion.is_moving(state.clock.now()),
    }
}

/// The outputs, in the order the model keeps them (the order `drop-output` counts in).
pub(super) fn outputs(state: &State) -> Vec<(OutputKey, OutputName, OutputConfig)> {
    state
        .core
        .model
        .outputs()
        .0
        .iter()
        .map(|(key, plugged)| (key.clone(), plugged.facts.name.clone(), plugged.config))
        .collect()
}

fn transform_word(transform: Transform) -> &'static str {
    match transform {
        Transform::Normal => "normal",
        Transform::R90 => "90",
        Transform::R180 => "180",
        Transform::R270 => "270",
        Transform::Flipped => "flipped",
        Transform::Flipped90 => "flipped-90",
        Transform::Flipped180 => "flipped-180",
        Transform::Flipped270 => "flipped-270",
    }
}

fn output_json(name: &OutputName, key: &OutputKey, config: &OutputConfig) -> String {
    let power = match config.power {
        OutputPower::On => "on",
        OutputPower::Off => "off",
    };
    let rect = config.logical_rect();
    format!(
        "{{\"name\":{},\"key\":{},\"width\":{},\"height\":{},\"refresh\":{},\"scale\":{},\"x\":{},\"y\":{},\"logical\":[{},{}],\"transform\":{},\"power\":{}}}",
        quoted(name.as_str()),
        quoted(key.as_str()),
        config.mode.size.w,
        config.mode.size.h,
        config.mode.refresh.0,
        config.scale.0,
        config.position.x,
        config.position.y,
        rect.size.w,
        rect.size.h,
        quoted(transform_word(config.transform)),
        quoted(power),
    )
}

fn outputs_json(state: &State) -> String {
    let rows: Vec<String> = outputs(state)
        .iter()
        .map(|(key, name, config)| output_json(name, key, config))
        .collect();
    format!("[{}]", rows.join(","))
}

/// Every window in the shape KWin's window list gave the consumer scripts: `class` (the app
/// id), `caption`, `frame` and `buffer` as `[x, y, w, h]`, `maximize`, `fullscreen`, `active`, plus `identifier` (`w<id>`, the name the
/// shell protocol takes) and `minimise_target` (`[x, y, w, h]` global, or `null` when none is set).
pub(super) fn windows(state: &State) -> String {
    let model = &state.core.model;
    let rows: Vec<String> = model
        .windows()
        .values()
        .map(|window| {
            let target = state
                .core
                .arrangement
                .windows
                .iter()
                .find(|target| target.window == window.id);
            let frame = target.map_or([0, 0, 0, 0], |t| {
                let rect = t.rect;
                [
                    i64::from(rect.origin.x),
                    i64::from(rect.origin.y),
                    i64::from(rect.size.w),
                    i64::from(rect.size.h),
                ]
            });
            let buffer = state.render.textures.window(&window.id).map_or(frame, |entry| {
                [
                    frame[0],
                    frame[1],
                    i64::from(entry.committed.w),
                    i64::from(entry.committed.h),
                ]
            });
            let maximize = i32::from(matches!(
                window.state,
                casement_core::window::WindowState::Filled { .. }
            )) * 3;
            let fullscreen = matches!(
                window.state,
                casement_core::window::WindowState::FullScreen { .. }
            );
            let active = model.focus().key == Some(window.id);
            let minimise_target = model.minimise_target(window.id).map_or_else(
                || "null".to_owned(),
                |target| {
                    let rect = target.rect;
                    format!(
                        "{:?}",
                        [
                            i64::from(rect.origin.x),
                            i64::from(rect.origin.y),
                            i64::from(rect.size.w),
                            i64::from(rect.size.h),
                        ]
                    )
                },
            );
            let minimised = matches!(
                window.state,
                casement_core::window::WindowState::Minimised { .. }
            );
            let minimise_destination = model.minimise_destination(window).map_or_else(
                || "null".to_owned(),
                |target| {
                    let rect = target.rect;
                    format!(
                        "{:?}",
                        [
                            i64::from(rect.origin.x),
                            i64::from(rect.origin.y),
                            i64::from(rect.size.w),
                            i64::from(rect.size.h),
                        ]
                    )
                },
            );
            format!(
                "{{\"identifier\":\"w{}\",\"class\":{},\"caption\":{},\"frame\":{:?},\"buffer\":{:?},\"maximize\":{},\"fullscreen\":{},\"active\":{},\"minimise_target\":{},\"minimised\":{},\"minimise_destination\":{}}}",
                window.id.0,
                quoted(window.claimed_id.as_str()),
                quoted(&window.title.0),
                frame,
                buffer,
                maximize,
                fullscreen,
                active,
                minimise_target,
                minimised,
                minimise_destination,
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// The scene: outputs, windows, the sampled frame as text, and what the backend and the
/// clock hold (a leak shows here).
pub(super) fn scene(state: &State) -> String {
    let frame = core_loop::sample_scene(state).text();
    format!(
        "{{\"outputs\":{},\"windows\":{},\"frame\":{},\"frames\":{}}}",
        outputs_json(state),
        windows(state),
        quoted(&frame),
        frames_presented(state, None),
    )
}

/// `Output(verb, args)`.
pub(super) fn output(state: &mut State, verb: &str, args: &[String]) -> DebugReply {
    match (verb, args) {
        ("list", _) => DebugReply::Json(outputs_json(state)),
        ("add", [spec]) => match VirtualOutputSpec::parse(spec) {
            Ok(spec) => {
                core_loop::plug_output(state, spec);
                DebugReply::Done
            }
            Err(error) => DebugReply::Refused(error.to_string()),
        },
        ("remove", [name]) => match OutputName::parse(name) {
            Ok(name) => {
                core_loop::unplug_output(state, &name);
                DebugReply::Done
            }
            Err(error) => DebugReply::Refused(error.to_string()),
        },
        ("set", [name, settings @ ..]) => set(state, name, settings),
        _ => DebugReply::Refused(format!("output {verb}: not understood")),
    }
}

/// `Output("set", NAME KEY=VALUE...)`: scale in 120ths, `mode=WxH@MILLIHERTZ`, `position=X,Y`,
/// `power=on|off`, `transform=...`.
fn set(state: &mut State, name: &str, settings: &[String]) -> DebugReply {
    let Some((key, config)) = OutputName::parse(name).ok().and_then(|name| {
        outputs(state)
            .into_iter()
            .find(|(_, found, _)| *found == name)
            .map(|(key, _, config)| (key, config))
    }) else {
        return DebugReply::Refused(format!("no output {name}"));
    };
    let changed = settings.iter().try_fold(config, |config, setting| {
        let (k, v) = setting.split_once('=')?;
        change(config, k, v)
    });
    match changed {
        Some(config) => {
            core_loop::configure_output(state, &key, config);
            DebugReply::Done
        }
        None => DebugReply::Refused(format!("output set {name}: {}", settings.join(" "))),
    }
}

/// `config` with one setting applied.
fn change(config: OutputConfig, key: &str, value: &str) -> Option<OutputConfig> {
    Some(match key {
        "scale" => OutputConfig {
            scale: Scale(value.parse().ok().filter(|n| *n > 0)?),
            ..config
        },
        "mode" => {
            let (size, refresh) = value.split_once('@')?;
            let (w, h) = size.split_once('x')?;
            OutputConfig {
                mode: Mode {
                    size: casement_core::geometry::PhysicalSize {
                        w: w.parse().ok().filter(|n| *n > 0)?,
                        h: h.parse().ok().filter(|n| *n > 0)?,
                    },
                    refresh: Millihertz(refresh.parse().ok().filter(|n| *n > 0)?),
                    ..config.mode
                },
                ..config
            }
        }
        "position" => {
            let (x, y) = value.split_once(',')?;
            OutputConfig {
                position: LogicalPoint {
                    x: x.parse().ok()?,
                    y: y.parse().ok()?,
                },
                ..config
            }
        }
        "power" => OutputConfig {
            power: match value {
                "on" => OutputPower::On,
                "off" => OutputPower::Off,
                _ => return None,
            },
            ..config
        },
        "transform" => OutputConfig {
            transform: [
                Transform::Normal,
                Transform::R90,
                Transform::R180,
                Transform::R270,
                Transform::Flipped,
                Transform::Flipped90,
                Transform::Flipped180,
                Transform::Flipped270,
            ]
            .into_iter()
            .find(|t| transform_word(*t) == value)?,
            ..config
        },
        _ => return None,
    })
}

/// `Fault("drop-output", INDEX)`: the output at that place in the list is unplugged.
pub(super) fn drop_output(state: &mut State, index: u32) -> DebugReply {
    let at = usize::try_from(index).unwrap_or(usize::MAX);
    match outputs(state).into_iter().nth(at) {
        Some((_, name, _)) => {
            core_loop::unplug_output(state, &name);
            DebugReply::Done
        }
        None => DebugReply::Refused(format!("no output at index {index}")),
    }
}
