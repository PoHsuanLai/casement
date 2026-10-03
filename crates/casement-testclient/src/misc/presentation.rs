//! `wp_presentation` (P0-H1). What a feedback object was told is in the transcript;
//! [`feedback_events`] reads it.

use crate::conn::TestClient;
use crate::surface::subsurface::{H1Data, Surface, quiet, seen};
use std::time::Duration;
use wayland_protocols::wp::presentation_time::client::{
    wp_presentation::WpPresentation, wp_presentation_feedback::WpPresentationFeedback,
};

/// What one feedback object was told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackEvent {
    /// `sync_output` (the `wl_output` object's text).
    SyncOutput(String),
    /// `presented`: when, the refresh period in nanoseconds (0 when unknown), the sequence and
    /// the `kind` flags.
    Presented {
        /// The timestamp.
        at: Duration,
        /// The refresh period in nanoseconds.
        refresh: u32,
        /// The vblank sequence.
        seq: u64,
        /// The `kind` flags.
        flags: u32,
    },
    /// `discarded`.
    Discarded,
}

/// A bound `wp_presentation`.
#[derive(Debug)]
pub struct Presentation {
    /// The global.
    pub presentation: WpPresentation,
}

impl Presentation {
    /// Binds the global at exactly `version`.
    pub fn bind(client: &TestClient, version: u32) -> Presentation {
        Presentation {
            presentation: client
                .bind(version..=version, H1Data)
                .expect("the compositor advertises wp_presentation"),
        }
    }

    /// Asks for feedback on the content update of `surface`'s next commit.
    pub fn feedback(&self, client: &TestClient, surface: &Surface) -> WpPresentationFeedback {
        self.presentation
            .feedback(&surface.surface, &client.qh(), H1Data)
    }

    /// The clock id the compositor announced, once the client was pumped.
    pub fn clock(&self, client: &TestClient) -> Option<u32> {
        seen(client, &self.presentation)
            .into_iter()
            .find(|(event, _)| event == "clock_id")
            .and_then(|(_, args)| args.first()?.parse().ok())
    }
}

/// What `feedback` was told, in order.
pub fn feedback_events(
    client: &TestClient,
    feedback: &WpPresentationFeedback,
) -> Vec<FeedbackEvent> {
    let number = |text: &String| text.parse::<u64>().unwrap_or(0);
    seen(client, feedback)
        .into_iter()
        .filter_map(|(event, args)| match (event.as_str(), args.as_slice()) {
            ("sync_output", [output]) => Some(FeedbackEvent::SyncOutput(output.clone())),
            ("presented", [sec_hi, sec_lo, nsec, refresh, seq_hi, seq_lo, flags]) => {
                Some(FeedbackEvent::Presented {
                    at: Duration::new(
                        number(sec_hi) << 32 | number(sec_lo),
                        u32::try_from(number(nsec)).unwrap_or(0),
                    ),
                    refresh: u32::try_from(number(refresh)).unwrap_or(0),
                    seq: number(seq_hi) << 32 | number(seq_lo),
                    flags: u32::try_from(number(flags)).unwrap_or(0),
                })
            }
            ("discarded", _) => Some(FeedbackEvent::Discarded),
            _ => None,
        })
        .collect()
}

quiet!(WpPresentation, WpPresentationFeedback);
