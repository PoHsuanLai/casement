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
