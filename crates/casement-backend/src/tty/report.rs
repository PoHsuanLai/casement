//! The KMS report (P1-V2): what `casement --kms-report FILE` appends once a second. A pure
//! value built from the backend's state ([`TtyBackend::report`]), written as one JSON line;
//! `dev/vkms/accept.sh` reads it with `jq`.
//!
//! ```text
//! {"flips":600,"missed":0,"seat":"held","outputs":[{"name":"Virtual-1","mode":"1024x768@60000","power":"on"}]}
//! ```
//!
//! `flips` and `missed` count from the start of the run, summed over outputs (an output that
//! went away keeps what it counted). `mode` is `WIDTHxHEIGHT@MILLIHERTZ`.

use super::{SeatState, TtyBackend};
use casement_core::OutputName;
use casement_core::output::{Mode, OutputPower};
use casement_core::time::Mono;
use std::time::Duration;

/// How often the report is written.
pub const REPORT_EVERY: Duration = Duration::from_secs(1);

/// One lit output in the report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReportedOutput {
    /// Its connector's name.
    pub name: OutputName,
    /// The mode it is lit with.
    pub mode: Mode,
    /// Lit or dark.
    pub power: OutputPower,
}

impl ReportedOutput {
    /// An output of the report.
    pub fn new(name: OutputName, mode: Mode, power: OutputPower) -> ReportedOutput {
        ReportedOutput { name, mode, power }
    }
}

/// The backend's state, as the report says it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct KmsReport {
    /// Frames that reached the glass, on every output.
    pub flips: u64,
    /// Of those, frames that missed the vblank they were made for.
    pub missed: u64,
    /// Whether this session holds the seat.
    pub seat: SeatState,
    /// The outputs that are lit or darkened, in name order.
    pub outputs: Vec<ReportedOutput>,
}

impl KmsReport {
    /// A report.
    pub fn new(
        flips: u64,
        missed: u64,
        seat: SeatState,
        outputs: Vec<ReportedOutput>,
    ) -> KmsReport {
        KmsReport {
            flips,
            missed,
            seat,
            outputs,
        }
    }

    /// The report as one JSON object, without a trailing newline.
    ///
    /// ```
    /// use casement_backend::tty::{KmsReport, SeatState};
    ///
    /// let report = KmsReport::new(3, 1, SeatState::Lent, Vec::new());
    /// assert_eq!(
    ///     report.json_line(),
    ///     r#"{"flips":3,"missed":1,"seat":"lent","outputs":[]}"#
    /// );
    /// ```
    pub fn json_line(&self) -> String {
        let seat = match self.seat {
            SeatState::Held => "held",
            _ => "lent",
        };
        let outputs: Vec<String> = self.outputs.iter().map(output_json).collect();
        format!(
            r#"{{"flips":{},"missed":{},"seat":"{seat}","outputs":[{}]}}"#,
            self.flips,
            self.missed,
            outputs.join(",")
        )
    }
}

/// One output as a JSON object.
fn output_json(output: &ReportedOutput) -> String {
    let mode = format!(
        "{}x{}@{}",
        output.mode.size.w, output.mode.size.h, output.mode.refresh.0
    );
    let power = match output.power {
        OutputPower::On => "on",
        OutputPower::Off => "off",
    };
    format!(
        r#"{{"name":{},"mode":"{mode}","power":"{power}"}}"#,
        json_string(output.name.as_str())
    )
}

/// `text` as a JSON string literal.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Whether a report is due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Due {
    /// Write one now.
    Now,
    /// Not yet.
    Later,
}

/// Decides when the next report is due, from the session clock (time is an argument).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReportPacer {
    next: Option<Mono>,
}

impl ReportPacer {
    /// A pacer whose first report is due at once.
    pub fn new() -> ReportPacer {
        ReportPacer::default()
    }

    /// Whether a report is due at `now`; when it is, the next one is [`REPORT_EVERY`] later.
    ///
    /// ```
    /// use casement_backend::tty::{Due, ReportPacer};
    /// use casement_core::time::Mono;
    /// use std::time::Duration;
    ///
    /// let at = |ms| Mono(Duration::from_millis(ms));
    /// let mut pacer = ReportPacer::new();
    /// assert_eq!(pacer.check(at(0)), Due::Now);
    /// assert_eq!(pacer.check(at(999)), Due::Later);
    /// assert_eq!(pacer.check(at(1000)), Due::Now);
    /// ```
    pub fn check(&mut self, now: Mono) -> Due {
        match self.next {
            Some(next) if now < next => Due::Later,
            _ => {
                self.next = Some(now.after(REPORT_EVERY));
                Due::Now
            }
        }
    }
}

impl TtyBackend {
    /// The backend's state as a report.
    pub fn report(&self) -> KmsReport {
        let total = self.scanouts.total();
        let outputs = self
            .lit
            .iter()
            .map(|(name, lit)| ReportedOutput::new(name.clone(), lit.config.mode, lit.power()))
            .collect();
        KmsReport::new(total.presented, total.missed, self.seat, outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::geometry::PhysicalSize;
    use casement_core::output::ModePreference;
    use casement_core::time::Millihertz;

    fn mode() -> Mode {
        Mode {
            size: PhysicalSize { w: 1024, h: 768 },
            refresh: Millihertz(60_000),
            preference: ModePreference::Preferred,
        }
    }

    fn output(name: &str, power: OutputPower) -> ReportedOutput {
        ReportedOutput::new(OutputName::parse(name).expect("a name"), mode(), power)
    }

    #[test]
    fn a_report_is_one_json_line_with_the_script_s_fields() {
        let report = KmsReport::new(
            600,
            0,
            SeatState::Held,
            vec![
                output("Virtual-1", OutputPower::On),
                output("Virtual-2", OutputPower::Off),
            ],
        );
        assert_eq!(
            report.json_line(),
            concat!(
                r#"{"flips":600,"missed":0,"seat":"held","outputs":["#,
                r#"{"name":"Virtual-1","mode":"1024x768@60000","power":"on"},"#,
                r#"{"name":"Virtual-2","mode":"1024x768@60000","power":"off"}]}"#
            )
        );
    }

    #[test]
    fn a_lent_seat_with_no_output_is_still_a_report() {
        let report = KmsReport::new(7, 2, SeatState::Lent, Vec::new());
        assert_eq!(
            report.json_line(),
            r#"{"flips":7,"missed":2,"seat":"lent","outputs":[]}"#
        );
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(json_string("a\"b\\c\n"), r#""a\"b\\c\u000a""#);
    }

    #[test]
    fn the_pacer_is_due_at_once_then_every_second() {
        let at = |ms| Mono(Duration::from_millis(ms));
        let mut pacer = ReportPacer::new();
        assert_eq!(pacer.check(at(5_000)), Due::Now);
        assert_eq!(pacer.check(at(5_500)), Due::Later);
        assert_eq!(pacer.check(at(6_000)), Due::Now);
        assert_eq!(pacer.check(at(6_001)), Due::Later);
    }
}
