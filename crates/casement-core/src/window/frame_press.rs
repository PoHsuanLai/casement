//! What a press on a server-side frame means: the click count on a titlebar and the action the
//! part pressed asks for. Pure; the edge carries the action out.

use super::frame::{FrameHit, Light};
use super::state::WindowState;
use crate::event::ResizeEdges;
use crate::geometry::LogicalPoint;
use crate::ids::WindowId;
use crate::settings::TitleDoubleClick;
use crate::time::Mono;
use casement_protocols::vocab::Toggle;
use std::time::Duration;

/// How long after a press a second one still makes a double click.
pub const DOUBLE_CLICK_WITHIN: Duration = Duration::from_millis(400);

/// How far, in logical pixels on either axis, a second press may be from the first.
pub const DOUBLE_CLICK_SLOP: i32 = 4;

/// How many times in a row a titlebar was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clicks {
    /// The first press, or one too late or too far from the last.
    Single,
    /// The second press close behind the first.
    Double,
}

/// The last titlebar press, to tell a double click from two single ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClickTracker {
    last: Option<(WindowId, Mono, LogicalPoint)>,
}

impl ClickTracker {
    /// A titlebar press on `window` at `point` at `now`: the tracker after it and the count.
    /// A double click uses the pair up, so a third press starts again.
    ///
    /// ```
    /// use casement_core::WindowId;
    /// use casement_core::geometry::LogicalPoint;
    /// use casement_core::time::Mono;
    /// use casement_core::window::{ClickTracker, Clicks};
    /// use std::time::Duration;
    ///
    /// let at = LogicalPoint { x: 10, y: 10 };
    /// let (tracker, first) = ClickTracker::default().press(WindowId(1), Mono(Duration::ZERO), at);
    /// let (_, second) = tracker.press(WindowId(1), Mono(Duration::from_millis(100)), at);
    /// assert_eq!((first, second), (Clicks::Single, Clicks::Double));
    /// ```
    pub fn press(self, window: WindowId, now: Mono, point: LogicalPoint) -> (ClickTracker, Clicks) {
        let double = self.last.is_some_and(|(held, at, was)| {
            held == window
                && now.since(at) <= DOUBLE_CLICK_WITHIN
                && (point.x - was.x).abs() <= DOUBLE_CLICK_SLOP
                && (point.y - was.y).abs() <= DOUBLE_CLICK_SLOP
        });
        match double {
            true => (ClickTracker { last: None }, Clicks::Double),
            false => (
                ClickTracker {
                    last: Some((window, now, point)),
                },
                Clicks::Single,
            ),
        }
    }
}

/// What a press on the frame asks the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChromeAction {
    /// Ask the client to close.
    Close,
    /// Minimise.
    Minimise,
    /// Fill the usable area, or go back from it.
    Maximise(Toggle),
    /// Start an interactive move.
    Move,
    /// Start an interactive resize.
    Resize(ResizeEdges),
}

