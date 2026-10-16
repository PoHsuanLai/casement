//! `casementctl privacy list | reload`: which program may use which power, and the administrator's
//! settings read again.
//!
//! `list` reads the two files the way casement does (the person's book, the administrator's
//! floor) and prints each power with the programs it has an answer for, who set it, and what
//! could not be read. `reload` asks the running casement to read the administrator's settings
//! again; the person's answers are never read again, casement's copy is the one that counts.

use super::CommandError;
use casement_privacy::{FilePrivacyStore, Owner, PrivacyFiles, SystemDir, book_in, list};
use std::path::PathBuf;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

/// The well-known name casement holds on the session bus.
const COMPOSITOR: &str = "org.quire.Compositor1";
/// What a casement process calls itself (`/proc/<pid>/comm`).
const COMM: &str = "casement";

/// What `privacy` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Privacy {
    /// Print the list.
    List(Places),
    /// Ask casement to read the administrator's settings again.
    Reload,
}

/// Where the two files are, when not the usual places (a test or an unusual setup names them).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Places {
    /// `--state-home DIR`: casement's state folder, as casement's own flag of that name.
    pub state_home: Option<PathBuf>,
    /// `--system-dir DIR`: the administrator's folder instead of `/etc/quire`.
    pub system_dir: Option<PathBuf>,
}

/// What `args` (after `privacy`) ask for.
pub fn parse(args: &[String]) -> Result<Privacy, CommandError> {
    let (verb, rest) = args
        .split_first()
        .ok_or_else(|| CommandError::BadArgument("privacy needs list or reload".to_owned()))?;
    match verb.as_str() {
        "list" => places(rest).map(Privacy::List),
        "reload" if rest.is_empty() => Ok(Privacy::Reload),
        "reload" => Err(CommandError::BadArgument(
            "privacy reload takes no arguments".to_owned(),
        )),
        other => Err(CommandError::Unknown(format!("privacy {other}"))),
    }
}

fn places(args: &[String]) -> Result<Places, CommandError> {
    let mut places = Places::default();
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let mut value = || {
            rest.next()
                .map(PathBuf::from)
                .ok_or_else(|| CommandError::BadArgument(format!("{flag} needs a folder")))
        };
        match flag.as_str() {
            "--state-home" => places.state_home = Some(value()?),
            "--system-dir" => places.system_dir = Some(value()?),
            other => {
                return Err(CommandError::BadArgument(format!(
                    "privacy list: unknown flag {other}"
                )));
            }
        }
    }
    Ok(places)
}

/// Runs the command: exit 0 when it did what was asked, 1 when it could not.
pub fn run(privacy: Privacy) -> u8 {
    match privacy {
        Privacy::List(places) => list_to_stdout(&places),
        Privacy::Reload => reload(),
    }
}

/// Casement's state folder: `--state-home`, else `$XDG_STATE_HOME/casement`, else
/// `$HOME/.local/state/casement`.
fn state_dir(places: &Places) -> Option<PathBuf> {
    places.state_home.clone().or_else(|| {
        let base = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            })?;
        Some(base.join("casement"))
    })
}

fn list_to_stdout(places: &Places) -> u8 {
    let Some(dir) = state_dir(places) else {
        eprintln!("casementctl: cannot tell where your privacy answers are kept (no HOME)");
        return 1;
    };
    let system = places
        .system_dir
        .clone()
        .map_or(SystemDir::Etc, SystemDir::At);
    let mut store = FilePrivacyStore::new(PrivacyFiles {
        book: book_in(&dir),
        // A floor anywhere but `/etc/quire` is the person's own test setup: whoever runs this owns it.
        floor: system.floor(Owner::User(rustix::process::getuid().as_raw())),
    });
    let listing = list(&mut store);
    print!("{}", listing.render());
    for note in listing.notes() {
        eprintln!("casementctl: {note}");
    }
    0
}

fn reload() -> u8 {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("casementctl: {error}");
            return 1;
        }
    };
    match runtime.block_on(compositor_pid()).and_then(signal_reload) {
        Ok(()) => {
            println!("Asked casement to read your administrator's privacy settings again.");
            0
        }
        Err(why) => {
            eprintln!("casementctl: {why}");
            1
        }
    }
}

/// The process that holds casement's name on the session bus.
async fn compositor_pid() -> Result<i32, String> {
    let unreachable = |error: &dyn std::fmt::Display| {
        format!("casement cannot be reached on the session bus ({error}); is it running?")
    };
    let connection = zbus::Connection::session()
        .await
        .map_err(|error| unreachable(&error))?;
    let bus = DBusProxy::new(&connection)
        .await
        .map_err(|error| unreachable(&error))?;
    let name = BusName::try_from(COMPOSITOR).map_err(|error| unreachable(&error))?;
    let pid = bus
        .get_connection_unix_process_id(name)
        .await
        .map_err(|error| unreachable(&error))?;
    i32::try_from(pid).map_err(|_| "casement's process number is not usable".to_owned())
}

/// Sends the reload signal to `pid`, once it is checked to be a casement.
fn signal_reload(pid: i32) -> Result<(), String> {
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
    if comm.trim() != COMM {
        return Err("the program serving casement's name on the bus is not casement".to_owned());
    }
    let target = rustix::process::Pid::from_raw(pid)
        .ok_or_else(|| "casement's process number is not usable".to_owned())?;
    rustix::process::kill_process(target, rustix::process::Signal::HUP)
        .map_err(|error| format!("casement could not be told: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn the_privacy_subcommands() {
        assert_eq!(parse(&args("reload")).expect("reload"), Privacy::Reload);
        assert_eq!(
            parse(&args("list")).expect("list"),
            Privacy::List(Places::default())
        );
        assert_eq!(
            parse(&args("list --state-home /s --system-dir /e")).expect("list"),
            Privacy::List(Places {
                state_home: Some(PathBuf::from("/s")),
                system_dir: Some(PathBuf::from("/e")),
            })
        );
        for bad in [
            "",
            "reload now",
            "list --verbose",
            "list --state-home",
            "forget",
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad}");
        }
    }
}
