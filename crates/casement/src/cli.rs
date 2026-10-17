//! The command line.
//!
//! ```text
//! casement [--backend headless|winit|tty] [--clock wall|stepped] [--outputs SPEC[;SPEC...]]...
//!          [--renderer pixman|llvmpipe] [--dmabuf none|soft] [--safe] [--config-home DIR]
//!          [--state-home DIR] [--system-dir DIR] [--xwayland satellite|off] [--xwayland-satellite PATH]
//!          [--drm-node PATH [--kms-report FILE] [--writeback-raw PATH]]
//! casement --write-schema PATH
//! ```
//!
//! `--outputs` takes `casement_backend::VirtualOutputSpec`'s grammar, several separated by `;`
//! or given as several flags. `--safe` is pixman (on a seat: pixman into dumb buffers, no GPU
//! opened), no effects, one output (the session's crash guard). The defaults are the headless
//! backend, the wall clock, pixman, no dma-buf and one 1920x1080 output at scale 1.
//!
//! `--drm-node`, `--kms-report` and `--writeback-raw` are the test mode of `dev/vkms/accept.sh`
//! ([`TtyMode::Direct`]): they need `--backend tty` and each other (the last two need
//! `--drm-node`).

use crate::icons::IconSearch;
use crate::privacy::SystemDir;
use crate::space_store::{StateDirs, StateHome};
use crate::xwayland::{SatellitePath, XwaylandConfig, XwaylandMode};
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
    /// Where state files (the Spaces file) live: `At` when `--state-home DIR` named one, else
    /// `Ephemeral` and `main` resolves it from the backend and the environment.
    pub state_home: StateHome,
    /// Where the administrator's files are looked for (`--system-dir`; `/etc/quire` unless a
    /// jailed run or a test names its own).
    pub system_dir: SystemDir,
    /// How X11 clients are served (A5).
    pub xwayland: XwaylandConfig,
    /// Whether `xwayland` came from a flag, or is the default `main` replaces with what the
    /// settings file and the installed programs say.
    pub xwayland_source: XwaylandSource,
    /// Where the tty backend gets its seat: the real one, or the vkms test mode.
    pub tty: TtyMode,
    /// Whether the app icons in Mission Control are looked for, and where: `Off` unless `main`
    /// resolves the XDG folders for a backend that has a person at it.
    pub icons: IconSearch,
}

/// Where [`StartConfig::xwayland`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum XwaylandSource {
    /// Neither `--xwayland` nor `--xwayland-satellite` was given: the settings file decides.
    Default,
    /// A flag was given, and it wins over the settings file.
    Flag,
}

/// Where the tty backend gets its seat. (Ignored by the other backends.)
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TtyMode {
    /// The machine's seat: libseat, udev, every GPU and input device.
    Seat,
    /// `--drm-node`: one vkms node opened directly, for `dev/vkms/accept.sh`.
    Direct(DirectTty),
}

/// The test mode on a vkms node (`--drm-node PATH`): nothing else on the machine is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DirectTty {
    /// The DRM node, opened directly after its driver was checked to be vkms.
    pub node: PathBuf,
    /// `--kms-report`: the file one JSON line a second is appended to.
    pub report: Option<PathBuf>,
    /// `--writeback-raw`: where a frame read back through the writeback connector is written
    /// on `SIGHUP` (and its `.meta` description).
    pub writeback_raw: Option<PathBuf>,
}

impl DirectTty {
    /// The test mode on `node`, with no report and no writeback.
    pub fn new(node: PathBuf) -> DirectTty {
        DirectTty {
            node,
            report: None,
            writeback_raw: None,
        }
    }
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
    /// `--safe`: the settings file ignored (defaults, the render GPU chosen automatically), no
    /// effects (materials opaque, the plainest minimise), no X11 apps, no icons, one display;
    /// on the headless backend also the pixman renderer, and on a seat every output is drawn by
    /// the CPU into dumb buffers with no GPU driver loaded (CASB-07).
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
    Run(Box<StartConfig>),
    /// Write the settings schema (`casement.settings.toml`) into this directory and exit
    /// (detent reads it).
    WriteSchema(PathBuf),
    /// Print the usage and exit.
    Help,
}

