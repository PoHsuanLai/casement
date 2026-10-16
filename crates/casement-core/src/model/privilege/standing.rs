//! What the compositor knows of the program on the other end of a public connection, as
//! casement-core's own vocabulary. The composition root fills it from the caller-identity
//! library when the connection is accepted; the model never reads a process.

use super::limits::{MAX_ROLE_BYTES, MAX_SUBJECT_BYTES};
use super::subject::{Engine, ProgramPath, SandboxAppId, Subject};
use crate::error::CoreError;
use casement_protocols::vocab::SubjectKind;

/// The role the desktop's table gives one of its own services (`shell`, `portal`): 1 to 64
/// lowercase letters, digits or `_`. The model only compares it, through
/// [`trusted_powers`](crate::policy::trusted_powers).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ServiceRole(String);

impl ServiceRole {
    /// The role written as `text`.
    pub fn parse(text: &str) -> Result<ServiceRole, CoreError> {
        let well_formed = !text.is_empty()
            && text.len() <= MAX_ROLE_BYTES
            && text
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if well_formed {
            Ok(ServiceRole(text.to_owned()))
        } else {
            Err(CoreError::Malformed {
                what: "service role",
                text: text.chars().take(64).collect(),
            })
        }
    }

    /// The role's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How well the compositor knows a public connection's program.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClientStanding {
    /// One of the desktop's own services, proven by the administrator's table (the portal).
    /// `exe` is its image, when known, so a service that is trusted for nothing is asked like
    /// any program.
    Service {
        /// The role the table gives it.
        role: ServiceRole,
        /// Its image.
        exe: Option<ProgramPath>,
    },
    /// A program whose image the system owns (`/usr/bin/grim`).
    Program {
        /// Its image.
        exe: ProgramPath,
    },
    /// A sandboxed app, vouched for by its engine (never by a name it chose).
    Sandboxed {
        /// Its id.
        app: SandboxAppId,
        /// The engine that vouches for it.
        engine: Engine,
    },
    /// A program nothing can be remembered about: an image the person can write, a hidden
    /// image, a peer that is gone, one that changed its image after connecting.
    Unverified {
        /// The image's file name, when it could be read, for the question's text.
        exe_name: Option<String>,
    },
}

/// Whom a decision is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Identity {
    /// A program that can be named and so remembered (unless the floor says otherwise).
    Named(Subject),
    /// A program that cannot: it gets a question valid for its connection only.
    Unnamed,
}

/// The subject a question carries to the shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskSubject {
    /// How well the program is known.
    pub kind: SubjectKind,
    /// Its path or app id, or its file name, or nothing.
    pub text: String,
}

impl ClientStanding {
    /// A connection nobody has vouched for yet: what a client is until the root says more.
    pub fn unproven() -> ClientStanding {
        ClientStanding::Unverified { exe_name: None }
    }

    /// Whom decisions about this connection are about.
    pub fn identity(&self) -> Identity {
        match self {
            ClientStanding::Service { exe: Some(exe), .. } | ClientStanding::Program { exe } => {
                Identity::Named(Subject::Program(exe.clone()))
            }
            ClientStanding::Sandboxed { app, engine } => Identity::Named(Subject::Sandboxed {
                app: app.clone(),
                engine: *engine,
            }),
            ClientStanding::Service { exe: None, .. } | ClientStanding::Unverified { .. } => {
                Identity::Unnamed
            }
        }
    }

    /// The program or app this connection is, as text: the image path of a program (or of a
    /// service whose image is known), the id of a sandboxed app. Nothing for a connection that
    /// cannot be named (an unverified program, a service whose image is not known).
    pub fn subject_text(&self) -> Option<&str> {
        match self {
            ClientStanding::Service { exe: Some(exe), .. } | ClientStanding::Program { exe } => {
                Some(exe.as_str())
            }
            ClientStanding::Sandboxed { app, .. } => Some(app.as_str()),
            ClientStanding::Service { exe: None, .. } | ClientStanding::Unverified { .. } => None,
        }
    }

    /// The desktop service role of this connection, if it is one.
    pub fn service_role(&self) -> Option<&ServiceRole> {
        match self {
            ClientStanding::Service { role, .. } => Some(role),
            _ => None,
        }
    }

    /// What the question says about this program.
    pub fn ask_subject(&self) -> AskSubject {
        match self.identity() {
            Identity::Named(subject) => AskSubject {
                kind: subject.kind(),
                text: subject.text().to_owned(),
            },
            Identity::Unnamed => AskSubject {
                kind: SubjectKind::Unrecognised,
                text: self.unnamed_text(),
            },
        }
    }

    /// The file name of a program that cannot be named, cleaned for the wire.
    fn unnamed_text(&self) -> String {
        let name = match self {
            ClientStanding::Unverified {
                exe_name: Some(name),
            } => name.as_str(),
            _ => "",
        };
        let clean: String = name.chars().filter(|c| !c.is_control()).collect();
        clean
            .char_indices()
            .map(|(at, c)| at + c.len_utf8())
            .take_while(|end| *end <= MAX_SUBJECT_BYTES)
            .last()
            .map_or_else(String::new, |end| clean[..end].to_owned())
    }
}
