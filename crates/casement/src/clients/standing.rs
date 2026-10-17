//! What the caller-identity library says of a connection's program, in the words the model and
//! the Dock use: [`ClientStanding`] for consent, [`PeerIdentity`] for the app a window belongs
//! to. Pure; `judge.rs` does the looking.

use casement_core::privilege::{ClientStanding, Engine, ProgramPath, SandboxAppId, ServiceRole};
use casement_ipc::dbus::peer::{self, Exe, Proof, Standing, roles};
use casement_wayland::security_context::Maker;
use porter_core::{AppId, AppName, Isolation, PeerIdentity};

/// The path of the image a process runs, when it could be read.
fn image_path(exe: &Exe) -> Option<ProgramPath> {
    match exe {
        Exe::Read { path, .. } => ProgramPath::parse(&path.to_text()).ok(),
        _ => None,
    }
}

/// The file name of the image, when it could be read.
pub(super) fn image_name(exe: &Exe) -> Option<String> {
    match exe {
        Exe::Read { path, .. } => path.file_name().map(str::to_owned),
        _ => None,
    }
}

/// Whether a service claim holds well enough to trust the connection as the desktop's own
/// (NEW-PI-3): only `Proof::Full`, which the library gives when the unit manager vouches for the
/// unit and the unit clears every loader variable. A cgroup and an image alone (`Proof::Exe`)
/// can be had by a same-user process that makes a control group of the right name and runs the
/// real image under `LD_PRELOAD`, so on the public socket they trust nobody: the shell, the
/// agent tier and the portal are handed their connections (`Compositor1.Connect`), which the bus
/// proves at `Full` (NEW-FA-2). A test build (`testing`, `debug-inject`) also takes `Proof::Dev`, the proof of a row
/// from the test caller file (`dev_callers`), which shows nothing about the image but is the
/// point of that file; every other build has no such proof.
fn trusted(proof: Proof) -> bool {
    proof >= Proof::Full || is_test_row(proof)
}

/// Whether the claim shows the process runs the root-owned image the table names (`Proof::Exe`
/// or better): enough to know which app it is, not to trust it.
fn image_shown(proof: Proof) -> bool {
    proof >= Proof::Exe
}

/// Whether `proof` is the proof of a test caller row.
#[cfg(any(feature = "testing", feature = "debug-inject"))]
fn is_test_row(proof: Proof) -> bool {
    proof == Proof::Dev
}

/// No test caller rows in this build: no proof is theirs.
#[cfg(not(any(feature = "testing", feature = "debug-inject")))]
fn is_test_row(_proof: Proof) -> bool {
    false
}

/// How the model takes the program `standing` describes, whose image is `exe`.
///
/// A desktop service counts as one only at `Proof::Full` ([`trusted`]). A service whose image
/// was shown but not its unit (`Proof::Exe`) is a program like any other, known by its image, so
/// per-app consent applies to it; a service known by its control group alone, which a same-user
/// process can move itself into, is nobody in particular. Anything the library could not vouch
/// for is unverified, and unverified programs are asked every time.
pub fn client_standing(standing: &Standing, exe: &Exe) -> ClientStanding {
    match standing {
        Standing::Service { role, proof, .. } if trusted(*proof) => {
            match ServiceRole::parse(role.as_str()) {
                Ok(role) => ClientStanding::Service {
                    role,
                    exe: image_path(exe),
                },
                Err(_) => unverified(exe),
            }
        }
        Standing::Service { proof, .. } if image_shown(*proof) => {
            image_path(exe).map_or_else(|| unverified(exe), |exe| ClientStanding::Program { exe })
        }
        Standing::Program { exe: path, .. } => match ProgramPath::parse(&path.to_text()) {
            Ok(exe) => ClientStanding::Program { exe },
            Err(_) => unverified(exe),
        },
        Standing::Sandboxed { app, engine } => {
            match (SandboxAppId::parse(app.as_str()), engine_of(*engine)) {
                (Ok(app), Some(engine)) => ClientStanding::Sandboxed { app, engine },
                _ => ClientStanding::Unverified { exe_name: None },
            }
        }
        Standing::Unverified { exe_name, .. } => ClientStanding::Unverified {
            exe_name: exe_name.clone(),
        },
        _ => unverified(exe),
    }
}

fn unverified(exe: &Exe) -> ClientStanding {
    ClientStanding::Unverified {
        exe_name: image_name(exe),
    }
}

fn engine_of(engine: peer::Engine) -> Option<Engine> {
    match engine {
        peer::Engine::Flatpak => Some(Engine::Flatpak),
        peer::Engine::Snap => Some(Engine::Snap),
        _ => None,
    }
}

