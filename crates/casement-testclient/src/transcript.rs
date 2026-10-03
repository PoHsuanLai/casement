//! What a client received, line by line, on virtual time: the protocol tests' golden form
//! (`tests/golden/transcripts/<protocol>-*.txt`).

use casement_core::time::Mono;

/// Every event a client received, in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Transcript(pub Vec<TranscriptLine>);

/// One received event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptLine {
    /// When, on the fixture's clock.
    pub at: Mono,
    /// The object, as `interface#id`.
    pub object: String,
    /// The event's name.
    pub event: String,
    /// Its arguments, as text.
    pub args: Vec<String>,
}

impl Transcript {
    /// The transcript as golden text: one line per event, `<ms>.<µs> <object>.<event>(<args>)`.
    pub fn text(&self) -> String {
        self.0.iter().map(TranscriptLine::text).collect()
    }
}

impl TranscriptLine {
    fn text(&self) -> String {
        let micros = self.at.0.as_micros();
        format!(
            "{}.{:03} {}.{}({})\n",
            micros / 1000,
            micros % 1000,
            self.object,
            self.event,
            self.args.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_transcript_reads_one_line_per_event_on_virtual_time() {
        let line = |ms: u64, object: &str, event: &str, args: &[&str]| TranscriptLine {
            at: Mono(Duration::from_millis(ms)),
            object: object.to_owned(),
            event: event.to_owned(),
            args: args.iter().map(|a| (*a).to_owned()).collect(),
        };
        let transcript = Transcript(vec![
            line(0, "xdg_toplevel#12", "configure", &["0", "0", "[]"]),
            line(16, "wl_callback#20", "done", &["16"]),
        ]);
        assert_eq!(
            transcript.text(),
            "0.000 xdg_toplevel#12.configure(0, 0, [])\n16.000 wl_callback#20.done(16)\n"
        );
    }
}
