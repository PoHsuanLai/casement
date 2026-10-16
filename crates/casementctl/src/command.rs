//! casementctl's subcommands.

mod bus;
mod capture;
mod debug;
mod jail;
mod plan;
mod privacy;
mod shot;

use std::path::PathBuf;
use std::process::ExitCode;

/// The exit code of a command that could not run safely or is not built yet (the plan's 2).
const COULD_NOT_RUN: u8 = 2;

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
    /// `privacy list | reload`: which program may use which power. It runs anywhere, not only
    /// in the jail: it is for the person at their own desktop.
    Privacy(privacy::Privacy),
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

impl DebugVerb {
    /// The verb `word` names.
    fn named(word: &str) -> Result<DebugVerb, CommandError> {
        [
            ("inject", DebugVerb::Inject),
            ("wait", DebugVerb::Wait),
            ("clock", DebugVerb::Clock),
            ("output", DebugVerb::Output),
            ("windows", DebugVerb::Windows),
            ("scene", DebugVerb::Scene),
            ("fault", DebugVerb::Fault),
        ]
        .into_iter()
        .find_map(|(name, verb)| (name == word).then_some(verb))
        .ok_or_else(|| CommandError::Unknown(format!("debug {word}")))
    }
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
#[non_exhaustive]
pub enum CommandError {
    /// Not a subcommand casementctl knows.
    #[error("unknown command: {0}")]
    Unknown(String),
    /// A malformed argument.
    #[error("bad argument: {0}")]
    BadArgument(String),
}

/// What `args` (without the program name) ask for. A `debug` command's arguments are checked
/// here, so a typo never reaches the bus.
pub fn parse(args: &[String]) -> Result<Command, CommandError> {
    let (name, rest) = args.split_first().ok_or_else(|| {
        CommandError::BadArgument("no command (debug, shot, report, privacy)".to_owned())
    })?;
    match name.as_str() {
        "debug" => {
            let (verb, rest) = rest
                .split_first()
                .ok_or_else(|| CommandError::BadArgument("debug needs a verb".to_owned()))?;
            debug_command(DebugVerb::named(verb)?, rest)
        }
        // The freeze plan writes output control without `debug` (`casementctl output list
        // --json`, `output set NAME --scale S`); the Debug1 table has it as a verb, so both
        // spellings are the same command.
        "output" => debug_command(DebugVerb::Output, rest),
        "shot" => {
            let (target, masked, out) = shot::parse_shot(rest)?;
            Ok(Command::Shot {
                target,
                masked,
                out,
            })
        }
        "privacy" => privacy::parse(rest).map(Command::Privacy),
        "report" => match rest
            .iter()
            .find(|flag| !matches!(flag.as_str(), "--kms" | "--input" | "--gpu"))
        {
            None => Ok(Command::Report),
            Some(flag) => Err(CommandError::BadArgument(format!(
                "report: unknown flag {flag}"
            ))),
        },
        other => Err(CommandError::Unknown(other.to_owned())),
    }
}

/// `debug VERB ARGS...`, its arguments checked.
fn debug_command(verb: DebugVerb, args: &[String]) -> Result<Command, CommandError> {
    plan::plan(verb, args)?;
    Ok(Command::Debug {
        verb,
        args: args.to_vec(),
    })
}

/// Runs `command` against the compositor on `$DBUS_SESSION_BUS_ADDRESS` (the jail's private
/// bus) and `$WAYLAND_DISPLAY`: exit 0 on success, 1 on a failed wait, 2 outside the jail.
pub fn run(command: Command) -> ExitCode {
    ExitCode::from(exit_code(command))
}

fn exit_code(command: Command) -> u8 {
    if let Command::Report = command {
        return not_built("report is the user's hardware report, built in P5");
    }
    if let Command::Privacy(asked) = command {
        return privacy::run(asked);
    }
    if let Some(breach) = jail::breach_here() {
        eprintln!(
            "casementctl: refusing to run outside the jail ({breach}); see ~/desktop/harness/jail.sh"
        );
        return COULD_NOT_RUN;
    }
    match command {
        Command::Debug { verb, args } => match plan::plan(verb, &args) {
            Ok(call) => debug::run_call(call),
            Err(error) => {
                eprintln!("casementctl: {error}");
                64
            }
        },
        Command::Shot {
            target,
            masked,
            out,
        } => {
            let cwd = std::env::current_dir().unwrap_or_default();
            match (&target, masked) {
                // A named output is captured over the wire, like any screenshot tool would.
                (ShotTarget::Output(name), Masked::No) => capture::run(name, &cwd.join(&out)),
                _ => match shot::call(&target, masked, &out, &cwd) {
                    Ok(call) => debug::run_call(call),
                    Err(why) => not_built(why),
                },
            }
        }
        Command::Report | Command::Privacy(_) => {
            not_built("report is the user's hardware report, built in P5")
        }
    }
}

fn not_built(why: &str) -> u8 {
    eprintln!("casementctl: {why}");
    COULD_NOT_RUN
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
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
            parse(&args("privacy reload")),
            Ok(Command::Privacy(_))
        ));
        assert!(matches!(
            parse(&args("debug frobnicate")),
            Err(CommandError::Unknown(_))
        ));
        assert!(matches!(
            parse(&args("shot --window x out.png")),
            Err(CommandError::BadArgument(_))
        ));
    }

    #[test]
    fn output_without_debug_is_the_same_command() {
        for text in [
            "output list --json",
            "output list",
            "output set HEADLESS-1 --scale 2",
            "output add 1920x1080",
            "output remove HEADLESS-2",
        ] {
            let (_, rest) = text.split_once(' ').expect("a verb");
            assert_eq!(
                parse(&args(text)).expect(text),
                parse(&args(&format!("debug output {rest}"))).expect(text),
                "{text}"
            );
        }
        assert!(matches!(
            parse(&args("output")),
            Err(CommandError::BadArgument(_))
        ));
        assert!(matches!(
            parse(&args("output frobnicate")),
            Err(CommandError::BadArgument(_))
        ));
    }

    #[test]
    fn report_takes_its_three_flags_and_nothing_else() {
        assert_eq!(
            parse(&args("report --kms --input --gpu")).expect("report"),
            Command::Report
        );
        assert_eq!(parse(&args("report")).expect("report"), Command::Report);
        assert!(matches!(
            parse(&args("report --net")),
            Err(CommandError::BadArgument(_))
        ));
    }

    #[test]
    fn a_debug_command_is_checked_before_it_is_run() {
        assert!(matches!(
            parse(&args("debug")),
            Err(CommandError::BadArgument(_))
        ));
        assert!(matches!(
            parse(&args("debug clock step")),
            Err(CommandError::BadArgument(_))
        ));
        assert!(matches!(
            parse(&args("debug wait")),
            Err(CommandError::BadArgument(_))
        ));
        assert!(matches!(
            parse(&args("")),
            Err(CommandError::BadArgument(_))
        ));
        assert!(matches!(
            parse(&args("launch")),
            Err(CommandError::Unknown(_))
        ));
        assert_eq!(
            parse(&args("debug wait mapped foot --timeout 500")).expect("wait"),
            Command::Debug {
                verb: DebugVerb::Wait,
                args: args("mapped foot --timeout 500")
            }
        );
    }

    #[test]
    fn masked_shots_and_report_are_not_built_yet() {
        // A named output is captured over the wire (`capture.rs`), the default output, a region
        // and a window go through Debug1 (`shot.rs`); a masked shot needs P4's mask pass.
        let shot = parse(&args("shot --masked out.png")).expect("shot");
        assert_eq!(exit_code(shot), COULD_NOT_RUN, "a masked shot");
        assert_eq!(exit_code(Command::Report), COULD_NOT_RUN);
    }
}
