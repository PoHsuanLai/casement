//! An app's key: its proven id, or the id it claimed (C39).

use casement_protocols::vocab::ProofKind;
use porter_core::{AppId, ClaimedId, Isolation, PeerIdentity};
use serde::{Deserialize, Serialize};

/// Which app a window belongs to. A proven key comes from the peer's scope or sandbox
/// (`porter_core::identity_of`); anything else is known only by the id it declared, which
/// policy never trusts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum AppKey {
    /// The peer's process proves this app.
    Proven(AppId),
    /// Nothing proves a name; the client declared this one.
    Claimed(ClaimedId),
}

impl AppKey {
    /// The key for a peer known as `peer` that declared `claimed`: the proven app when the
    /// peer's process proves one (`porter_core::identity_of`), else the declared id, however
    /// well it matches a real app's name.
    pub fn of(peer: &PeerIdentity, claimed: &ClaimedId) -> AppKey {
        match peer {
            PeerIdentity::Proven(app) => AppKey::Proven(app.clone()),
            PeerIdentity::Unproven => AppKey::Claimed(claimed.clone()),
        }
    }

    /// The key as `quire_desktop_app_v1.identity` sends it: the app name, or
    /// `claimed:<id>` for a claimed key (03 §1.1).
    pub fn wire(&self) -> String {
        match self {
            AppKey::Proven(app) => app.name.as_str().to_owned(),
            AppKey::Claimed(claimed) => format!("claimed:{}", claimed.as_str()),
        }
    }

    /// How the key was established, as the wire says it.
    pub fn proof(&self) -> ProofKind {
        match self {
            AppKey::Proven(AppId {
                isolation: Isolation::Flatpak,
                ..
            }) => ProofKind::SecurityContext,
            AppKey::Proven(_) => ProofKind::Scope,
            AppKey::Claimed(_) => ProofKind::Claimed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use porter_core::{CgroupPath, PeerFacts, SandboxFacts, identity_of};

    fn facts(cgroup: &str, sandbox: SandboxFacts) -> PeerFacts {
        PeerFacts {
            pid: 4242,
            cgroup: CgroupPath::parse(cgroup).expect("cgroup path"),
            sandbox,
        }
    }

    fn flatpak(app: &str) -> SandboxFacts {
        SandboxFacts::Flatpak {
            app: app.to_owned(),
            instance: "1".to_owned(),
        }
    }

    /// A name, the facts a client's peer gave, what it declared, and the key, its wire text
    /// and its proof.
    type Row = (
        &'static str,
        PeerFacts,
        &'static str,
        &'static str,
        &'static str,
        ProofKind,
    );

    /// `AppKey::of` over `identity_of`'s answers: the key is proven exactly when the peer's
    /// scope or sandbox proves it, whatever the client declares.
    #[test]
    fn a_key_is_proven_only_by_what_identity_of_proves() {
        let scope = "/user.slice/user-1000.slice/user@1000.service/app.slice/";
        let rows: Vec<Row> = vec![
            (
                "a launcher's scope proves the app",
                facts(
                    &format!("{scope}app-org.quire.Mail-1234.scope"),
                    SandboxFacts::Native,
                ),
                "org.quire.Mail",
                "org.quire.Mail",
                "scope",
                ProofKind::Scope,
            ),
            (
                "a proven peer's own declaration is ignored",
                facts(
                    &format!("{scope}app-org.quire.Mail-1234.scope"),
                    SandboxFacts::Native,
                ),
                "evil",
                "org.quire.Mail",
                "scope",
                ProofKind::Scope,
            ),
            (
                "a flatpak sandbox proves through its metadata",
                facts(
                    &format!("{scope}app-flatpak-org.gnome.Calc-1.scope"),
                    flatpak("org.gnome.Calc"),
                ),
                "calc",
                "org.gnome.Calc",
                "flatpak",
                ProofKind::SecurityContext,
            ),
            (
                "a terminal's scope is not a proof",
                facts(&format!("{scope}app-foot-99.scope"), SandboxFacts::Native),
                "foot",
                "claimed:foot",
                "claimed",
                ProofKind::Claimed,
            ),
            (
                "a session scope is not a proof",
                facts("/user.slice/session-3.scope", SandboxFacts::Native),
                "org.quire.Mail",
                "claimed:org.quire.Mail",
                "claimed",
                ProofKind::Claimed,
            ),
            (
                "another sandbox engine proves nothing",
                facts(
                    "/user.slice/session-3.scope",
                    SandboxFacts::Engine {
                        engine: "org.snap".to_owned(),
                        app: "org.snap.Thing".to_owned(),
                        instance: "1".to_owned(),
                    },
                ),
                "thing",
                "claimed:thing",
                "claimed",
                ProofKind::Claimed,
            ),
        ];
        for (name, peer, declared, wire, kind, proof) in rows {
            let claimed = ClaimedId::parse(declared).expect("claimed id");
            let key = AppKey::of(&identity_of(&peer), &claimed);
            assert_eq!(key.wire(), wire, "{name}: wire");
            assert_eq!(key.proof(), proof, "{name}: proof ({kind})");
            assert_eq!(
                matches!(key, AppKey::Proven(_)),
                proof != ProofKind::Claimed,
                "{name}: proven"
            );
        }
    }

    /// CASW-04 (porter PR #38): sandbox metadata alone does not prove a Flatpak app; the
    /// peer's cgroup must be that app's own `app-flatpak-<id>-<n>.scope`.
    #[test]
    fn flatpak_metadata_under_a_session_scope_is_not_proven() {
        let peer = facts("/user.slice/session-3.scope", flatpak("org.gnome.Calc"));
        let claimed = ClaimedId::parse("calc").expect("claimed id");
        let key = AppKey::of(&identity_of(&peer), &claimed);
        assert_eq!(key.proof(), ProofKind::Claimed);
        assert_eq!(key.wire(), "claimed:calc");
    }
}
