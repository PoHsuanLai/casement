//! Who a grant is about: a program by its image, or a sandboxed app by its id. A grant binds
//! to a subject, never to a display name or a window's claimed app id.

use super::limits::{MAX_APP_BYTES, MAX_SUBJECT_BYTES};
use crate::error::CoreError;
use casement_protocols::vocab::SubjectKind;
use serde::{Deserialize, Serialize};

/// The path of a program's image as the kernel reports it: absolute, with no empty, `.` or
/// `..` part, so one program has one spelling and a stored row cannot dodge a match.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProgramPath(String);

impl ProgramPath {
    /// The path written as `text`.
    pub fn parse(text: &str) -> Result<ProgramPath, CoreError> {
        let well_formed = text.starts_with('/')
            && text.len() <= MAX_SUBJECT_BYTES
            && !text.chars().any(char::is_control)
            && !text.ends_with(" (deleted)")
            && text
                .split('/')
                .skip(1)
                .all(|part| !part.is_empty() && part != "." && part != "..");
        if well_formed {
            Ok(ProgramPath(text.to_owned()))
        } else {
            Err(CoreError::Malformed {
                what: "program path",
                text: text.chars().take(64).collect(),
            })
        }
    }

    /// The path's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The last part of the path: the program's file name.
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or_default()
    }
}

impl TryFrom<String> for ProgramPath {
    type Error = CoreError;
    fn try_from(text: String) -> Result<ProgramPath, CoreError> {
        ProgramPath::parse(&text)
    }
}

impl From<ProgramPath> for String {
    fn from(path: ProgramPath) -> String {
        path.0
    }
}

/// A sandboxed app's id as its sandbox reports it (`org.gnome.Loupe`): 1 to 255 ASCII
/// letters, digits, `.`, `_` or `-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SandboxAppId(String);

impl SandboxAppId {
    /// The id written as `text`.
    pub fn parse(text: &str) -> Result<SandboxAppId, CoreError> {
        let well_formed = !text.is_empty()
            && text.len() <= MAX_APP_BYTES
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if well_formed {
            Ok(SandboxAppId(text.to_owned()))
        } else {
            Err(CoreError::Malformed {
                what: "sandboxed app id",
                text: text.chars().take(64).collect(),
            })
        }
    }

    /// The id's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SandboxAppId {
    type Error = CoreError;
    fn try_from(text: String) -> Result<SandboxAppId, CoreError> {
        SandboxAppId::parse(&text)
    }
}

impl From<SandboxAppId> for String {
    fn from(id: SandboxAppId) -> String {
        id.0
    }
}

/// The sandbox engine that vouches for an app id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Engine {
    /// Flatpak.
    Flatpak,
    /// Snap.
    Snap,
}

/// What a grant is about.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Subject {
    /// A program whose image the system owns, by its path.
    Program(ProgramPath),
    /// A sandboxed app, by the id its engine vouches for.
    Sandboxed {
        /// The app id.
        app: SandboxAppId,
        /// The engine that vouches for it.
        engine: Engine,
    },
}

impl Subject {
    /// How the question names this subject on the wire.
    pub fn kind(&self) -> SubjectKind {
        match self {
            Subject::Program(_) => SubjectKind::Program,
            Subject::Sandboxed { .. } => SubjectKind::SandboxedApp,
        }
    }

    /// The text the question carries: the path, or the app id.
    pub fn text(&self) -> &str {
        match self {
            Subject::Program(path) => path.as_str(),
            Subject::Sandboxed { app, .. } => app.as_str(),
        }
    }
}
