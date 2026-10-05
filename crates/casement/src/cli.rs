//! The command line.
//!
//! ```text
//! casement [--backend headless|winit|tty] [--clock wall|stepped] [--outputs SPEC[;SPEC...]]...
//!          [--renderer pixman|llvmpipe] [--dmabuf none|soft] [--safe] [--config-home DIR]
//! casement --write-schema PATH
//! ```
//!
//! `--outputs` takes `casement_backend::VirtualOutputSpec`'s grammar, several separated by `;`
//! or given as several flags. `--safe` is pixman, no effects, one output (the session's crash
//! guard). The defaults are the headless backend, the wall clock, pixman, no dma-buf and one
//! 1920x1080 output at scale 1.

use casement_backend::{
    BackendChoice, BackendError, DmabufMode, RenderCostModel, SoftRenderer, VirtualOutputSpec,
};
use std::io::Read;
use std::path::PathBuf;

/// The output a run gets when `--outputs` names none.
pub const DEFAULT_OUTPUT: &str = "HEADLESS-1:1920x1080@60000:120:0,0";

/// How casement starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartConfig {
    /// The backend.
    pub backend: BackendChoice,
    /// The clock.
    pub clock: ClockChoice,
    /// The virtual outputs (headless) or the window (winit, the first).
    pub outputs: Vec<VirtualOutputSpec>,
    /// The headless renderer.
    pub renderer: SoftRenderer,
    /// Whether dma-buf is offered.
    pub dmabuf: DmabufMode,
    /// What a frame costs (measured, except in tests).
    pub cost: RenderCostModel,
    /// Normal, or the crash guard's safe mode.
    pub mode: StartMode,
    /// Where the settings document lives.
    pub config_home: ConfigHome,
    /// Where Space-id entropy comes from.
    pub entropy: Entropy,
}

/// The session clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClockChoice {
    /// The monotonic wall clock.
    Wall,
    /// A virtual clock stepped by `casementctl debug clock` (the jailed binary's tests).
    Stepped,
    /// A virtual clock the test kit's `Fixture` advances (never on the command line).
    Virtual,
}

/// Normal, or safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StartMode {
    /// As configured.
    Normal,
    /// `--safe`: pixman, no effects, one output.
    Safe,
}

/// Where the settings document lives. (The sketch said `ConfigHome(PathBuf)`; the default is
/// read from the environment at start, which the parser must not do: FINDINGS.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigHome {
    /// `$XDG_CONFIG_HOME/casement` (read where the binary starts).
    Xdg,
    /// This directory stands in for `$XDG_CONFIG_HOME`: the settings file is
    /// `DIR/casement/settings.toml`.
    At(PathBuf),
}

/// Where Space-id entropy comes from (Space ids are minted from a seed: `Model::step` stays
/// pure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Entropy {
    /// The system's (`/dev/urandom`).
    System,
    /// These bytes (tests: deterministic ids).
    Fixed([u8; 16]),
}

impl Entropy {
    /// The sixteen bytes.
    pub fn bytes(self) -> std::io::Result<[u8; 16]> {
        match self {
            Entropy::Fixed(bytes) => Ok(bytes),
            Entropy::System => {
                let mut bytes = [0_u8; 16];
                std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
                Ok(bytes)
            }
        }
    }
}

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Run the compositor.
    Run(StartConfig),
    /// Write the settings schema (`casement.settings.toml`) into this directory and exit
    /// (detent reads it).
    WriteSchema(PathBuf),
    /// Print the usage and exit.
    Help,
}

/// Why the command line was refused.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// A flag casement does not know.
    #[error("unknown argument {0}")]
    Unknown(String),
    /// A flag without its value.
    #[error("{0} needs a value")]
    MissingValue(String),
    /// A value the flag does not take.
    #[error("{flag}: bad value {value}")]
    BadValue {
        /// The flag.
        flag: String,
        /// The value.
        value: String,
    },
    /// An output spec did not parse.
    #[error("{0}")]
    Output(#[from] BackendError),
}

