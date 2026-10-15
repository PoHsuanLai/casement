//! The one conversion between the stored document and the model's settings: `to_core` reads the
//! document, and the `*_doc` functions are its inverse, which is how each section's `Default`
//! is the model's own default (a default is written once, in `casement-core`).

use super::action::ActionWords;
use super::doc::CasementDoc;
use super::sections::{
    BindingsSection, FirstSight, GpuSection, KeyboardSection, NightLightSection, OutputsSection,
    PointerSection, SpacesSection, TouchpadSection, WindowsSection,
};
use casement_core::CoreError;
use casement_core::gpu::{ByPathId, GpuChoice};
use casement_core::input::{Binding, Bindings, Chord};
use casement_core::output::{Kelvin, MinuteOfDay, ScalePolicy};
use casement_core::settings::{
    CasementSettings, GpuSettings, KeyboardSettings, NightLightSettings, OutputSettings,
    PointerProfile, PointerSettings, RepeatRate, SpaceSettings, WindowSettings, XkbNames,
};
use ds_core::geometry::scale::Scale;
use ds_settings::{Count, Ms};
use std::time::Duration;

/// The smallest and largest `outputs.fixed_scale`, in percent (the schema's range).
const FIXED_SCALE: (u16, u16) = (100, 400);

/// The settings `doc` says; every key it leaves out takes `CasementSettings::defaults`'s value.
pub fn to_core(doc: &CasementDoc) -> CasementSettings {
    CasementSettings {
        windows: windows(&doc.windows),
        spaces: spaces(&doc.spaces),
        keyboard: keyboard(&doc.keyboard),
        bindings: bindings(&doc.bindings),
        outputs: outputs(&doc.outputs),
        pointer: pointer(&doc.pointer, &doc.touchpad),
        gpu: gpu(&doc.gpu),
        night_light: night_light(&doc.night_light),
    }
}

fn windows(doc: &WindowsSection) -> WindowSettings {
    WindowSettings {
        minimise_effect: doc.minimise_effect,
        double_click_title: doc.double_click_title,
        tile_edges: doc.tile_edges,
        tile_top_fills: doc.tile_top_fills,
        tile_option_drag: doc.tile_option_drag,
        tile_margins: doc.tile_margins,
        first_click: doc.first_click,
        quit_grace: duration(doc.quit_grace_ms),
        not_responding: duration(doc.not_responding_ms),
    }
}

pub(super) fn windows_doc(core: &WindowSettings) -> WindowsSection {
    WindowsSection {
        minimise_effect: core.minimise_effect,
        double_click_title: core.double_click_title,
        tile_edges: core.tile_edges,
        tile_top_fills: core.tile_top_fills,
        tile_option_drag: core.tile_option_drag,
        tile_margins: core.tile_margins,
        first_click: core.first_click,
        quit_grace_ms: millis(core.quit_grace),
        not_responding_ms: millis(core.not_responding),
    }
}

fn spaces(doc: &SpacesSection) -> SpaceSettings {
    SpaceSettings {
        per_display: doc.per_display,
        auto_rearrange: doc.auto_rearrange,
        switch_on_activate: doc.switch_on_activate,
    }
}

pub(super) fn spaces_doc(core: &SpaceSettings) -> SpacesSection {
    SpacesSection {
        per_display: core.per_display,
        auto_rearrange: core.auto_rearrange,
        switch_on_activate: core.switch_on_activate,
    }
}

/// A repeat rate below 1 a second would never repeat (and divide by zero in the interval).
fn keyboard(doc: &KeyboardSection) -> KeyboardSettings {
    KeyboardSettings {
        layout: XkbNames {
            rules: doc.xkb_rules.clone(),
            model: doc.xkb_model.clone(),
            layout: doc.xkb_layout.clone(),
            variant: doc.xkb_variant.clone(),
            options: doc.xkb_options.clone(),
        },
        repeat_delay: duration(doc.repeat_delay_ms),
        repeat_rate: RepeatRate(u32::from(doc.repeat_rate.0).max(1)),
    }
}

pub(super) fn keyboard_doc(core: &KeyboardSettings) -> KeyboardSection {
    KeyboardSection {
        xkb_rules: core.layout.rules.clone(),
        xkb_model: core.layout.model.clone(),
        xkb_layout: core.layout.layout.clone(),
        xkb_variant: core.layout.variant.clone(),
        xkb_options: core.layout.options.clone(),
        repeat_delay_ms: millis(core.repeat_delay),
        repeat_rate: Count(u16::try_from(core.repeat_rate.0).unwrap_or(u16::MAX)),
    }
}

/// The bindings in chord-text order; one whose chord or action does not parse is dropped with
/// a warning.
fn bindings(doc: &BindingsSection) -> Bindings {
    Bindings(
        doc.0
            .iter()
            .filter_map(|(text, words)| match binding(text, words) {
                Ok(binding) => Some(binding),
                Err(error) => {
                    tracing::warn!(%error, "a binding that cannot be read is left out");
                    None
                }
            })
            .collect(),
    )
}

