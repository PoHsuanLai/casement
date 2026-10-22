//! The reveal machine of one output and edge: hidden, waiting for the dwell, sliding in, shown,
//! sliding away.

use super::{Place, RevealTiming};
use crate::time::Mono;
use casement_protocols::vocab::Shown;
use ds_core::vocab::Fraction;
use ds_style::tokens::easing::EasingToken;
use std::time::Duration;

/// Whether the output shows a full-screen Space (with a surface to reveal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cover {
    /// It does not: nothing reveals.
    #[default]
    No,
    /// It does.
    Yes,
}

/// Whether the surface has a pop-up open (an open bar menu keeps the bar up).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Popup {
    /// None open.
    #[default]
    Closed,
    /// One is open.
    Open,
}

/// Where the reveal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    /// Behind the full-screen window.
    #[default]
    Hidden,
    /// The pointer came into the strip at `since`; it has not left.
    Pending { since: Mono },
    /// Coming in since `since`, from `from` thousandths in.
    Showing { since: Mono, from: Fraction },
    /// Wholly in.
    Shown,
    /// Going away since `since`, from `from` thousandths in; it drops behind the window when
    /// it is gone.
    Hiding { since: Mono, from: Fraction },
}

/// What happened. Every input is stepped at an instant, which is also how a timer's firing
/// reaches the machine ([`RevealIn::Tick`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealIn {
    /// The output now shows (or no longer shows) a full-screen Space with a surface to reveal.
    Cover(Cover),
    /// The pointer is now here, relative to the edge.
    Pointer(Place),
    /// A pop-up of the surface opened or closed.
    Popup(Popup),
    /// The timer the machine asked for fired.
    Tick,
}

/// What the edge does about a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealOut {
    /// Arm the machine's timer for this instant (replacing an earlier one).
    Arm(Mono),
    /// Cancel the machine's timer.
    Cancel,
    /// Tell the shell the edge is shown or hidden (`reveal`).
    Tell(Shown),
}

/// Whether the surface is drawn and hit above the full-screen window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stacking {
    /// Behind it, where a bar or a dock normally is in full screen.
    Behind,
    /// Above it: sliding in, shown, or sliding away.
    Above,
}

/// One output's reveal of one edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reveal {
    phase: Phase,
    cover: Cover,
    place: Place,
    popup: Popup,
}

impl Reveal {
    /// Behind the window, in no full screen.
    pub fn new() -> Reveal {
        Reveal::default()
    }

    /// Whether the full-screen Space has been told to this machine.
    pub fn cover(&self) -> Cover {
        self.cover
    }

    /// Where the pointer has been told to be.
    pub fn place(&self) -> Place {
        self.place
    }

    /// Whether a pop-up has been told to this machine.
    pub fn popup(&self) -> Popup {
        self.popup
    }

    /// Whether the surface is above the full-screen window now.
    pub fn stacking(&self) -> Stacking {
        match self.phase {
            Phase::Hidden | Phase::Pending { .. } => Stacking::Behind,
            Phase::Showing { .. } | Phase::Shown | Phase::Hiding { .. } => Stacking::Above,
        }
    }

    /// How far in the surface is at `now`, in thousandths: the slide along the timing table's
    /// easing (out when it comes, exit when it goes), or the fade's opacity.
    pub fn visible(&self, timing: RevealTiming, now: Mono) -> Fraction {
        match self.phase {
            Phase::Hidden | Phase::Pending { .. } => Fraction(0),
            Phase::Shown => Fraction(1000),
            Phase::Showing { since, from } => {
                let eased = EasingToken::Out
                    .easing()
                    .at(through(now, since, timing.travel));
                let rest = u32::from(1000 - from.0);
                Fraction(from.0 + share(rest, eased))
            }
            Phase::Hiding { since, from } => {
                let eased = EasingToken::Exit
                    .easing()
                    .at(through(now, since, timing.travel));
                Fraction(share(u32::from(from.0), Fraction(1000 - eased.clamped().0)))
            }
        }
    }

    /// The machine after `input` at `now`, and what the edge does about it.
    pub fn step(
        self,
        timing: RevealTiming,
        input: RevealIn,
        now: Mono,
    ) -> (Reveal, Vec<RevealOut>) {
        let told = match input {
            RevealIn::Cover(cover) => Reveal { cover, ..self },
            RevealIn::Pointer(place) => Reveal { place, ..self },
            RevealIn::Popup(popup) => Reveal { popup, ..self },
            RevealIn::Tick => self,
        };
        told.settled(timing, now)
    }

