//! Who a multi-finger swipe belongs to, and which way it goes (03 section 4.2, 4.3).
//!
//! A swipe whose finger count is neither the Spaces swipe's nor Mission Control's is the
//! client's and passes through untouched. One that is the desktop's is held back from the
//! clients from its first step: it decides its axis once the fingers have moved a little
//! (horizontal switches Spaces, vertical drives Mission Control), then drives that axis in whole
//! logical pixels (the sub-pixel remainder is carried to the next step). A swipe that locks onto
//! an axis nobody wants at that finger count is swallowed to its end and does nothing.
//!
//! Pure data and functions: the model feeds each swipe step and acts on what comes back.

use super::{GestureEnd, GestureStep};
use crate::geometry::SubPoint;
use crate::settings::{Enabled, GestureSettings};

/// Sub-pixels in a logical pixel.
const SUB_PER_PX: i64 = 120;

/// How far the fingers travel, in sub-pixels, before the swipe picks its axis (10 logical
/// pixels: far enough that a slightly diagonal start does not decide wrongly).
const LOCK: i64 = 10 * SUB_PER_PX;

/// The direction a locked swipe drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Left and right: the Spaces.
    Horizontal,
    /// Up and down: Mission Control.
    Vertical,
}

/// Where a swipe is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Route {
    /// No swipe is going.
    #[default]
    Idle,
    /// A swipe of a finger count the desktop does not use: the clients' until it ends.
    Passing,
    /// The desktop's swipe, still choosing its axis from the sub-pixels moved so far.
    Deciding {
        /// How many fingers.
        fingers: u8,
        /// Sub-pixels moved rightward so far.
        x: i64,
        /// Sub-pixels moved downward so far.
        y: i64,
    },
    /// The desktop's swipe, driving `axis`.
    Driving {
        /// The axis it locked onto.
        axis: Axis,
        /// The sub-pixels moved along it that did not make a whole pixel yet.
        carry: i64,
    },
    /// The desktop's swipe, locked onto an axis that does nothing at this finger count.
    Dead,
}

/// What a swipe step asks of the desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    /// The swipe locked onto this axis.
    Began(Axis),
    /// The fingers moved this many whole logical pixels along the axis: rightward or downward
    /// are positive, before the natural-direction setting (horizontal moves already follow it).
    Moved(Axis, i32),
    /// The fingers lifted.
    Ended(Axis),
    /// The swipe was abandoned.
    Cancelled(Axis),
}

/// Whose a swipe step is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fate {
    /// The clients': deliver it.
    Pass,
    /// The desktop's: hold it back from the clients, and do these.
    Take(Vec<Drive>),
}

/// The route after `step`, and whose step it was.
///
/// ```
/// use casement_core::input::GestureStep;
/// use casement_core::input::gesture_route::{Fate, Route, route};
/// use casement_core::settings::GestureSettings;
///
/// let settings = GestureSettings::defaults();
/// let (_, fate) = route(Route::Idle, GestureStep::Begin { fingers: 2 }, &settings);
/// assert_eq!(fate, Fate::Pass);
/// let (next, fate) = route(Route::Idle, GestureStep::Begin { fingers: 4 }, &settings);
/// assert!(matches!(next, Route::Deciding { fingers: 4, .. }));
/// assert_eq!(fate, Fate::Take(Vec::new()));
/// ```
pub fn route(
    state: Route,
    step: GestureStep<SubPoint>,
    settings: &GestureSettings,
) -> (Route, Fate) {
    match (state, step) {
        (_, GestureStep::Begin { fingers }) => begin(fingers, settings),
        (Route::Idle, _) => (Route::Idle, Fate::Pass),
        (Route::Passing, GestureStep::Update(_)) => (Route::Passing, Fate::Pass),
        (Route::Passing, GestureStep::End(_)) => (Route::Idle, Fate::Pass),
        (Route::Deciding { fingers, x, y }, GestureStep::Update(delta)) => decide(
            fingers,
            x + i64::from(delta.x.0),
            y + i64::from(delta.y.0),
            settings,
        ),
        (Route::Deciding { .. } | Route::Dead, GestureStep::End(_)) => {
            (Route::Idle, Fate::Take(Vec::new()))
        }
        (Route::Dead, GestureStep::Update(_)) => (Route::Dead, Fate::Take(Vec::new())),
        (Route::Driving { axis, carry }, GestureStep::Update(delta)) => {
            drive(axis, carry, delta, settings)
        }
        (Route::Driving { axis, .. }, GestureStep::End(GestureEnd::Lifted)) => {
            (Route::Idle, Fate::Take(vec![Drive::Ended(axis)]))
        }
        (Route::Driving { axis, .. }, GestureStep::End(GestureEnd::Cancelled)) => {
            (Route::Idle, Fate::Take(vec![Drive::Cancelled(axis)]))
        }
    }
}