/// The app a window of this program belongs to, for the Dock and the window list: the app a
/// desktop service is, the app a sandbox attests, or the app the launcher's scope names for a
/// program whose image the system owns. The scope name is a claim (anyone can start a scope
/// under any name), so it only groups windows; consent never follows it (it binds to the
/// image, `Subject`). A program the library could not vouch for belongs to no proven app.
pub fn peer_of(standing: &Standing) -> PeerIdentity {
    let proven = match standing {
        // A test caller row names its app as the image would (the point of that file).
        Standing::Service { app, proof, .. } if image_shown(*proof) || is_test_row(*proof) => {
            named(app.as_str(), Isolation::Unsandboxed)
        }
        Standing::Program {
            scope_app: Some(app),
            ..
        } => named(app.as_str(), Isolation::Unsandboxed),
        Standing::Sandboxed {
            app,
            engine: peer::Engine::Flatpak,
        } => named(app.as_str(), Isolation::Flatpak),
        _ => None,
    };
    proven.map_or(PeerIdentity::Unproven, PeerIdentity::Proven)
}

/// Whether the program is the desktop's sandbox launcher, the one program whose security
/// contexts give their clients an app (CASW-04): it stands as the table's `sandbox_launcher`
/// row (`/usr/bin/flatpak`), which has no unit, so the most a public-socket peer shows is its
/// root-owned image (`Proof::Exe`). A same-user process that runs that image under a preloaded
/// library looks the same; the library cannot see that at this proof.
pub fn maker_of(standing: &Standing) -> Maker {
    match standing {
        Standing::Service { role, proof, .. }
            if *role == roles::SANDBOX_LAUNCHER && (image_shown(*proof) || is_test_row(*proof)) =>
        {
            Maker::SandboxLauncher
        }
        _ => Maker::Other,
    }
}

