//! What caused a change of key window, judged from what the loop has just heard.

use super::key_window::KeyCause;
use super::stamp::Stamp;
use casement_core::input::{Edge, InputEvent, InputKind};
use casement_core::{Event, ToplevelRequest};
use std::time::Duration;

/// How soon after a press the compositor's own reaction to it still counts as caused by it. The
/// reaction is in the same loop turn as the event, so this is only a guard against a stale press.
pub const DIRECT_WINDOW: Duration = Duration::from_millis(250);

/// How soon after a press a request that changes the key window still counts as caused by it: a
/// click on the dock or a chord the shell took goes to the shell, which asks the compositor a
/// round trip later (and a switcher that commits on release waits for the person).
pub const REQUEST_WINDOW: Duration = Duration::from_secs(2);

/// What kind of press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressKind {
    /// A key went down.
    Key,
    /// A pointer button went down.
    Button,
}

/// The last key or button press the loop heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Press {
    /// Which.
    pub kind: PressKind,
    /// When the event happened.
    pub at: Stamp,
}

/// What the loop turn that is running heard, as far as the key window is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedKind {
    /// Input, timers and the like: no sign of who asked.
    #[default]
    Plain,
    /// A window or client came or went: the change is a consequence, not a choice.
    Structural,
    /// Somebody asked: an activation, the shell, a window list client, a workspace request.
    Request,
}

impl FeedKind {
    /// The kind of a batch of events. A window or client arriving or leaving outranks a request
    /// in the same batch.
    pub fn of(events: &[Event]) -> FeedKind {
        events
            .iter()
            .map(FeedKind::of_event)
            .fold(FeedKind::Plain, FeedKind::stronger)
    }

    fn of_event(event: &Event) -> FeedKind {
        match event {
            Event::ToplevelMapped(_)
            | Event::ToplevelUnmapped { .. }
            | Event::ClientGone { .. } => FeedKind::Structural,
            Event::ToplevelRequest {
                request: ToplevelRequest::Activate(_),
                ..
            }
            | Event::Desktop(_)
            | Event::Shell(_)
            | Event::WlrToplevel { .. }
            | Event::Workspace(_) => FeedKind::Request,
            _ => FeedKind::Plain,
        }
    }

    fn stronger(self, other: FeedKind) -> FeedKind {
        match (self, other) {
            (FeedKind::Structural, _) | (_, FeedKind::Structural) => FeedKind::Structural,
            (FeedKind::Request, _) | (_, FeedKind::Request) => FeedKind::Request,
            (FeedKind::Plain, FeedKind::Plain) => FeedKind::Plain,
        }
    }
}

/// The last press in `events`, by `stamp_of` the session instant of an event.
pub fn press_in(events: &[Event], stamp_of: impl Fn(&InputEvent) -> Stamp) -> Option<Press> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Input(input) => match input.kind {
                InputKind::Key {
                    edge: Edge::Press, ..
                } => Some(Press {
                    kind: PressKind::Key,
                    at: stamp_of(input),
                }),
                InputKind::Button {
                    edge: Edge::Press, ..
                } => Some(Press {
                    kind: PressKind::Button,
                    at: stamp_of(input),
                }),
                _ => None,
            },
            _ => None,
        })
        .next_back()
}

/// The cause of a change of key window made at `now` in a turn of `feed`, and the stamp of the
/// input event behind it ([`Stamp::NONE`] when there is none): a window or client coming or going
/// is `other`; a request is `request`, with the press that probably started it when one came
/// within [`REQUEST_WINDOW`]; anything else is the press that is within [`DIRECT_WINDOW`] (a
/// chord is `keyboard`, a click `pointer`), else `other`.
pub fn cause_of(feed: FeedKind, press: Option<Press>, now: Stamp) -> (KeyCause, Stamp) {
    let within = |window: Duration| press.filter(|press| now.within(press.at, window));
    match (feed, within(DIRECT_WINDOW), within(REQUEST_WINDOW)) {
        (FeedKind::Structural, _, _) => (KeyCause::Other, Stamp::NONE),
        (FeedKind::Request, _, press) => (KeyCause::Request, press.map_or(Stamp::NONE, |p| p.at)),
        (
            FeedKind::Plain,
            Some(Press {
                kind: PressKind::Key,
                at,
            }),
            _,
        ) => (KeyCause::Keyboard, at),
        (
            FeedKind::Plain,
            Some(Press {
                kind: PressKind::Button,
                at,
            }),
            _,
        ) => (KeyCause::Pointer, at),
        (FeedKind::Plain, None, _) => (KeyCause::Other, Stamp::NONE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: Press = Press {
        kind: PressKind::Key,
        at: Stamp(10_000_000_000),
    };
    const CLICK: Press = Press {
        kind: PressKind::Button,
        at: Stamp(10_000_000_000),
    };

    fn after(ms: u64) -> Stamp {
        Stamp(10_000_000_000 + ms * 1_000_000)
    }

    #[test]
    fn the_cause_follows_the_turn_and_how_recent_the_press_is() {
        // name, turn, press, now, cause, input_at
        let cases = [
            (
                "chord in the same turn",
                FeedKind::Plain,
                Some(KEY),
                after(1),
                KeyCause::Keyboard,
                KEY.at,
            ),
            (
                "click in the same turn",
                FeedKind::Plain,
                Some(CLICK),
                after(1),
                KeyCause::Pointer,
                CLICK.at,
            ),
            (
                "stale press, plain turn",
                FeedKind::Plain,
                Some(KEY),
                after(300),
                KeyCause::Other,
                Stamp::NONE,
            ),
            (
                "no press at all",
                FeedKind::Plain,
                None,
                after(1),
                KeyCause::Other,
                Stamp::NONE,
            ),
            (
                "shell asked right after a chord",
                FeedKind::Request,
                Some(KEY),
                after(40),
                KeyCause::Request,
                KEY.at,
            ),
            (
                "shell asked after a long hold",
                FeedKind::Request,
                Some(KEY),
                after(1_900),
                KeyCause::Request,
                KEY.at,
            ),
            (
                "request with a stale press",
                FeedKind::Request,
                Some(KEY),
                after(2_100),
                KeyCause::Request,
                Stamp::NONE,
            ),
            (
                "request with no press",
                FeedKind::Request,
                None,
                after(1),
                KeyCause::Request,
                Stamp::NONE,
            ),
            (
                "a window opened after a press",
                FeedKind::Structural,
                Some(KEY),
                after(1),
                KeyCause::Other,
                Stamp::NONE,
            ),
        ];
        for (name, feed, press, now, cause, input_at) in cases {
            assert_eq!(cause_of(feed, press, now), (cause, input_at), "{name}");
        }
    }

    #[test]
    fn a_structural_event_outranks_a_request_in_one_batch() {
        use FeedKind::{Plain, Request, Structural};
        // name, left, right, stronger
        let cases = [
            ("plain and plain", Plain, Plain, Plain),
            ("plain and request", Plain, Request, Request),
            ("request and structural", Request, Structural, Structural),
            ("structural and request", Structural, Request, Structural),
        ];
        for (name, left, right, want) in cases {
            assert_eq!(left.stronger(right), want, "{name}");
        }
    }
}
