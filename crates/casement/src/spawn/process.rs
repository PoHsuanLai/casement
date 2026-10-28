//! The real spawner: child processes, each reaped by the one that started it.

use super::scope::{manager_answers, scope_argv, unit_for};
use super::{EnvVar, Home, SERVICE_VARS, SpawnError, SpawnSpec, Spawned, Spawner};
use casement_core::Pid;
use casement_core::time::Mono;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// How often the children are looked at to see which ended. A look is one system call per
/// child, and nothing needs to know of an exit within a quarter of a second.
const SWEEP_EVERY: Duration = Duration::from_millis(250);

/// The real spawner: a child process per program, held until it ends so that it is reaped (no
/// zombie) and its exit can be told ([`Spawner::exited`]).
#[derive(Debug)]
pub struct ProcessSpawner {
    runtime: Option<PathBuf>,
    children: Vec<(Pid, Child)>,
    /// When the children were last looked at.
    swept: Option<Mono>,
    /// How many programs were started, for scope names.
    started: u32,
}

impl ProcessSpawner {
    /// A spawner that puts the programs a binding starts in scopes of their own in the service
    /// manager that listens under `runtime`, when there is one.
    pub fn new(runtime: Option<PathBuf>) -> ProcessSpawner {
        ProcessSpawner {
            runtime,
            children: Vec::new(),
            swept: None,
            started: 0,
        }
    }

    /// The command for `argv`: in a scope of its own when `home` asks for one and a manager
    /// answers, else the program itself.
    fn argv_for(&mut self, argv: &[String], home: Home) -> Vec<String> {
        match (home, manager_answers(self.runtime.as_deref())) {
            (Home::OwnScope, true) => {
                self.started = self.started.saturating_add(1);
                let program = argv.first().map_or("", String::as_str);
                let unit = unit_for(program, std::process::id(), self.started);
                scope_argv(argv, &unit)
            }
            _ => argv.to_vec(),
        }
    }
}

/// The command for `words` (a program and its arguments): without the service manager's
/// variables, with `env` added, on its own process group and with no input. Nothing when `words`
/// is empty.
fn command(words: &[String], env: &[EnvVar]) -> Option<Command> {
    let (program, args) = words.split_first()?;
    let mut command = Command::new(program);
    command.args(args);
    for name in SERVICE_VARS {
        command.env_remove(name);
    }
    command
        .envs(env.iter().map(|var| (&var.name, &var.value)))
        .stdin(Stdio::null())
        .process_group(0);
    Some(command)
}

impl Default for ProcessSpawner {
    fn default() -> ProcessSpawner {
        ProcessSpawner::new(None)
    }
}

