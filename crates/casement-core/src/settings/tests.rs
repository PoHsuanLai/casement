use super::*;
use crate::output::ScalePolicy;
use std::time::Duration;

#[test]
fn defaults_follow_the_mac_rows() {
    let settings = CasementSettings::defaults();
    assert_eq!(settings.windows.minimise_effect, MinimiseEffect::Genie);
    assert_eq!(settings.windows.double_click_title, TitleDoubleClick::Zoom);
    assert_eq!(settings.windows.first_click, FirstClick::ActivateOnly);
    assert_eq!(
        (
            settings.windows.tile_edges,
            settings.windows.tile_top_fills,
            settings.windows.tile_option_drag,
            settings.windows.tile_margins
        ),
        (Enabled::On, Enabled::On, Enabled::On, Enabled::On)
    );
    assert_eq!(settings.windows.quit_grace, Duration::from_millis(2000));
    assert_eq!(settings.windows.not_responding, Duration::from_millis(5000));
    assert_eq!(
        (
            settings.spaces.per_display,
            settings.spaces.auto_rearrange,
            settings.spaces.switch_on_activate
        ),
        (Enabled::On, Enabled::Off, Enabled::On)
    );
    assert_eq!(settings.keyboard.repeat_delay, Duration::from_millis(400));
    assert_eq!(settings.keyboard.repeat_rate, RepeatRate(10));
    assert_eq!(settings.keyboard.layout.layout, "");
    assert_eq!(settings.outputs.first_sight, ScalePolicy::WholeLikeMac);
    assert_eq!(settings.pointer, PointerSettings::defaults());
    assert_eq!(settings.pointer.touchpad.natural_scroll, Enabled::On);
    assert_eq!(settings.pointer.mouse.natural_scroll, Enabled::On);
}

#[test]
fn the_default_gpu_is_automatic() {
    assert_eq!(
        CasementSettings::defaults().gpu.render,
        crate::gpu::GpuChoice::Auto
    );
}

#[test]
fn night_light_is_off_by_default_and_schedules_when_asked() {
    use crate::output::{MinuteOfDay, Schedule};
    let settings = CasementSettings::defaults().night_light;
    assert_eq!(settings.mode, NightLightMode::Off);
    assert_eq!(settings.light().schedule, Schedule::Off);
    let scheduled = NightLightSettings {
        mode: NightLightMode::Scheduled,
        ..settings
    };
    assert_eq!(
        scheduled.light().schedule,
        Schedule::Between {
            from: MinuteOfDay::wrapping(22 * 60),
            to: MinuteOfDay::wrapping(7 * 60),
        }
    );
}

#[test]
fn outputs_in_the_skip_list_keep_their_own_curve_and_the_rest_warm() {
    use crate::ids::{OutputKey, OutputName};
    use crate::output::{EdidFacts, EdidSerial, GammaRamp, MinuteOfDay};
    let edid = |serial: EdidSerial| EdidFacts {
        make: "ACME".to_owned(),
        model: "M1".to_owned(),
        serial,
    };
    let output = |connector: &str, serial: EdidSerial| {
        let name = OutputName::parse(connector).expect("a connector name");
        let key = OutputKey::of(&edid(serial), &name);
        (name, key)
    };
    let by_name = output("DP-1", EdidSerial::Unknown);
    let by_key = output("HDMI-A-1", EdidSerial::Known("S1".to_owned()));
    let other = output("DP-2", EdidSerial::Unknown);
    let settings = NightLightSettings {
        mode: NightLightMode::Always,
        ..NightLightSettings::defaults()
    }
    .skipping(vec![
        " DP-1 ".to_owned(),
        by_key.1.as_str().to_owned(),
        "connector:DP-3".to_owned(),
    ]);
    let noon = MinuteOfDay::wrapping(12 * 60);
    let ramp = |(name, key): &(OutputName, OutputKey)| settings.ramp_for(name, key, noon, 256);
    assert_eq!(ramp(&by_name), GammaRamp::Identity, "listed by connector");
    assert_eq!(ramp(&by_key), GammaRamp::Identity, "listed by its key");
    assert!(
        matches!(ramp(&other), GammaRamp::Table(_)),
        "an output not listed is warmed"
    );
    assert_eq!(
        settings.warming_of(&other.0, &other.1),
        OutputWarming::Warmed
    );
    let unlisted = NightLightSettings::defaults();
    assert!(
        unlisted.skip_outputs.is_empty(),
        "nothing is skipped by default"
    );
}

#[test]
fn transparency_is_full_by_default_and_reduced_when_the_switch_is_on() {
    let settings = CasementSettings::defaults().accessibility;
    assert_eq!(settings.reduce_transparency, Enabled::Off);
    assert_eq!(settings.transparency(), Transparency::Full);
    for (switch, wanted) in [
        (Enabled::Off, Transparency::Full),
        (Enabled::On, Transparency::Reduced),
    ] {
        assert_eq!(AccessibilitySettings::new(switch).transparency(), wanted);
    }
    assert_eq!(Transparency::default(), Transparency::Full);
}
