//! Dragging a window to a Space in Mission Control: the shell's Space targets, and the small
//! machine from a press on a slot to its release. A press that never moves past the threshold
//! is the pick it always was; past it the window follows the pointer and a release over a
//! target moves the window to that Space, a release elsewhere sends it back to its slot.
//!
//! Pure data and functions: the loop feeds pointer positions and acts on what comes back.

use crate::geometry::{LogicalPoint, LogicalRect};
use crate::ids::{OutputKey, WindowId};
use crate::input::Origin;
use porter_core::SpaceId;
use std::collections::BTreeMap;

/// How far the pointer must travel from the press, in logical pixels, before a press on a slot
/// becomes a drag (the Mac's drag threshold is a few pixels too).
pub const DRAG_THRESHOLD: u32 = 8;

/// A place a dragged window can be dropped: the rectangle (global logical pixels) and the Space
/// it moves the window to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpaceTarget {
    /// The Space.
    pub space: SpaceId,
    /// Where it is, in global logical pixels.
    pub rect: LogicalRect,
}

impl SpaceTarget {
    /// A target of `rect` for `space`.
    pub fn new(space: SpaceId, rect: LogicalRect) -> SpaceTarget {
        SpaceTarget { space, rect }
    }
}

/// The Space under the dragged window: on which output, and which Space.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Over {
    /// The output whose bar holds the target.
    pub output: OutputKey,
    /// The Space.
    pub space: SpaceId,
}

/// Every output's Space targets, as the shell set them.
///
/// ```
/// use casement_core::geometry::LogicalPoint;
/// use casement_core::overview_drag::SpaceTargets;
/// use casement_core::OutputKey;
///
/// let targets = SpaceTargets::default();
/// let none = targets.hit(LogicalPoint { x: 1, y: 1 }, |_: &OutputKey| true);
/// assert!(none.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpaceTargets(BTreeMap<OutputKey, Vec<SpaceTarget>>);

impl SpaceTargets {
    /// Forgets the targets of `output`.
    pub fn clear(&mut self, output: &OutputKey) {
        self.0.remove(output);
    }

    /// Forgets every target (the shell went away).
    pub fn clear_all(&mut self) {
        self.0.clear();
    }

    /// Adds `target` to `output`, in front of the ones it already has.
    pub fn add(&mut self, output: OutputKey, target: SpaceTarget) {
        self.0.entry(output).or_default().push(target);
    }

    /// The target of an output that is `live` under `at`: the last one added wins, and outputs
    /// are tried in key order.
    pub fn hit(&self, at: LogicalPoint, live: impl Fn(&OutputKey) -> bool) -> Option<Over> {
        self.0
            .iter()
            .filter(|(output, _)| live(output))
            .find_map(|(output, targets)| {
                targets
                    .iter()
                    .rev()
                    .find(|target| target.rect.contains(at))
                    .map(|target| Over {
                        output: output.clone(),
                        space: target.space.clone(),
                    })
            })
    }

    /// Whether no output has a target.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Where a press on a slot is: not pressed, pressed and not yet moved far, or dragging.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Drag {
    /// No button held on a slot.
    #[default]
    Idle,
    /// The button went down on `window`'s slot at `start`; a release now picks the window.
    Pressed {
        /// The window.
        window: WindowId,
        /// The pointer that pressed.
        origin: Origin,
        /// Where the pointer was.
        start: LogicalPoint,
        /// The slot's top left, so the window keeps its place under the pointer.
        slot: LogicalPoint,
    },
    /// The pointer moved past [`DRAG_THRESHOLD`]: `window` follows it.
    Dragging {
        /// The window.
        window: WindowId,
        /// The pointer that pressed.
        origin: Origin,
        /// Where the pointer pressed.
        start: LogicalPoint,
        /// The slot's top left.
        slot: LogicalPoint,
        /// Where the pointer is now.
        at: LogicalPoint,
        /// The target under the pointer now.
        over: Option<Over>,
    },
}

/// What a pointer move did to a drag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Moved {
    /// Nothing the loop must act on.
    Nothing,
    /// The window follows the pointer (and the target under it, if it changed, is `over`).
    Follows {
        /// The window to place under the pointer.
        window: WindowId,
        /// Its new top left.
        top_left: LogicalPoint,
        /// The target under the pointer, when it is not the one before (`Some(None)`: none now).
        changed: Option<Option<Over>>,
    },
}

/// How a release ended the press.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Released {
    /// There was no press.
    Nothing,
    /// The press never became a drag: pick the window.
    Pick {
        /// The window.
        window: WindowId,
        /// The pointer.
        origin: Origin,
    },
    /// Dropped over a target: move the window to the Space and stay in Mission Control.
    Drop {
        /// The window.
        window: WindowId,
        /// Where it was dropped.
        over: Over,
    },
    /// Dropped elsewhere: the window flies back to its slot.
    FlyBack {
        /// The window.
        window: WindowId,
    },
}

impl Drag {
    /// A press on `window`'s slot (whose top left is `slot`) at `start`.
    pub fn press(
        window: WindowId,
        origin: Origin,
        start: LogicalPoint,
        slot: LogicalPoint,
    ) -> Drag {
        Drag::Pressed {
            window,
            origin,
            start,
            slot,
        }
    }

