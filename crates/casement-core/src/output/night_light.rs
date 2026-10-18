//! Night light: the screen warms toward a colour temperature on a schedule. Time is an argument
//! (minutes since local midnight, which `casement/src/time/` reads); everything here is integers.
//! The result is a [`GammaRamp`] for `Effect::SetGamma`, the same path `zwlr_gamma_control_v1`
//! clients use.

use super::gamma::{GammaRamp, GammaTable};

const DAY: u16 = 1440;

/// A colour temperature in kelvin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Kelvin(pub u16);

impl Kelvin {
    /// The screen's own white: no tint.
    pub const NEUTRAL: Kelvin = Kelvin(6500);
    /// The warmest the table goes.
    pub const WARMEST: Kelvin = Kelvin(1000);
    /// The default night warmth.
    pub const NIGHT: Kelvin = Kelvin(3400);
}

/// How much of each channel survives, in thousandths (1000 is untouched).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tint(pub [u16; 3]);

impl Tint {
    /// No tint.
    pub const NONE: Tint = Tint([1000; 3]);
}

/// Blackbody tints from 1000 K to 6500 K in 500 K steps, each channel against 6500 K's.
const TABLE: [[u16; 3]; 12] = [
    [1000, 267, 0],
    [1000, 426, 0],
    [1000, 539, 56],
    [1000, 626, 280],
    [1000, 697, 440],
    [1000, 758, 563],
    [1000, 810, 664],
    [1000, 856, 750],
    [1000, 897, 824],
    [1000, 935, 889],
    [1000, 969, 947],
    [1000, 1000, 1000],
];
const STEP: u16 = 500;

impl Kelvin {
    /// The tint of a blackbody at this temperature, linearly between table rows; outside the
    /// table it is the nearest end.
    ///
    /// ```
    /// use casement_core::output::{Kelvin, Tint};
    ///
    /// assert_eq!(Kelvin::NEUTRAL.tint(), Tint::NONE);
    /// assert_eq!(Kelvin(3000).tint(), Tint([1000, 697, 440]));
    /// ```
    pub fn tint(self) -> Tint {
        let clamped = self.0.clamp(Kelvin::WARMEST.0, Kelvin::NEUTRAL.0);
        let offset = clamped - Kelvin::WARMEST.0;
        let (row, rest) = (usize::from(offset / STEP), u32::from(offset % STEP));
        let (low, high) = (TABLE[row], TABLE[(row + 1).min(TABLE.len() - 1)]);
        let channel = |i: usize| -> u16 {
            let (a, b) = (i32::from(low[i]), i32::from(high[i]));
            let step = i32::try_from(rest).unwrap_or(0);
            u16::try_from(a + (b - a) * step / i32::from(STEP)).unwrap_or(1000)
        };
        Tint([channel(0), channel(1), channel(2)])
    }
}

impl Tint {
    /// This tint `strength` thousandths of the way from none (0) to itself (1000).
    pub fn eased(self, strength: u16) -> Tint {
        let strength = u32::from(strength.min(1000));
        let channel = |value: u16| -> u16 {
            let lost = (1000 - u32::from(value)) * strength / 1000;
            u16::try_from(1000 - lost).unwrap_or(1000)
        };
        Tint(self.0.map(channel))
    }
}

/// A minute of the local day, `0..1440`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MinuteOfDay(u16);

impl MinuteOfDay {
    /// The minute `minutes` names, wrapped into the day.
    pub fn wrapping(minutes: u16) -> MinuteOfDay {
        MinuteOfDay(minutes % DAY)
    }

    /// Minutes since midnight.
    pub fn get(self) -> u16 {
        self.0
    }

    /// Minutes from `self` forward to `other`, wrapping past midnight (0 for the same minute).
    fn until(self, other: MinuteOfDay) -> u16 {
        (other.0 + DAY - self.0) % DAY
    }
}

/// When night light is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Schedule {
    /// Never.
    Off,
    /// All day.
    Always,
    /// From one minute to another, across midnight when `to` is before `from`. This is also
    /// sunset to sunrise: sill (which knows the place) writes those times in.
    Between {
        /// Warming starts.
        from: MinuteOfDay,
        /// Warming ends.
        to: MinuteOfDay,
    },
}

/// A night light: its schedule, how warm it gets and how long it takes to get there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NightLight {
    /// When it is on.
    pub schedule: Schedule,
    /// How warm the screen gets.
    pub warmth: Kelvin,
    /// Minutes to fade in after `from` and out before `to`.
    pub fade_minutes: u16,
}