/// A swipe of `fingers` began: the desktop's when either swipe uses that count.
fn begin(fingers: u8, settings: &GestureSettings) -> (Route, Fate) {
    match settings.spaces.matches(fingers) || settings.mission_control.matches(fingers) {
        true => (
            Route::Deciding {
                fingers,
                x: 0,
                y: 0,
            },
            Fate::Take(Vec::new()),
        ),
        false => (Route::Passing, Fate::Pass),
    }
}

/// The fingers have moved `(x, y)` sub-pixels in all: pick the axis once they went far enough.
fn decide(fingers: u8, x: i64, y: i64, settings: &GestureSettings) -> (Route, Fate) {
    if x.abs().max(y.abs()) < LOCK {
        return (Route::Deciding { fingers, x, y }, Fate::Take(Vec::new()));
    }
    let (axis, along, wanted) = match x.abs() >= y.abs() {
        true => (Axis::Horizontal, x, settings.spaces.matches(fingers)),
        false => (Axis::Vertical, y, settings.mission_control.matches(fingers)),
    };
    match wanted {
        false => (Route::Dead, Fate::Take(Vec::new())),
        true => {
            let (route, moved) = advance(axis, 0, along, settings);
            let drives = [Some(Drive::Began(axis)), moved]
                .into_iter()
                .flatten()
                .collect();
            (route, Fate::Take(drives))
        }
    }
}

/// The fingers moved by `delta` on a swipe driving `axis`.
fn drive(axis: Axis, carry: i64, delta: SubPoint, settings: &GestureSettings) -> (Route, Fate) {
    let along = match axis {
        Axis::Horizontal => i64::from(delta.x.0),
        Axis::Vertical => i64::from(delta.y.0),
    };
    let (route, moved) = advance(axis, carry, along, settings);
    (route, Fate::Take(moved.into_iter().collect()))
}

/// `along` more sub-pixels on `axis` after `carry`: the route with the new remainder, and the
/// whole pixels moved, if any.
fn advance(
    axis: Axis,
    carry: i64,
    along: i64,
    settings: &GestureSettings,
) -> (Route, Option<Drive>) {
    let total = carry + along;
    let pixels = i32::try_from(total / SUB_PER_PX).unwrap_or(i32::MAX);
    let route = Route::Driving {
        axis,
        carry: total % SUB_PER_PX,
    };
    match pixels {
        0 => (route, None),
        _ => (
            route,
            Some(Drive::Moved(axis, directed(axis, pixels, settings))),
        ),
    }
}

/// `pixels` along `axis` as the person means it: with natural direction off the Spaces move
/// against the fingers. Mission Control's up and down do not change.
fn directed(axis: Axis, pixels: i32, settings: &GestureSettings) -> i32 {
    match (axis, settings.natural) {
        (Axis::Horizontal, Enabled::Off) => pixels.saturating_neg(),
        _ => pixels,
    }
}
