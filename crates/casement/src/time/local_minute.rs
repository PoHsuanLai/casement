//! The night light's tick: the local wall-clock minute, told to the model at start and at each
//! minute boundary after. It lives here because it reads the wall clock and arms a calloop
//! `Timer` (row B8). The conversion from jiff's `Zoned` and the time to the next boundary are
//! pure, so a table tests them with zones built from POSIX strings and no clock.

use crate::core_loop;
use crate::state::State;
use casement_core::Event;
use casement_core::output::MinuteOfDay;
use jiff::Zoned;
use smithay::reexports::calloop::LoopHandle;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use std::time::Duration;

/// The wait from one minute boundary to the next.
const MINUTE: Duration = Duration::from_secs(60);

/// The minute of the local day `local` falls in: its wall-clock hour and minute, so a day
/// that skips or repeats an hour (daylight saving) skips or repeats those minutes too.
pub fn minute_of(local: &Zoned) -> MinuteOfDay {
    let minutes = i32::from(local.hour()) * 60 + i32::from(local.minute());
    MinuteOfDay::wrapping(u16::try_from(minutes).unwrap_or(0))
}

/// How long from `local` to the next minute boundary (a whole minute when on one).
pub fn until_next_minute(local: &Zoned) -> Duration {
    let seconds = u64::try_from(local.second()).unwrap_or(0);
    let nanos = u32::try_from(local.subsec_nanosecond()).unwrap_or(0);
    MINUTE.saturating_sub(Duration::new(seconds, nanos))
}

/// Tells the model the local minute on the loop's first turn and at every minute boundary after.
/// Each firing reads the clock again and sleeps to the next boundary, not a fixed minute, so a
/// suspend or a clock change is caught up at the next firing.
pub fn tick_every_minute(handle: &LoopHandle<'static, State>) {
    let inserted = handle.insert_source(Timer::immediate(), |_, (), state: &mut State| {
        let now = Zoned::now();
        core_loop::feed(state, vec![Event::NightLightTick(minute_of(&now))]);
        TimeoutAction::ToDuration(until_next_minute(&now))
    });
    if let Err(error) = inserted {
        tracing::warn!(%error, "the night light's minute timer could not be armed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::Timestamp;
    use jiff::tz::TimeZone;

    /// New York's rules as a POSIX string: no zone database is read.
    const NEW_YORK: &str = "EST5EDT,M3.2.0,M11.1.0";

    fn local(zone: &str, second: i64, nanos: i32) -> Zoned {
        let zone = TimeZone::posix(zone).expect("a POSIX zone");
        Timestamp::new(second, nanos)
            .expect("a timestamp")
            .to_zoned(zone)
    }

    fn wall(zone: &str, civil: &str) -> Zoned {
        let zone = TimeZone::posix(zone).expect("a POSIX zone");
        civil
            .parse::<jiff::civil::DateTime>()
            .expect("a civil time")
            .to_zoned(zone)
            .expect("a zoned time")
    }

    #[test]
    fn the_minute_is_the_wall_clock_hour_and_minute() {
        let table = [
            ("UTC0", "2026-06-01T00:00:00", 0),
            ("UTC0", "2026-06-01T00:01:30", 1),
            ("UTC0", "2026-06-01T12:00:00", 720),
            ("UTC0", "2026-06-01T23:59:59", 1439),
            (NEW_YORK, "2026-03-08T01:59:00", 119),
            (NEW_YORK, "2026-03-08T03:00:00", 180),
            (NEW_YORK, "2026-11-01T00:30:00", 30),
        ];
        for (zone, civil, minute) in table {
            assert_eq!(minute_of(&wall(zone, civil)).get(), minute, "{civil}");
        }
    }

    #[test]
    fn a_zone_ahead_of_utc_reads_its_own_wall_clock() {
        // 2026-06-01T20:30:00Z is half past five in the morning at +09:00.
        let tokyo = local("JST-9", 1_780_345_800, 0);
        assert_eq!(minute_of(&tokyo).get(), 5 * 60 + 30);
    }

    #[test]
    fn the_spring_forward_day_skips_the_hour_it_loses() {
        // 06:59:00Z is 01:59 EST; a minute later it is 03:00 EDT.
        let before = local(NEW_YORK, 1_772_953_140, 0);
        let after = local(NEW_YORK, 1_772_953_200, 0);
        assert_eq!(minute_of(&before).get(), 119);
        assert_eq!(minute_of(&after).get(), 180);
    }

    #[test]
    fn the_fall_back_day_repeats_the_hour_it_gains() {
        // 05:30Z and 06:30Z on 2026-11-01 are both 01:30 on the wall (EDT, then EST).
        let first = local(NEW_YORK, 1_793_511_000, 0);
        let second = local(NEW_YORK, 1_793_514_600, 0);
        assert_eq!(minute_of(&first), minute_of(&second));
        assert_eq!(minute_of(&first).get(), 90);
    }

    #[test]
    fn the_wait_runs_to_the_next_minute_boundary() {
        let table = [
            (0, 0, Duration::from_secs(60)),
            (1, 0, Duration::from_secs(59)),
            (59, 0, Duration::from_secs(1)),
            (30, 500_000_000, Duration::from_millis(29_500)),
            (59, 999_999_999, Duration::from_nanos(1)),
        ];
        for (second, nanos, wait) in table {
            // 2026-06-01T00:00:00Z plus whole minutes keeps the seconds as given.
            let now = local("UTC0", 1_780_272_000 + second, nanos);
            assert_eq!(until_next_minute(&now), wait, "{second}.{nanos}");
        }
    }
}
