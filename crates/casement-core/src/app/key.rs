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
    /// The key for a peer known as `peer` that declared `claimed`.
    pub fn of(peer: &PeerIdentity, claimed: &ClaimedId) -> AppKey {
        let _ = (peer, claimed);
        todo!("P0-F1-window: Proven when the peer is proven, else Claimed")
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