/// What pressing `hit` asks of a window in `state`, as the `clicks` count and the person's
/// `double_click` setting say. None for a press that is not on the frame.
pub fn chrome_action(
    hit: FrameHit,
    clicks: Clicks,
    double_click: TitleDoubleClick,
    state: &WindowState,
) -> Option<ChromeAction> {
    let zoom = || {
        ChromeAction::Maximise(match state {
            WindowState::Filled { .. } => Toggle::Off,
            _ => Toggle::On,
        })
    };
    match (hit, clicks) {
        (FrameHit::Light(Light::Close), _) => Some(ChromeAction::Close),
        (FrameHit::Light(Light::Minimise), _) => Some(ChromeAction::Minimise),
        (FrameHit::Light(Light::Zoom), _) => Some(zoom()),
        (FrameHit::Edge(edges), _) => Some(ChromeAction::Resize(edges)),
        (FrameHit::Titlebar, Clicks::Single) => Some(ChromeAction::Move),
        (FrameHit::Titlebar, Clicks::Double) => Some(match double_click {
            TitleDoubleClick::Zoom | TitleDoubleClick::Fill => zoom(),
            TitleDoubleClick::Minimise => ChromeAction::Minimise,
            _ => ChromeAction::Move,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{LogicalRect, LogicalSize};

    fn ms(n: u64) -> Mono {
        Mono(Duration::from_millis(n))
    }

    fn at(x: i32, y: i32) -> LogicalPoint {
        LogicalPoint { x, y }
    }

    #[test]
    fn a_second_press_close_behind_the_first_is_a_double_click() {
        const CASES: &[(&str, u64, (i32, i32), u32, Clicks)] = &[
            ("quick and still", 200, (10, 10), 1, Clicks::Double),
            ("at the limit", 400, (14, 6), 1, Clicks::Double),
            ("too late", 401, (10, 10), 1, Clicks::Single),
            ("too far", 100, (15, 10), 1, Clicks::Single),
            ("another window", 100, (10, 10), 2, Clicks::Single),
        ];
        for (name, after, (x, y), window, want) in CASES {
            let (tracker, _) = ClickTracker::default().press(WindowId(1), ms(1000), at(10, 10));
            let (_, got) = tracker.press(WindowId(*window), ms(1000 + after), at(*x, *y));
            assert_eq!(got, *want, "{name}");
        }
    }

    #[test]
    fn a_double_click_uses_the_pair_up() {
        let (t, _) = ClickTracker::default().press(WindowId(1), ms(0), at(0, 0));
        let (t, second) = t.press(WindowId(1), ms(50), at(0, 0));
        let (_, third) = t.press(WindowId(1), ms(100), at(0, 0));
        assert_eq!((second, third), (Clicks::Double, Clicks::Single));
    }

    #[test]
    fn a_press_on_the_frame_asks_for_its_action() {
        let rect = LogicalRect::new(at(0, 0), LogicalSize { w: 1, h: 1 });
        let floating = WindowState::Floating(rect);
        let filled = WindowState::Filled { restore: rect };
        use ChromeAction as A;
        use TitleDoubleClick as D;
        let cases: &[(&str, FrameHit, Clicks, D, &WindowState, Option<A>)] = &[
            (
                "close",
                FrameHit::Light(Light::Close),
                Clicks::Single,
                D::Zoom,
                &floating,
                Some(A::Close),
            ),
            (
                "minimise",
                FrameHit::Light(Light::Minimise),
                Clicks::Single,
                D::Zoom,
                &floating,
                Some(A::Minimise),
            ),
            (
                "zoom in",
                FrameHit::Light(Light::Zoom),
                Clicks::Single,
                D::Zoom,
                &floating,
                Some(A::Maximise(Toggle::On)),
            ),
            (
                "zoom back",
                FrameHit::Light(Light::Zoom),
                Clicks::Single,
                D::Zoom,
                &filled,
                Some(A::Maximise(Toggle::Off)),
            ),
            (
                "drag",
                FrameHit::Titlebar,
                Clicks::Single,
                D::Zoom,
                &floating,
                Some(A::Move),
            ),
            (
                "double click zooms",
                FrameHit::Titlebar,
                Clicks::Double,
                D::Zoom,
                &floating,
                Some(A::Maximise(Toggle::On)),
            ),
            (
                "double click restores",
                FrameHit::Titlebar,
                Clicks::Double,
                D::Zoom,
                &filled,
                Some(A::Maximise(Toggle::Off)),
            ),
            (
                "double click minimises",
                FrameHit::Titlebar,
                Clicks::Double,
                D::Minimise,
                &floating,
                Some(A::Minimise),
            ),
            (
                "double click does nothing",
                FrameHit::Titlebar,
                Clicks::Double,
                D::Nothing,
                &floating,
                Some(A::Move),
            ),
            (
                "edge",
                FrameHit::Edge(ResizeEdges::Left),
                Clicks::Single,
                D::Zoom,
                &floating,
                Some(A::Resize(ResizeEdges::Left)),
            ),
            (
                "content",
                FrameHit::Content,
                Clicks::Single,
                D::Zoom,
                &floating,
                None,
            ),
            (
                "outside",
                FrameHit::Outside,
                Clicks::Single,
                D::Zoom,
                &floating,
                None,
            ),
        ];
        for (name, hit, clicks, double, state, want) in cases {
            assert_eq!(
                chrome_action(*hit, *clicks, *double, state),
                *want,
                "{name}"
            );
        }
    }
}