/// What `args` (without the program name) ask for.
pub fn parse_args(args: &[String]) -> Result<Command, CliError> {
    let mut config = StartConfig {
        backend: BackendChoice::Headless,
        clock: ClockChoice::Wall,
        outputs: Vec::new(),
        renderer: SoftRenderer::Pixman,
        dmabuf: DmabufMode::None,
        cost: RenderCostModel::Measured,
        mode: StartMode::Normal,
        config_home: ConfigHome::Xdg,
        entropy: Entropy::System,
    };
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let mut value = || {
            rest.next()
                .cloned()
                .ok_or_else(|| CliError::MissingValue(flag.clone()))
        };
        match flag.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            "--write-schema" => return Ok(Command::WriteSchema(PathBuf::from(value()?))),
            "--backend" => {
                config.backend = word(
                    flag,
                    &value()?,
                    &[
                        ("headless", BackendChoice::Headless),
                        ("winit", BackendChoice::Winit),
                        ("tty", BackendChoice::Tty),
                    ],
                )?
            }
            "--clock" => {
                config.clock = word(
                    flag,
                    &value()?,
                    &[
                        ("wall", ClockChoice::Wall),
                        ("stepped", ClockChoice::Stepped),
                    ],
                )?
            }
            "--renderer" => {
                config.renderer = word(
                    flag,
                    &value()?,
                    &[
                        ("pixman", SoftRenderer::Pixman),
                        ("llvmpipe", SoftRenderer::Llvmpipe),
                    ],
                )?
            }
            "--dmabuf" => {
                config.dmabuf = word(
                    flag,
                    &value()?,
                    &[("none", DmabufMode::None), ("soft", DmabufMode::Soft)],
                )?
            }
            "--safe" => config.mode = StartMode::Safe,
            "--config-home" => config.config_home = ConfigHome::At(PathBuf::from(value()?)),
            "--outputs" => {
                let text = value()?;
                let specs = text
                    .split(';')
                    .filter(|spec| !spec.is_empty())
                    .map(VirtualOutputSpec::parse)
                    .collect::<Result<Vec<_>, _>>()?;
                config.outputs.extend(specs);
            }
            other => return Err(CliError::Unknown(other.to_owned())),
        }
    }
    if config.outputs.is_empty() {
        config
            .outputs
            .push(VirtualOutputSpec::parse(DEFAULT_OUTPUT)?);
    }
    if config.mode == StartMode::Safe {
        config.renderer = SoftRenderer::Pixman;
        config.outputs.truncate(1);
    }
    Ok(Command::Run(config))
}

/// The value of `flag` among `words`.
fn word<T: Copy>(flag: &str, value: &str, words: &[(&str, T)]) -> Result<T, CliError> {
    words
        .iter()
        .find(|(w, _)| *w == value)
        .map(|(_, t)| *t)
        .ok_or_else(|| CliError::BadValue {
            flag: flag.to_owned(),
            value: value.to_owned(),
        })
}

/// The usage text.
pub const USAGE: &str = "usage: casement [--backend headless|winit|tty] [--clock wall|stepped] \
[--outputs SPEC[;SPEC...]] [--renderer pixman|llvmpipe] [--dmabuf none|soft] [--safe] \
[--config-home DIR]\n       casement --write-schema DIR\n";

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn run(text: &str) -> StartConfig {
        match parse_args(&args(text)).expect(text) {
            Command::Run(config) => config,
            other => panic!("{text}: {other:?}"),
        }
    }

    #[test]
    fn the_defaults_are_headless_wall_pixman_one_output() {
        let config = run("");
        assert_eq!(config.backend, BackendChoice::Headless);
        assert_eq!(config.clock, ClockChoice::Wall);
        assert_eq!(config.renderer, SoftRenderer::Pixman);
        assert_eq!(config.dmabuf, DmabufMode::None);
        assert_eq!(config.mode, StartMode::Normal);
        assert_eq!(config.config_home, ConfigHome::Xdg);
        assert_eq!(
            config.outputs,
            vec![VirtualOutputSpec::parse(DEFAULT_OUTPUT).expect("default")]
        );
    }

    #[test]
    fn every_flag_is_read() {
        let config = run(
            "--backend headless --clock stepped --renderer llvmpipe --dmabuf soft --config-home /tmp/c \
             --outputs DP-1:3840x2160@60000:180:0,0;DP-2:1920x1080@60000:120:2560,0 --outputs HDMI-A-1:1280x720@50000:120:0,1440",
        );
        assert_eq!(config.clock, ClockChoice::Stepped);
        assert_eq!(config.renderer, SoftRenderer::Llvmpipe);
        assert_eq!(config.dmabuf, DmabufMode::Soft);
        assert_eq!(config.config_home, ConfigHome::At(PathBuf::from("/tmp/c")));
        let names: Vec<&str> = config.outputs.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, ["DP-1", "DP-2", "HDMI-A-1"]);
    }

    #[test]
    fn safe_mode_is_pixman_and_one_output() {
        let config = run(
            "--renderer llvmpipe --outputs A-1:800x600@60000:120:0,0;B-1:800x600@60000:120:800,0 --safe",
        );
        assert_eq!(config.mode, StartMode::Safe);
        assert_eq!(config.renderer, SoftRenderer::Pixman);
        assert_eq!(config.outputs.len(), 1);
    }

    #[test]
    fn write_schema_and_help_are_commands() {
        assert_eq!(
            parse_args(&args("--write-schema /tmp/s")).expect("schema"),
            Command::WriteSchema(PathBuf::from("/tmp/s"))
        );
        assert_eq!(parse_args(&args("--help")).expect("help"), Command::Help);
    }

    #[test]
    fn bad_command_lines_are_refused() {
        assert!(matches!(
            parse_args(&args("--frobnicate")),
            Err(CliError::Unknown(_))
        ));
        assert!(matches!(
            parse_args(&args("--backend")),
            Err(CliError::MissingValue(_))
        ));
        assert!(matches!(
            parse_args(&args("--backend x11")),
            Err(CliError::BadValue { .. })
        ));
        assert!(matches!(
            parse_args(&args("--clock virtual")),
            Err(CliError::BadValue { .. })
        ));
        assert!(matches!(
            parse_args(&args("--outputs DP-1")),
            Err(CliError::Output(_))
        ));
    }

    #[test]
    fn fixed_entropy_is_returned_as_is() {
        assert_eq!(Entropy::Fixed([7; 16]).bytes().expect("fixed"), [7; 16]);
    }
}
