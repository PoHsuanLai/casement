//! A subject as the stored files write it: `{ program = "/usr/bin/grim" }` or
//! `{ app = "org.gnome.Loupe", sandbox = "flatpak" }`.

use super::subject::{Engine, ProgramPath, SandboxAppId, Subject};
use serde::{Deserialize, Serialize};

/// One subject, either form.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct SubjectDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    program: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sandbox: Option<Engine>,
}

impl SubjectDto {
    /// How `subject` is written.
    pub(super) fn of(subject: &Subject) -> SubjectDto {
        match subject {
            Subject::Program(path) => SubjectDto {
                program: Some(path.as_str().to_owned()),
                ..SubjectDto::default()
            },
            Subject::Sandboxed { app, engine } => SubjectDto {
                app: Some(app.as_str().to_owned()),
                sandbox: Some(*engine),
                ..SubjectDto::default()
            },
        }
    }

    /// The subject written, or nothing when the row names neither form well.
    pub(super) fn subject(&self) -> Option<Subject> {
        match (&self.program, &self.app, self.sandbox) {
            (Some(program), None, None) => ProgramPath::parse(program).ok().map(Subject::Program),
            (None, Some(app), Some(engine)) => SandboxAppId::parse(app)
                .ok()
                .map(|app| Subject::Sandboxed { app, engine }),
            _ => None,
        }
    }
}