/// Why the command line was refused.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
    /// A flag that only makes sense together with another.
    #[error("{flag} needs {needs}")]
    Needs {
        /// The flag.
        flag: &'static str,
        /// What it needs.
        needs: &'static str,
    },
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
        state_home: StateHome::Ephemeral,
        system_dir: SystemDir::Etc,
        xwayland: XwaylandConfig::off(),
        xwayland_source: XwaylandSource::Default,
        tty: TtyMode::Seat,
        icons: IconSearch::Off,
    };
    let mut node = None;
    let mut report = None;
    let mut writeback_raw = None;
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
            "--state-home" => {
                config.state_home = StateHome::At(StateDirs::at(PathBuf::from(value()?)))
            }
            "--system-dir" => config.system_dir = SystemDir::At(PathBuf::from(value()?)),
            "--xwayland" => {
                config.xwayland_source = XwaylandSource::Flag;
                config.xwayland.mode = word(
                    flag,
                    &value()?,
                    &[
                        ("satellite", XwaylandMode::Satellite),
                        ("off", XwaylandMode::Off),
                    ],
                )?
            }
            "--xwayland-satellite" => {
                config.xwayland_source = XwaylandSource::Flag;
                config.xwayland.satellite = SatellitePath(PathBuf::from(value()?))
            }
            "--drm-node" => node = Some(PathBuf::from(value()?)),
            "--kms-report" => report = Some(PathBuf::from(value()?)),
            "--writeback-raw" => writeback_raw = Some(PathBuf::from(value()?)),
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
    config.tty = tty_mode(config.backend, node, report, writeback_raw)?;
    if config.outputs.is_empty() {
        config
            .outputs
            .push(VirtualOutputSpec::parse(DEFAULT_OUTPUT)?);
    }
    if config.mode == StartMode::Safe {
        config.renderer = SoftRenderer::Pixman;
        config.outputs.truncate(1);
    }
    Ok(Command::Run(Box::new(config)))
}