    /// Whether the pointer or a pop-up keeps the surface up.
    fn held(&self) -> bool {
        self.place != Place::Away || self.popup == Popup::Open
    }

    /// The machine moved to `phase`.
    fn at(self, phase: Phase) -> Reveal {
        Reveal { phase, ..self }
    }

    /// Where the told facts put the machine at `now`.
    fn settled(self, timing: RevealTiming, now: Mono) -> (Reveal, Vec<RevealOut>) {
        match (self.cover, self.phase) {
            (Cover::No, Phase::Hidden) => (self, Vec::new()),
            (Cover::No, Phase::Pending { .. } | Phase::Hiding { .. }) => {
                (self.at(Phase::Hidden), vec![RevealOut::Cancel])
            }
            (Cover::No, Phase::Showing { .. } | Phase::Shown) => (
                self.at(Phase::Hidden),
                vec![RevealOut::Cancel, RevealOut::Tell(Shown::Hidden)],
            ),
            (Cover::Yes, Phase::Hidden) => match self.place {
                Place::InStrip => (
                    self.at(Phase::Pending { since: now }),
                    vec![RevealOut::Arm(now.after(timing.dwell))],
                ),
                Place::Away | Place::OnSurface => (self, Vec::new()),
            },
            (Cover::Yes, Phase::Pending { since }) => self.waiting(timing, since, now),
            (Cover::Yes, Phase::Showing { since, .. }) => match now.since(since) >= timing.travel {
                true => self.at(Phase::Shown).settled(timing, now),
                false => self.leaving(timing, now),
            },
            (Cover::Yes, Phase::Shown) => self.leaving(timing, now),
            (Cover::Yes, Phase::Hiding { since, .. }) => self.returning(timing, since, now),
        }
    }

    /// Resting in the strip since `since`: it comes when the rest is long enough, and the wait
    /// is over when the pointer leaves the strip.
    fn waiting(self, timing: RevealTiming, since: Mono, now: Mono) -> (Reveal, Vec<RevealOut>) {
        match (self.place, now.since(since) >= timing.dwell) {
            (Place::InStrip, true) => (
                self.at(Phase::Showing {
                    since: now,
                    from: Fraction(0),
                }),
                vec![
                    RevealOut::Tell(Shown::Shown),
                    RevealOut::Arm(now.after(timing.travel)),
                ],
            ),
            (Place::InStrip, false) => (self, Vec::new()),
            (Place::Away | Place::OnSurface, _) => {
                (self.at(Phase::Hidden), vec![RevealOut::Cancel])
            }
        }
    }

    /// In (or coming in): it goes when nothing holds it.
    fn leaving(self, timing: RevealTiming, now: Mono) -> (Reveal, Vec<RevealOut>) {
        match self.held() {
            true => (self, Vec::new()),
            false => (
                self.at(Phase::Hiding {
                    since: now,
                    from: self.visible(timing, now),
                }),
                vec![
                    RevealOut::Tell(Shown::Hidden),
                    RevealOut::Arm(now.after(timing.travel)),
                ],
            ),
        }
    }

    /// Going away since `since`: the pointer coming back turns it round from where it is; when
    /// it is gone it drops behind the window.
    fn returning(self, timing: RevealTiming, since: Mono, now: Mono) -> (Reveal, Vec<RevealOut>) {
        match (self.held(), now.since(since) >= timing.travel) {
            (true, _) => (
                self.at(Phase::Showing {
                    since: now,
                    from: self.visible(timing, now),
                }),
                vec![
                    RevealOut::Tell(Shown::Shown),
                    RevealOut::Arm(now.after(timing.travel)),
                ],
            ),
            (false, true) => (self.at(Phase::Hidden), Vec::new()),
            (false, false) => (self, Vec::new()),
        }
    }
}

/// How far `now` is through a move of `length` begun at `since`, in thousandths.
fn through(now: Mono, since: Mono, length: Duration) -> Fraction {
    match length.is_zero() {
        true => Fraction(1000),
        false => {
            let part = now.since(since).as_micros() * 1000 / length.as_micros();
            Fraction(u16::try_from(part.min(1000)).unwrap_or(1000))
        }
    }
}

/// `part` thousandths of `whole`.
fn share(whole: u32, part: Fraction) -> u16 {
    u16::try_from(whole * u32::from(part.clamped().0) / 1000).unwrap_or(1000)
}