    /// The window being pressed or dragged.
    pub fn window(&self) -> Option<WindowId> {
        match self {
            Drag::Idle => None,
            Drag::Pressed { window, .. } | Drag::Dragging { window, .. } => Some(*window),
        }
    }

    /// The target under the pointer while dragging.
    pub fn over(&self) -> Option<&Over> {
        match self {
            Drag::Dragging { over, .. } => over.as_ref(),
            _ => None,
        }
    }

    /// The pointer moved to `at`; `targets` and `live` find the target under it.
    ///
    /// ```
    /// use casement_core::geometry::LogicalPoint;
    /// use casement_core::input::Origin;
    /// use casement_core::overview_drag::{Drag, Moved, SpaceTargets};
    /// use casement_core::WindowId;
    ///
    /// let at = |x, y| LogicalPoint { x, y };
    /// let pressed = Drag::press(WindowId(1), Origin::Test, at(100, 100), at(80, 90));
    /// let (still, moved) = pressed.moved(at(103, 100), &SpaceTargets::default(), |_| true);
    /// assert!(matches!(still, Drag::Pressed { .. }) && moved == Moved::Nothing);
    /// let (going, moved) = still.moved(at(140, 100), &SpaceTargets::default(), |_| true);
    /// assert!(matches!(going, Drag::Dragging { .. }));
    /// assert!(matches!(moved, Moved::Follows { top_left, .. } if top_left == at(120, 90)));
    /// ```
    pub fn moved(
        self,
        at: LogicalPoint,
        targets: &SpaceTargets,
        live: impl Fn(&OutputKey) -> bool,
    ) -> (Drag, Moved) {
        match self {
            Drag::Idle => (Drag::Idle, Moved::Nothing),
            Drag::Pressed {
                window,
                origin,
                start,
                slot,
            } => match travelled(start, at) {
                false => (
                    Drag::Pressed {
                        window,
                        origin,
                        start,
                        slot,
                    },
                    Moved::Nothing,
                ),
                true => {
                    let over = targets.hit(at, live);
                    let moved = Moved::Follows {
                        window,
                        top_left: carried(slot, start, at),
                        changed: over.clone().map(Some),
                    };
                    let drag = Drag::Dragging {
                        window,
                        origin,
                        start,
                        slot,
                        at,
                        over,
                    };
                    (drag, moved)
                }
            },
            Drag::Dragging {
                window,
                origin,
                start,
                slot,
                over: before,
                ..
            } => {
                let over = targets.hit(at, live);
                let changed = (over != before).then(|| over.clone());
                let moved = Moved::Follows {
                    window,
                    top_left: carried(slot, start, at),
                    changed,
                };
                let drag = Drag::Dragging {
                    window,
                    origin,
                    start,
                    slot,
                    at,
                    over,
                };
                (drag, moved)
            }
        }
    }

    /// The button went up: how the press ended. The machine is idle after.
    ///
    /// ```
    /// use casement_core::geometry::LogicalPoint;
    /// use casement_core::input::Origin;
    /// use casement_core::overview_drag::{Drag, Released};
    /// use casement_core::WindowId;
    ///
    /// let at = LogicalPoint { x: 5, y: 5 };
    /// let pressed = Drag::press(WindowId(3), Origin::Test, at, at);
    /// assert!(matches!(pressed.released(), Released::Pick { window: WindowId(3), .. }));
    /// assert_eq!(Drag::Idle.released(), Released::Nothing);
    /// ```
    pub fn released(self) -> Released {
        match self {
            Drag::Idle => Released::Nothing,
            Drag::Pressed { window, origin, .. } => Released::Pick { window, origin },
            Drag::Dragging {
                window,
                over: Some(over),
                ..
            } => Released::Drop { window, over },
            Drag::Dragging {
                window, over: None, ..
            } => Released::FlyBack { window },
        }
    }
}

/// Whether `at` is past the threshold from `start`.
fn travelled(start: LogicalPoint, at: LogicalPoint) -> bool {
    let dx = i64::from(at.x) - i64::from(start.x);
    let dy = i64::from(at.y) - i64::from(start.y);
    let limit = i64::from(DRAG_THRESHOLD);
    dx * dx + dy * dy > limit * limit
}