impl Spawner for ProcessSpawner {
    fn spawn(&mut self, spec: SpawnSpec) -> Result<Spawned, SpawnError> {
        let SpawnSpec {
            argv,
            env,
            pass,
            home,
        } = spec;
        if argv.0.is_empty() {
            return Err(SpawnError::Empty);
        }
        let start = |source: std::io::Error| SpawnError::Start {
            program: argv.0.first().cloned().unwrap_or_default(),
            source,
        };
        for fd in &pass {
            rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::empty())
                .map_err(|errno| start(errno.into()))?;
        }
        let wanted = self.argv_for(&argv.0, home);
        let run = |words: &[String]| -> std::io::Result<Child> {
            command(words, &env)
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?
                .spawn()
        };
        // No `systemd-run` on this machine: the program itself, as with no manager at all.
        let child = match run(&wanted) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && wanted != argv.0 => {
                run(&argv.0)
            }
            other => other,
        }
        .map_err(start)?;
        // Our copies of the passed descriptors close; the child has its own.
        drop(pass);
        let pid = Pid(child.id());
        self.children.push((pid, child));
        Ok(Spawned::new(pid))
    }

    fn exited(&mut self, now: Mono) -> Vec<Pid> {
        if self.swept.is_some_and(|last| now.since(last) < SWEEP_EVERY) {
            return Vec::new();
        }
        self.swept = Some(now);
        let mut ended = Vec::new();
        let mut running = Vec::new();
        for (pid, mut child) in std::mem::take(&mut self.children) {
            match child.try_wait() {
                Ok(None) => running.push((pid, child)),
                // Ended, or not ours to wait for any more: either way it is not running.
                Ok(Some(_)) | Err(_) => ended.push(pid),
            }
        }
        self.children = running;
        ended
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::input::Argv;

    fn argv(words: &[&str]) -> Argv {
        Argv(words.iter().map(|word| (*word).to_owned()).collect())
    }

    fn at(secs: u64) -> Mono {
        Mono(Duration::from_secs(secs))
    }

    #[test]
    fn an_empty_argument_list_starts_nothing() {
        let spec = SpawnSpec::new(Argv(Vec::new()));
        assert!(matches!(
            ProcessSpawner::default().spawn(spec),
            Err(SpawnError::Empty)
        ));
    }

    #[test]
    fn a_program_that_ends_is_reaped_and_reported_once() {
        let mut spawner = ProcessSpawner::default();
        let started = spawner
            .spawn(SpawnSpec::new(argv(&["true"])))
            .expect("started");
        let ended = (0..400).find_map(|turn| {
            std::thread::sleep(Duration::from_millis(10));
            let got = spawner.exited(at(turn));
            (!got.is_empty()).then_some(got)
        });
        assert_eq!(ended, Some(vec![started.pid]));
        assert_eq!(spawner.exited(at(10_000)), Vec::<Pid>::new());
    }

    #[test]
    fn a_program_killed_from_outside_is_reported() {
        let mut spawner = ProcessSpawner::default();
        let started = spawner
            .spawn(SpawnSpec::new(argv(&["sleep", "30"])))
            .expect("started");
        assert_eq!(spawner.exited(at(0)), Vec::<Pid>::new());
        let pid = i32::try_from(started.pid.0)
            .ok()
            .and_then(rustix::process::Pid::from_raw)
            .expect("a pid");
        rustix::process::kill_process(pid, rustix::process::Signal::KILL).expect("killed");
        let ended = (1..400).find_map(|turn| {
            std::thread::sleep(Duration::from_millis(10));
            let got = spawner.exited(at(turn));
            (!got.is_empty()).then_some(got)
        });
        assert_eq!(ended, Some(vec![started.pid]));
    }

    #[test]
    fn the_children_are_looked_at_at_most_every_quarter_second() {
        let mut spawner = ProcessSpawner::default();
        spawner
            .spawn(SpawnSpec::new(argv(&["true"])))
            .expect("started");
        let _ = spawner.exited(at(100));
        std::thread::sleep(Duration::from_millis(200));
        // The same second: not looked at, whatever has happened.
        assert_eq!(spawner.exited(at(100)), Vec::<Pid>::new());
    }

    #[test]
    fn a_program_never_inherits_the_service_managers_variables() {
        let env = [EnvVar {
            name: "WAYLAND_DISPLAY".to_owned(),
            value: "wayland-1".to_owned(),
        }];
        let words = ["foot".to_owned()];
        let made = command(&words, &env).expect("a command");
        let set: Vec<_> = made.get_envs().collect();
        for name in SERVICE_VARS {
            let entry = set.iter().find(|(set, _)| *set == name);
            assert_eq!(
                entry.map(|(_, value)| value.is_none()),
                Some(true),
                "{name}"
            );
        }
        let wayland = set.iter().find(|(set, _)| *set == "WAYLAND_DISPLAY");
        assert_eq!(
            wayland.and_then(|(_, value)| *value),
            Some(std::ffi::OsStr::new("wayland-1"))
        );
        assert!(command(&[], &env).is_none());
    }

    #[test]
    fn only_a_binding_program_with_a_reachable_manager_runs_in_a_scope() {
        let dir = tempfile::tempdir().expect("a scratch folder");
        std::fs::create_dir(dir.path().join("systemd")).expect("folder");
        std::fs::write(dir.path().join("systemd/private"), b"").expect("socket stand-in");
        let words = ["foot".to_owned(), "-e".to_owned(), "top".to_owned()];
        let mut with_manager = ProcessSpawner::new(Some(dir.path().to_owned()));
        let scoped = with_manager.argv_for(&words, Home::OwnScope);
        assert_eq!(
            scoped[..5],
            ["systemd-run", "--user", "--scope", "--collect", "--quiet"]
        );
        assert_eq!(scoped[scoped.len() - 4..], ["--", "foot", "-e", "top"]);
        assert_eq!(with_manager.argv_for(&words, Home::WithCompositor), words);
        let mut without = ProcessSpawner::new(None);
        assert_eq!(without.argv_for(&words, Home::OwnScope), words);
    }
}
