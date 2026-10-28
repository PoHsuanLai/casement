//! The globals table: which global needs which consent, and who is shown what (I7). The golden
//! `tests/golden/globals.txt` has one row per global; a new one is a row to review.

use casement_core::policy::{
    ConnectionRole, GlobalKind, PowerSet, advertised, advertised_to, power_of, trusted_powers,
};
use casement_core::privilege::ServiceRole;
use casement_protocols::vocab::{Power, WireCode};
use porter_core::SandboxFacts;

const GOLDEN: &str = include_str!("../golden/globals.txt");

fn sandbox() -> ConnectionRole {
    ConnectionRole::Sandboxed(SandboxFacts::Flatpak {
        app: "org.example.App".to_owned(),
        instance: "1".to_owned(),
    })
}

fn word(seen: bool) -> &'static str {
    if seen { "yes" } else { "no" }
}

/// The table as the golden writes it.
fn table() -> Vec<String> {
    GlobalKind::all()
        .map(|kind| {
            let power = power_of(kind).map_or_else(|| "-".to_owned(), |power| format!("{power:?}"));
            format!(
                "{kind:?} {power} {} {} {}",
                word(advertised(kind, &ConnectionRole::Public)),
                word(advertised(kind, &ConnectionRole::Agent)),
                word(advertised(kind, &sandbox())),
            )
        })
        .collect()
}

#[test]
fn the_table_is_the_golden() {
    let golden: Vec<&str> = GOLDEN
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(table(), golden);
}

#[test]
fn the_shell_is_shown_every_global() {
    for kind in GlobalKind::all() {
        assert!(advertised(kind, &ConnectionRole::Shell), "{kind:?}");
    }
}

/// I7: a global a sandbox is kept from, and the public socket is shown, either needs consent
/// or is one of the two that stay ungated on purpose (layer shell, whose keyboard grab the
/// model closes, and the security-context manager).
#[test]
fn no_privileged_global_on_the_public_socket_is_ungated() {
    let ungated_on_purpose = [GlobalKind::LayerShell, GlobalKind::SecurityContext];
    for kind in GlobalKind::all() {
        let privileged = !advertised(kind, &sandbox()) && advertised(kind, &ConnectionRole::Public);
        if privileged && !ungated_on_purpose.contains(&kind) {
            assert!(
                power_of(kind).is_some(),
                "{kind:?} is privileged and has no power"
            );
        }
    }
}

/// A global with a power is shown to a public client until that power is refused, and only
/// then; nobody else is affected.
#[test]
fn a_refused_power_hides_its_globals_from_a_public_client_only() {
    for kind in GlobalKind::all() {
        for &power in <Power as WireCode>::ALL {
            let hidden = PowerSet::default().with(power);
            let public = advertised_to(kind, &ConnectionRole::Public, &hidden);
            let want = match (power_of(kind), advertised(kind, &ConnectionRole::Public)) {
                (Some(own), true) if own == power => false,
                (_, seen) => seen,
            };
            assert_eq!(public, want, "{kind:?} without {power:?}");
            for role in [
                ConnectionRole::Shell,
                ConnectionRole::Agent,
                ConnectionRole::Portal,
                sandbox(),
            ] {
                assert_eq!(
                    advertised_to(kind, &role, &hidden),
                    advertised(kind, &role),
                    "{kind:?} in {role:?} without {power:?}"
                );
            }
        }
    }
}

/// NEW-FA-2: the portal's own connection is shown what the public socket is shown and nothing the
/// shell or the agent tier alone are: not `quire_shell_v1`, not `quire_desktop_manager_v1`.
#[test]
fn the_portal_is_shown_what_every_client_is_and_not_the_shell_or_agent_globals() {
    for kind in GlobalKind::all() {
        assert_eq!(
            advertised(kind, &ConnectionRole::Portal),
            advertised(kind, &ConnectionRole::Public),
            "{kind:?}"
        );
    }
    for kind in [GlobalKind::QuireShell, GlobalKind::QuireDesktop] {
        assert!(!advertised(kind, &ConnectionRole::Portal), "{kind:?}");
    }
}

#[test]
fn each_power_gates_at_least_one_global() {
    for &power in <Power as WireCode>::ALL {
        assert!(
            GlobalKind::all().any(|kind| power_of(kind) == Some(power)),
            "{power:?} gates nothing"
        );
    }
}

#[test]
fn the_shell_and_the_portal_are_trusted_and_nobody_else() {
    let role = |text: &str| ServiceRole::parse(text).expect("a role");
    assert_eq!(trusted_powers(&role("shell")), <Power as WireCode>::ALL);
    assert_eq!(
        trusted_powers(&role("portal")),
        &[Power::Screen, Power::Windows]
    );
    for other in [
        "portal_frontend",
        "agent_tier",
        "updater_cli",
        "compositor",
        "x",
    ] {
        assert!(trusted_powers(&role(other)).is_empty(), "{other}");
    }
}

#[test]
fn a_role_is_a_small_lowercase_word() {
    let long = "a".repeat(65);
    for bad in ["", "Shell", "has space", "dash-ed", long.as_str()] {
        assert!(ServiceRole::parse(bad).is_err(), "{bad:?}");
    }
    assert!(ServiceRole::parse("portal_frontend").is_ok());
}