/// The slot's top left carried along with the pointer's travel from `start` to `at`.
fn carried(slot: LogicalPoint, start: LogicalPoint, at: LogicalPoint) -> LogicalPoint {
    LogicalPoint {
        x: slot.x.saturating_add(at.x.saturating_sub(start.x)),
        y: slot.y.saturating_add(at.y.saturating_sub(start.y)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::LogicalSize;
    use crate::ids::OutputName;
    use crate::output::{EdidFacts, EdidSerial};

    fn at(x: i32, y: i32) -> LogicalPoint {
        LogicalPoint { x, y }
    }

    fn output(name: &str) -> OutputKey {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse(name).expect("a name"))
    }

    fn space(id: &str) -> SpaceId {
        SpaceId::parse(id).expect("an id")
    }

    fn rect(x: i32, y: i32, w: u32, h: u32) -> LogicalRect {
        LogicalRect::new(at(x, y), LogicalSize { w, h })
    }

    /// Two targets on one output: s-a at the left, s-b overlapping its right edge in front.
    fn targets() -> (SpaceTargets, OutputKey) {
        let out = output("HDMI-A-1");
        let mut targets = SpaceTargets::default();
        targets.add(
            out.clone(),
            SpaceTarget::new(space("s-a"), rect(0, 0, 100, 50)),
        );
        targets.add(
            out.clone(),
            SpaceTarget::new(space("s-b"), rect(90, 0, 100, 50)),
        );
        (targets, out)
    }

    #[test]
    fn a_hit_takes_the_front_most_target_of_a_live_output() {
        let (targets, out) = targets();
        // name, point, live, space
        let cases = [
            ("left only", at(10, 10), true, Some("s-a")),
            ("overlap: the later one", at(95, 10), true, Some("s-b")),
            ("right only", at(150, 10), true, Some("s-b")),
            ("outside", at(250, 10), true, None),
            ("below", at(10, 60), true, None),
            ("an output that left", at(10, 10), false, None),
        ];
        for (name, point, live, want) in cases {
            let got = targets
                .hit(point, |key| live && *key == out)
                .map(|over| over.space);
            assert_eq!(got, want.map(space), "{name}");
        }
    }

    #[test]
    fn clearing_an_output_forgets_only_its_targets() {
        let (mut targets, out) = targets();
        let other = output("DP-1");
        targets.add(
            other.clone(),
            SpaceTarget::new(space("s-c"), rect(0, 0, 10, 10)),
        );
        targets.clear(&out);
        assert!(
            targets
                .hit(at(5, 5), |_| true)
                .is_some_and(|o| o.output == other)
        );
        targets.clear_all();
        assert!(targets.is_empty());
    }

    #[test]
    fn a_press_becomes_a_drag_only_past_the_threshold() {
        let none = SpaceTargets::default();
        let start = at(100, 100);
        // name, pointer, dragging
        let cases = [
            ("on the spot", at(100, 100), false),
            ("inside", at(105, 105), false),
            ("exactly the threshold", at(108, 100), false),
            ("just past", at(109, 100), true),
            ("far", at(300, 40), true),
        ];
        for (name, pointer, dragging) in cases {
            let press = Drag::press(WindowId(1), Origin::Test, start, at(80, 90));
            let (drag, _) = press.moved(pointer, &none, |_| true);
            assert_eq!(matches!(drag, Drag::Dragging { .. }), dragging, "{name}");
        }
    }

    #[test]
    fn the_window_keeps_its_place_under_the_pointer() {
        let none = SpaceTargets::default();
        let press = Drag::press(WindowId(1), Origin::Test, at(100, 100), at(80, 90));
        let (drag, first) = press.moved(at(150, 120), &none, |_| true);
        assert!(matches!(first, Moved::Follows { top_left, .. } if top_left == at(130, 110)));
        let (_, next) = drag.moved(at(160, 100), &none, |_| true);
        assert!(matches!(next, Moved::Follows { top_left, .. } if top_left == at(140, 90)));
    }

    #[test]
    fn the_target_under_the_window_is_reported_when_it_changes() {
        let (targets, out) = targets();
        let live = |key: &OutputKey| *key == out;
        let press = Drag::press(WindowId(1), Origin::Test, at(500, 500), at(480, 480));
        let (drag, _) = press.moved(at(10, 10), &targets, live);
        assert_eq!(drag.over().map(|o| o.space.clone()), Some(space("s-a")));
        // Still over s-a: no change reported.
        let (drag, again) = drag.moved(at(20, 10), &targets, live);
        assert!(matches!(again, Moved::Follows { changed: None, .. }));
        // Over s-b, then over none (Some(None)).
        let (drag, onto) = drag.moved(at(150, 10), &targets, live);
        assert!(matches!(
            onto,
            Moved::Follows {
                changed: Some(Some(_)),
                ..
            }
        ));
        let (_, off) = drag.moved(at(400, 400), &targets, live);
        assert!(matches!(
            off,
            Moved::Follows {
                changed: Some(None),
                ..
            }
        ));
    }

    #[test]
    fn a_release_picks_drops_or_flies_back() {
        let (targets, out) = targets();
        let live = |key: &OutputKey| *key == out;
        let press = || Drag::press(WindowId(7), Origin::Test, at(500, 500), at(480, 480));
        assert!(matches!(
            press().released(),
            Released::Pick {
                window: WindowId(7),
                ..
            }
        ));
        let (over_b, _) = press().moved(at(150, 10), &targets, live);
        assert!(matches!(
            over_b.released(),
            Released::Drop { window: WindowId(7), over } if over.space == space("s-b")
        ));
        let (nowhere, _) = press().moved(at(400, 400), &targets, live);
        assert!(matches!(
            nowhere.released(),
            Released::FlyBack {
                window: WindowId(7),
                ..
            }
        ));
    }
}