/// The tty mode the test flags ask for: they need each other and the tty backend.
fn tty_mode(
    backend: BackendChoice,
    node: Option<PathBuf>,
    report: Option<PathBuf>,
    writeback_raw: Option<PathBuf>,
) -> Result<TtyMode, CliError> {
    let Some(node) = node else {
        return match (report, writeback_raw) {
            (Some(_), _) => Err(CliError::Needs {
                flag: "--kms-report",
                needs: "--drm-node",
            }),
            (None, Some(_)) => Err(CliError::Needs {
                flag: "--writeback-raw",
                needs: "--drm-node",
            }),
            (None, None) => Ok(TtyMode::Seat),
        };
    };
    if backend != BackendChoice::Tty {
        return Err(CliError::Needs {
            flag: "--drm-node",
            needs: "--backend tty",
        });
    }
    Ok(TtyMode::Direct(DirectTty {
        node,
        report,
        writeback_raw,
    }))
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
pub const USAGE: &str = concat!(
    "usage: casement [--backend headless|winit|tty] [--clock wall|stepped] ",
    "[--outputs SPEC[;SPEC...]] [--renderer pixman|llvmpipe] [--dmabuf none|soft] [--safe] ",
    "[--config-home DIR] [--state-home DIR] [--system-dir DIR] [--xwayland satellite|off] ",
    "[--xwayland-satellite PATH]\n",
    "       casement --write-schema DIR\n",
    "\n",
    "The administrator's privacy settings are read from /etc/quire; --system-dir DIR reads them\n",
    "from DIR instead (privacy.toml and privacy.d/), for a jailed run or a test.\n",
    "\n",
    "Without --xwayland, older X11 apps follow xwayland.mode in settings.toml (auto, satellite\n",
    "or off; auto runs them when xwayland-satellite is installed). Headless runs stay off.\n",
    "\n",
    "Test mode for the vkms acceptance script (dev/vkms/accept.sh). It needs --backend tty and\n",
    "is not a general feature: it runs on one virtual (vkms) display node and opens nothing else,\n",
    "no seat, no VT, no other GPU, no input device.\n",
    "  --drm-node PATH       open only this DRM node, directly; refused unless its driver is vkms.\n",
    "                        SIGUSR1 then pauses the session and SIGUSR2 activates it (a\n",
    "                        simulated VT switch).\n",
    "  --kms-report FILE     append one JSON line a second: flips, missed vblanks, seat held or\n",
    "                        lent, and each output's name, mode and power (needs --drm-node).\n",
    "  --writeback-raw PATH  on SIGHUP read one frame back through the writeback connector and\n",
    "                        write the raw buffer to PATH and \"WIDTH HEIGHT PITCH FOURCC\" to\n",
    "                        PATH.meta (needs --drm-node).\n",
);

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    fn run(text: &str) -> StartConfig {
        match parse_args(&args(text)).expect(text) {
            Command::Run(config) => *config,
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
    fn the_state_home_flag_names_the_state_directory() {
        assert_eq!(run("").state_home, StateHome::Ephemeral);
        assert_eq!(
            run("--state-home /tmp/st").state_home,
            StateHome::At(StateDirs::at(PathBuf::from("/tmp/st")))
        );
        assert!(matches!(
            parse_args(&args("--state-home")),
            Err(CliError::MissingValue(_))
        ));
    }

    #[test]
    fn the_xwayland_flags_are_read() {
        let config = run("--xwayland satellite --xwayland-satellite /opt/satellite");
        assert_eq!(config.xwayland.mode, XwaylandMode::Satellite);
        assert_eq!(
            config.xwayland.satellite,
            SatellitePath(PathBuf::from("/opt/satellite"))
        );
        assert_eq!(config.xwayland_source, XwaylandSource::Flag);
        assert_eq!(run("").xwayland.mode, XwaylandMode::Off);
        assert_eq!(run("").xwayland_source, XwaylandSource::Default);
        assert!(matches!(
            parse_args(&args("--xwayland x11")),
            Err(CliError::BadValue { .. })
        ));
    }

    #[test]
    fn the_vkms_flags_are_read_together() {
        let config = run(
            "--backend tty --drm-node /dev/dri/card1 --kms-report /tmp/r.jsonl --writeback-raw /tmp/w.raw",
        );
        assert_eq!(
            config.tty,
            TtyMode::Direct(DirectTty {
                node: PathBuf::from("/dev/dri/card1"),
                report: Some(PathBuf::from("/tmp/r.jsonl")),
                writeback_raw: Some(PathBuf::from("/tmp/w.raw")),
            })
        );
        let only_node = run("--drm-node /dev/dri/card1 --backend tty");
        assert_eq!(
            only_node.tty,
            TtyMode::Direct(DirectTty::new(PathBuf::from("/dev/dri/card1")))
        );
        assert_eq!(run("--backend tty").tty, TtyMode::Seat);
        assert_eq!(run("").tty, TtyMode::Seat);
    }

    #[test]
    fn the_vkms_flags_need_each_other_and_the_tty_backend() {
        for text in [
            "--backend tty --kms-report /tmp/r",
            "--backend tty --writeback-raw /tmp/w",
            "--drm-node /dev/dri/card1",
            "--backend winit --drm-node /dev/dri/card1",
        ] {
            assert!(
                matches!(parse_args(&args(text)), Err(CliError::Needs { .. })),
                "{text}"
            );
        }
        assert!(matches!(
            parse_args(&args("--backend tty --drm-node")),
            Err(CliError::MissingValue(_))
        ));
    }

    #[test]
    fn the_usage_names_the_hooks_the_script_probes_for() {
        for hook in ["--drm-node", "--kms-report", "--writeback-raw"] {
            assert!(USAGE.contains(hook), "{hook}");
        }
        assert!(USAGE.contains("SIGUSR1") && USAGE.contains("SIGHUP"));
        assert!(USAGE.contains("not a general feature"));
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
