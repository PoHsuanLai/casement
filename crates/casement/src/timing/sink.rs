//! Where the lines go: the journal, or (a test) a list.

/// The one effectful edge of the timing lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Sink {
    /// `tracing` at INFO, target `timing`: the subscriber `log::init` installs writes it to
    /// stderr (the session unit's log file or the journal). Always on, in release too.
    #[default]
    Journal,
    /// Kept in order for a test to read.
    Kept(Vec<String>),
}

impl Sink {
    /// Writes one line.
    pub fn write(&mut self, line: String) {
        match self {
            Sink::Journal => tracing::info!(target: "timing", "{line}"),
            Sink::Kept(lines) => lines.push(line),
        }
    }

    /// The lines kept so far; none for the journal.
    pub fn lines(&self) -> &[String] {
        match self {
            Sink::Journal => &[],
            Sink::Kept(lines) => lines,
        }
    }
}