impl NightLight {
    /// How far into its warming the light is at `now`, in thousandths.
    ///
    /// ```
    /// use casement_core::output::{Kelvin, MinuteOfDay, NightLight, Schedule};
    ///
    /// let at = MinuteOfDay::wrapping;
    /// let light = NightLight {
    ///     schedule: Schedule::Between { from: at(1260), to: at(420) },
    ///     warmth: Kelvin::NIGHT,
    ///     fade_minutes: 30,
    /// };
    /// assert_eq!(light.strength(at(720)), 0);
    /// assert_eq!(light.strength(at(1275)), 500);
    /// assert_eq!(light.strength(at(0)), 1000);
    /// ```
    pub fn strength(&self, now: MinuteOfDay) -> u16 {
        match self.schedule {
            Schedule::Off => 0,
            Schedule::Always => 1000,
            Schedule::Between { from, to } => {
                let (length, elapsed) = (from.until(to), from.until(now));
                match elapsed < length {
                    false => 0,
                    true => self
                        .ramp(elapsed)
                        .min(self.ramp(length - elapsed))
                        .min(1000),
                }
            }
        }
    }

    /// `minutes` into a fade as thousandths, saturating at 1000.
    fn ramp(&self, minutes: u16) -> u16 {
        match self.fade_minutes {
            0 => 1000,
            fade => u16::try_from(u32::from(minutes) * 1000 / u32::from(fade)).unwrap_or(1000),
        }
    }

    /// The tint at `now`.
    pub fn tint(&self, now: MinuteOfDay) -> Tint {
        self.warmth.tint().eased(self.strength(now))
    }

    /// The ramp to drive an output with at `now`: the identity when the light is off, else a
    /// table of `size` steps. A size under two cannot hold a ramp and gives the identity.
    ///
    /// ```
    /// use casement_core::output::{GammaRamp, MinuteOfDay, NightLight};
    ///
    /// let off = NightLight::off();
    /// assert_eq!(off.ramp_at(MinuteOfDay::wrapping(0), 256), GammaRamp::Identity);
    /// ```
    pub fn ramp_at(&self, now: MinuteOfDay, size: u32) -> GammaRamp {
        match self.tint(now) {
            Tint::NONE => GammaRamp::Identity,
            tint => GammaTable::tinted(size, tint).map_or(GammaRamp::Identity, GammaRamp::Table),
        }
    }

    /// A light that never comes on.
    pub fn off() -> NightLight {
        NightLight {
            schedule: Schedule::Off,
            warmth: Kelvin::NIGHT,
            fade_minutes: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes: u16) -> MinuteOfDay {
        MinuteOfDay::wrapping(minutes)
    }

    fn evening() -> NightLight {
        NightLight {
            schedule: Schedule::Between {
                from: at(21 * 60),
                to: at(7 * 60),
            },
            warmth: Kelvin::NIGHT,
            fade_minutes: 30,
        }
    }

    #[test]
    fn strength_over_the_day() {
        let rows = [
            (12 * 60, 0),
            (20 * 60 + 59, 0),
            (21 * 60, 0),
            (21 * 60 + 15, 500),
            (21 * 60 + 30, 1000),
            (23 * 60 + 59, 1000),
            (0, 1000),
            (6 * 60 + 30, 1000),
            (6 * 60 + 45, 500),
            (7 * 60, 0),
            (7 * 60 + 1, 0),
        ];
        for (minute, want) in rows {
            assert_eq!(evening().strength(at(minute)), want, "minute {minute}");
        }
    }

    #[test]
    fn a_daytime_window_does_not_wrap() {
        let light = NightLight {
            schedule: Schedule::Between {
                from: at(60),
                to: at(120),
            },
            fade_minutes: 0,
            ..evening()
        };
        assert_eq!(light.strength(at(59)), 0);
        assert_eq!(light.strength(at(60)), 1000);
        assert_eq!(light.strength(at(119)), 1000);
        assert_eq!(light.strength(at(120)), 0);
    }

    #[test]
    fn an_empty_window_and_the_plain_schedules() {
        let same = Schedule::Between {
            from: at(5),
            to: at(5),
        };
        let strengths = [Schedule::Off, Schedule::Always, same].map(|schedule| {
            NightLight {
                schedule,
                ..evening()
            }
            .strength(at(5))
        });
        assert_eq!(strengths, [0, 1000, 0]);
    }

    #[test]
    fn tints_follow_the_blackbody_table() {
        assert_eq!(Kelvin(6500).tint(), Tint::NONE);
        assert_eq!(Kelvin(9000).tint(), Tint::NONE);
        assert_eq!(Kelvin(500).tint(), Tint([1000, 267, 0]));
        assert_eq!(Kelvin(3400).tint(), Tint([1000, 745, 538]));
        assert_eq!(Tint([1000, 600, 400]).eased(500), Tint([1000, 800, 700]));
        assert_eq!(Tint([1000, 600, 400]).eased(0), Tint::NONE);
    }

    #[test]
    fn the_ramp_is_identity_off_and_warm_on() {
        assert_eq!(evening().ramp_at(at(12 * 60), 256), GammaRamp::Identity);
        let GammaRamp::Table(table) = evening().ramp_at(at(0), 256) else {
            panic!("a table at midnight");
        };
        assert_eq!(table.size(), 256);
        assert_eq!(table.red()[255], 65535);
        assert!(table.blue()[255] < table.green()[255]);
        assert!(table.green()[255] < table.red()[255]);
        assert_eq!(table.blue()[0], 0);
        assert_eq!(evening().ramp_at(at(0), 1), GammaRamp::Identity);
    }
}
