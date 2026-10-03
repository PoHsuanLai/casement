//! casementctl's subcommands.

use std::path::PathBuf;
use std::process::ExitCode;

/// A subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `debug VERB ARGS...`: a call on `org.quire.Casement.Debug1`.
    Debug {
        /// The verb (`inject`, `wait`, `clock`, `output`, `windows`, `scene`, `fault`).
        verb: DebugVerb,
        /// Its arguments.
        args: Vec<String>,
    },
    /// `shot [TARGET] [--masked] OUT.png` (P1-C1).
    Shot {
        /// What to capture.
        target: ShotTarget,
        /// Whether sensitive regions are masked.
        masked: Masked,
        /// Where the PNG goes.
        out: PathBuf,
    },
    /// `report --kms --input --gpu` (P5; the user's hardware report).
    Report,
}

/// A Debug1 verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugVerb {
    /// `inject`.
    Inject,
    /// `wait`.
    Wait,
    /// `clock`.
    Clock,
    /// `output`.
    Output,
    /// `windows`.
    Windows,
    /// `scene`.
    Scene,
    /// `fault`.
    Fault,
}

/// What `shot` captures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotTarget {
    /// The first output.
    Default,
    /// `--output NAME`.
    Output(String),
    /// `--window ID`.
    Window(u64),
    /// `--region X,Y,W,H` (global logical).
    Region {
        /// Left.
        x: i32,
        /// Top.
        y: i32,
        /// Width.
        w: u32,
        /// Height.
        h: u32,
    },
}

/// Whether a shot masks sensitive regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Masked {
    /// `--masked`.
    Yes,
    /// Not masked.
    No,
}

/// Why a command line was refused.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// Not a subcommand casementctl knows.
    #[error("unknown command: {0}")]
    Unknown(String),
    /// A malformed argument.
    #[error("bad argument: {0}")]
    BadArgument(String),
}

/// What `args` (without the program name) ask for.
pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let _ = args;
    todo!("P0-T-dev: casementctl parse")
}

/// Runs `command` against the compositor on `$DBUS_SESSION_BUS_ADDRESS` (the jail's private
/// bus) and `$WAYLAND_DISPLAY`: exit 0 on success, 1 on a failed wait, 2 outside the jail.
pub fn run(command: Command) -> ExitCode {
    let _ = command;
    todo!("P0-T-dev: casementctl run")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    #[ignore = "P0-T-dev"]
    fn the_subcommand_table() {
        assert_eq!(
            parse(&args("debug clock step 3")).expect("clock"),
            Command::Debug {
                verb: DebugVerb::Clock,
                args: args("step 3")
            }
        );
        assert_eq!(
            parse(&args("debug windows")).expect("windows"),
            Command::Debug {
                verb: DebugVerb::Windows,
                args: Vec::new()
            }
        );
        assert_eq!(
            parse(&args("shot --region 0,0,640,480 --masked out.png")).expect("shot"),
            Command::Shot {
                target: ShotTarget::Region {
                    x: 0,
                    y: 0,
                    w: 640,
                    h: 480
                },
                masked: Masked::Yes,
                out: PathBuf::from("out.png"),
            }
        );
        assert!(matches!(
            parse(&args("debug frobnicate")),
            Err(CommandError::Unknown(_))
        ));
        assert!(matches!(
            parse(&args("shot --window x out.png")),
            Err(CommandError::BadArgument(_))
        ));
    }
}