fn binding(text: &str, words: &ActionWords) -> Result<Binding, CoreError> {
    Ok(Binding {
        chord: Chord::parse(text)?,
        action: words.parse()?,
    })
}

pub(super) fn bindings_doc(core: &Bindings) -> BindingsSection {
    BindingsSection(
        core.0
            .iter()
            .map(|binding| (binding.chord.text(), ActionWords::of(&binding.action)))
            .collect(),
    )
}

fn outputs(doc: &OutputsSection) -> OutputSettings {
    OutputSettings {
        first_sight: match doc.first_sight_scale {
            FirstSight::WholeLikeMac => ScalePolicy::WholeLikeMac,
            FirstSight::Fixed => ScalePolicy::Fixed(Scale::from_percent(
                doc.fixed_scale.0.clamp(FIXED_SCALE.0, FIXED_SCALE.1),
            )),
        },
    }
}

/// A fixed scale that is not a whole percent shows as the nearest one below.
pub(super) fn outputs_doc(core: &OutputSettings) -> OutputsSection {
    let (first_sight_scale, fixed_scale) = match core.first_sight {
        ScalePolicy::WholeLikeMac => (FirstSight::WholeLikeMac, Count(FIXED_SCALE.0)),
        ScalePolicy::Fixed(scale) => (FirstSight::Fixed, percent_of(scale)),
    };
    OutputsSection {
        first_sight_scale,
        fixed_scale,
    }
}

fn percent_of(scale: Scale) -> Count {
    let percent = u64::from(scale.numerator()) * 100 / u64::from(Scale::DENOMINATOR);
    Count(u16::try_from(percent).unwrap_or(FIXED_SCALE.1))
}

fn pointer(mouse: &PointerSection, touchpad: &TouchpadSection) -> PointerSettings {
    PointerSettings::new(
        PointerProfile::new(
            mouse.accel,
            mouse.natural_scroll,
            mouse.tap,
            mouse.scroll_factor,
            mouse.disable_while_typing,
        ),
        PointerProfile::new(
            touchpad.accel,
            touchpad.natural_scroll,
            touchpad.tap,
            touchpad.scroll_factor,
            touchpad.disable_while_typing,
        ),
    )
}

pub(super) fn mouse_doc(core: &PointerProfile) -> PointerSection {
    PointerSection {
        accel: core.accel,
        natural_scroll: core.natural_scroll,
        tap: core.tap,
        scroll_factor: core.scroll_factor,
        disable_while_typing: core.disable_while_typing,
    }
}

pub(super) fn touchpad_doc(core: &PointerProfile) -> TouchpadSection {
    TouchpadSection {
        accel: core.accel,
        natural_scroll: core.natural_scroll,
        tap: core.tap,
        scroll_factor: core.scroll_factor,
        disable_while_typing: core.disable_while_typing,
    }
}

/// `gpu.render`: `auto`, or a PCI address. Text that is neither (a typo) is logged and renders
/// as `auto`, so a bad key costs only itself and the machine still starts.
fn gpu(doc: &GpuSection) -> GpuSettings {
    let text = doc.render.trim();
    let render = if text.is_empty() || text.eq_ignore_ascii_case(AUTO) {
        GpuChoice::Auto
    } else {
        match ByPathId::parse(text) {
            Ok(id) => GpuChoice::ByPath(id),
            Err(error) => {
                tracing::warn!(%error, "gpu.render is neither `auto` nor a PCI address; using auto");
                GpuChoice::Auto
            }
        }
    };
    GpuSettings::new(render)
}

pub(super) fn gpu_doc(core: &GpuSettings) -> GpuSection {
    GpuSection {
        render: match &core.render {
            GpuChoice::ByPath(id) => id.as_str().to_owned(),
            _ => AUTO.to_owned(),
        },
    }
}

/// `night_light.*`; a time past the day wraps into it and a warmth outside the table stops at its
/// ends.
fn night_light(doc: &NightLightSection) -> NightLightSettings {
    NightLightSettings::new(
        doc.mode,
        (
            MinuteOfDay::wrapping(doc.from.0),
            MinuteOfDay::wrapping(doc.to.0),
        ),
        Kelvin(doc.kelvin.0.clamp(Kelvin::WARMEST.0, Kelvin::NEUTRAL.0)),
        doc.fade_minutes.0,
    )
}

pub(super) fn night_light_doc(core: &NightLightSettings) -> NightLightSection {
    NightLightSection {
        mode: core.mode,
        from: Count(core.from.get()),
        to: Count(core.to.get()),
        kelvin: Count(core.warmth.0),
        fade_minutes: Count(core.fade_minutes),
    }
}

/// What `gpu.render` says for the automatic choice.
const AUTO: &str = "auto";

fn duration(ms: Ms) -> Duration {
    Duration::from_millis(u64::from(ms.0))
}

/// `duration` in whole milliseconds, saturating at what the document can hold.
fn millis(duration: Duration) -> Ms {
    Ms(u16::try_from(duration.as_millis()).unwrap_or(u16::MAX))
}
