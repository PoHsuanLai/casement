//! The rules of an atomic `TEST_ONLY` commit, as a pure function (P1-D2): what the fake kernel
//! says to a commit, and what the real device checks before it hands a commit to the kernel
//! (the kernel's own test runs when a frame is queued).

use super::kms::{
    CommitVerdict, ConnectorId, ConnectorStatus, KmsCommit, KmsRefusal, KmsResources,
    ModesetPolicy, OutputCommit, VrrState,
};
use casement_core::output::VrrCapability;
use std::collections::BTreeMap;

/// The lit outputs of a device, by connector.
pub type Lit = BTreeMap<ConnectorId, OutputCommit>;

/// The verdict on `commit` for a device offering `resources` and currently lighting `lit`.
///
/// Every output needs an attached connector, a CRTC that can drive it and that no other output
/// uses, a mode the monitor lists, and (for variable refresh) a monitor and CRTC that can do it.
/// A changed CRTC or mode on a lit connector, or a newly lit one, is a modeset.
///
/// ```
/// use casement_backend::tty::{CommitVerdict, KmsCommit, KmsResources, Lit, ModesetPolicy, judge};
///
/// let nothing = KmsCommit::new(Vec::new(), ModesetPolicy::Forbid);
/// assert_eq!(judge(&KmsResources::default(), &Lit::new(), &nothing), CommitVerdict::Accepted);
/// ```
pub fn judge(resources: &KmsResources, lit: &Lit, commit: &KmsCommit) -> CommitVerdict {
    let refused = commit
        .outputs
        .iter()
        .enumerate()
        .find_map(|(n, output)| refusal(resources, lit, commit, n, output));
    if let Some(refusal) = refused {
        return CommitVerdict::Refused(refusal);
    }
    let modeset = commit.outputs.iter().any(|output| {
        lit.get(&output.connector)
            .is_none_or(|was| was.crtc != output.crtc || was.mode != output.mode)
    });
    match (modeset, commit.modeset) {
        (true, ModesetPolicy::Forbid) => CommitVerdict::NeedsModeset,
        _ => CommitVerdict::Accepted,
    }
}

fn refusal(
    resources: &KmsResources,
    lit: &Lit,
    commit: &KmsCommit,
    index: usize,
    output: &OutputCommit,
) -> Option<KmsRefusal> {
    let Some(connector) = resources
        .connectors
        .iter()
        .find(|c| c.id == output.connector && c.status == ConnectorStatus::Connected)
    else {
        return Some(KmsRefusal::Unsupported);
    };
    let crtc = resources.crtcs.iter().find(|c| c.id == output.crtc);
    let crtc_free = {
        let in_commit = |other: &OutputCommit| other.crtc == output.crtc;
        let twice = commit
            .outputs
            .iter()
            .enumerate()
            .any(|(n, other)| n != index && in_commit(other));
        let held = lit
            .values()
            .filter(|was| was.connector != output.connector)
            .filter(|was| !commit.outputs.iter().any(|o| o.connector == was.connector))
            .any(in_commit);
        !twice && !held
    };
    match crtc {
        Some(crtc) if connector.possible_crtcs.contains(&crtc.id) && crtc_free => {
            let vrr_ok = output.vrr == VrrState::Off
                || (connector.vrr == VrrCapability::Capable && crtc.vrr == VrrCapability::Capable);
            if !connector.modes.contains(&output.mode) {
                Some(KmsRefusal::Mode)
            } else if !vrr_ok {
                Some(KmsRefusal::Unsupported)
            } else {
                None
            }
        }
        _ => Some(KmsRefusal::NoCrtc),
    }
}