fn named(app: &str, isolation: Isolation) -> Option<AppId> {
    AppName::parse(app)
        .ok()
        .map(|name| AppId { name, isolation })
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_ipc::dbus::peer::{AppKey, ExePath, RoleName, WhyNot};

    fn exe(path: &str) -> Exe {
        Exe::Read {
            path: ExePath::new(path).expect("a path"),
            trust: peer::FileTrust::RootOnly,
        }
    }

    fn program(path: &str, scope: Option<&str>) -> Standing {
        Standing::Program {
            exe: ExePath::new(path).expect("a path"),
            scope_app: scope.map(|app| AppKey::new(app).expect("an app")),
        }
    }

    fn service(role: &str, proof: Proof) -> Standing {
        Standing::Service {
            app: AppKey::new("org.quire.Portal").expect("an app"),
            role: RoleName::new(role).expect("a role"),
            proof,
            pin: peer::Pin::Reopened,
        }
    }

    /// (case, the library's standing, the image, what the model is told)
    #[test]
    fn the_model_is_told_only_what_the_library_vouched_for() {
        let grim = exe("/usr/bin/grim");
        let cases = [
            (
                "a system program",
                program("/usr/bin/grim", None),
                ClientStanding::Program {
                    exe: ProgramPath::parse("/usr/bin/grim").expect("a path"),
                },
            ),
            (
                "a service vouched for by the unit manager",
                service("portal", Proof::Full),
                ClientStanding::Service {
                    role: ServiceRole::parse("portal").expect("a role"),
                    exe: ProgramPath::parse("/usr/bin/grim").ok(),
                },
            ),
            (
                "a service shown by its image and cgroup only is a program (NEW-PI-3)",
                service("shell", Proof::Exe),
                ClientStanding::Program {
                    exe: ProgramPath::parse("/usr/bin/grim").expect("a path"),
                },
            ),
            (
                "a service known by its control group alone",
                service("portal", Proof::Cgroup),
                ClientStanding::Unverified {
                    exe_name: Some("grim".to_owned()),
                },
            ),
            (
                "a program the person can write",
                Standing::Unverified {
                    exe_name: Some("payload".to_owned()),
                    scope_app: None,
                    why: WhyNot::ImageWritable,
                },
                ClientStanding::Unverified {
                    exe_name: Some("payload".to_owned()),
                },
            ),
            (
                "a process that left",
                Standing::Unverified {
                    exe_name: None,
                    scope_app: None,
                    why: WhyNot::PeerGone,
                },
                ClientStanding::Unverified { exe_name: None },
            ),
            (
                "a sandboxed app",
                Standing::Sandboxed {
                    app: AppKey::new("org.gnome.Loupe").expect("an app"),
                    engine: peer::Engine::Flatpak,
                },
                ClientStanding::Sandboxed {
                    app: SandboxAppId::parse("org.gnome.Loupe").expect("an app"),
                    engine: Engine::Flatpak,
                },
            ),
        ];
        for (case, standing, want) in cases {
            assert_eq!(client_standing(&standing, &grim), want, "{case}");
        }
    }

    /// NEW-PI-3: per proof level, whether a claim to be a desktop service is trusted as one.
    #[test]
    fn only_a_full_proof_makes_a_connection_a_desktop_service() {
        let grim = exe("/usr/bin/grim");
        let is_service = |proof: Proof| {
            matches!(
                client_standing(&service("shell", proof), &grim),
                ClientStanding::Service { .. }
            )
        };
        assert!(!is_service(Proof::Cgroup));
        assert!(!is_service(Proof::Exe));
        assert!(is_service(Proof::Full));
        // The ladder's top is the only way in: nothing below it is a service.
        assert!(Proof::Cgroup < Proof::Exe && Proof::Exe < Proof::Full);
    }

    /// NEW-FA-2: the portal's own capture is trusted (no sheet of casement's) only when the
    /// library proves it at `Full` as the portal's row; a lesser proof is a program the person is
    /// asked about, and a control group alone is nobody. Which powers the portal role is trusted
    /// with is `trusted_powers`' to say (the screen and the windows, nothing more).
    #[test]
    fn the_portal_is_trusted_only_when_proven_full_and_otherwise_asked_like_a_program() {
        let transom = exe("/usr/libexec/transom");
        let path = ProgramPath::parse("/usr/libexec/transom").expect("a path");
        let portal = ServiceRole::parse("portal").expect("a role");
        let cases = [
            (
                "proven at Full",
                Proof::Full,
                ClientStanding::Service {
                    role: portal,
                    exe: Some(path.clone()),
                },
            ),
            (
                "shown by image and control group only",
                Proof::Exe,
                ClientStanding::Program { exe: path },
            ),
            (
                "known by its control group alone",
                Proof::Cgroup,
                ClientStanding::Unverified {
                    exe_name: Some("transom".to_owned()),
                },
            ),
        ];
        for (case, proof, want) in cases {
            assert_eq!(
                client_standing(&service("portal", proof), &transom),
                want,
                "{case}"
            );
        }
    }

    #[test]
    fn a_scope_name_groups_windows_but_only_for_a_program_the_system_owns() {
        let proven = |standing: &Standing| matches!(peer_of(standing), PeerIdentity::Proven(_));
        assert!(proven(&program(
            "/usr/bin/foot",
            Some("org.codeberg.dnkl.foot")
        )));
        assert!(!proven(&program("/usr/bin/foot", None)));
        assert!(proven(&service("portal", Proof::Exe)));
        assert!(!proven(&service("portal", Proof::Cgroup)));
        let writable = Standing::Unverified {
            exe_name: None,
            scope_app: None,
            why: WhyNot::ImageWritable,
        };
        assert!(!proven(&writable));
    }

    /// CASW-04: per standing, whether the program may make security contexts that carry an app.
    /// Only the table's launcher row, shown by its image, does; any other row, any other
    /// proven program and a scope that merely names a Flatpak app do not.
    #[test]
    fn only_the_launcher_row_makes_contexts_that_carry_an_app() {
        let launcher = |proof: Proof| Standing::Service {
            app: AppKey::new("org.quire.Flatpak").expect("an app"),
            role: roles::SANDBOX_LAUNCHER,
            proof,
            pin: peer::Pin::Reopened,
        };
        let cases = [
            (
                "the launcher by its image",
                launcher(Proof::Exe),
                Maker::SandboxLauncher,
            ),
            (
                "the launcher by its control group alone",
                launcher(Proof::Cgroup),
                Maker::Other,
            ),
            (
                "another desktop row",
                service("portal", Proof::Full),
                Maker::Other,
            ),
            (
                "a root-owned program",
                program("/usr/bin/foot", None),
                Maker::Other,
            ),
            (
                "a root-owned program in a scope it named app-flatpak-*",
                program("/usr/bin/foot", Some("org.mozilla.firefox")),
                Maker::Other,
            ),
            (
                "a sandboxed app",
                Standing::Sandboxed {
                    app: AppKey::new("org.mozilla.firefox").expect("an app"),
                    engine: peer::Engine::Flatpak,
                },
                Maker::Other,
            ),
        ];
        for (case, standing, want) in cases {
            assert_eq!(maker_of(&standing), want, "{case}");
        }
    }

    /// The launcher is proven as an app by its image, so its contexts are capped by a real proof,
    /// and its powers are those of a program of the system (the standing is unchanged).
    #[test]
    fn the_launcher_is_a_proven_program_with_no_extra_power() {
        let flatpak = exe("/usr/bin/flatpak");
        let standing = Standing::Service {
            app: AppKey::new("org.quire.Flatpak").expect("an app"),
            role: roles::SANDBOX_LAUNCHER,
            proof: Proof::Exe,
            pin: peer::Pin::Reopened,
        };
        assert!(matches!(peer_of(&standing), PeerIdentity::Proven(_)));
        assert_eq!(
            client_standing(&standing, &flatpak),
            ClientStanding::Program {
                exe: ProgramPath::parse("/usr/bin/flatpak").expect("a path"),
            }
        );
    }
}
