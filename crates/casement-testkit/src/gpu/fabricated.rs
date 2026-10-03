//! The jail's fabricated GPU nodes (`jail.sh --fabricate-gpus N`, A3): directories
//! `/sys/dev/char/226:(128+n)/device/drm` for n < N, in the render-node minor range because the
//! drm crate types a node by `minor >> 6` and rejects 192..255.

use std::path::Path;

/// The DRM major.
pub const DRM_MAJOR: u32 = 226;

/// The first render-node minor.
pub const FIRST_RENDER_MINOR: u32 = 128;

/// One fabricated node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FabricatedNode {
    /// Its minor (128..192).
    pub minor: u32,
}

/// The render-range DRM nodes under `sys_dev_char` (`/sys/dev/char` in the jail), by minor.
/// Entries outside 226:128..191 are ignored.
pub fn fabricated_nodes(sys_dev_char: &Path) -> Vec<FabricatedNode> {
    let mut nodes: Vec<FabricatedNode> = std::fs::read_dir(sys_dev_char)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| {
            let (major, minor) = name.split_once(':')?;
            let (major, minor) = (major.parse::<u32>().ok()?, minor.parse::<u32>().ok()?);
            (major == DRM_MAJOR && (FIRST_RENDER_MINOR..FIRST_RENDER_MINOR + 64).contains(&minor))
                .then_some(FabricatedNode { minor })
        })
        .collect();
    nodes.sort();
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_render_range_drm_nodes_count() {
        let root = tempfile::tempdir().expect("temp dir");
        for name in [
            "226:129",
            "226:128",
            "226:200",
            "226:0",
            "13:64",
            "not-a-node",
        ] {
            std::fs::create_dir_all(root.path().join(name).join("device/drm")).expect("node");
        }
        assert_eq!(
            fabricated_nodes(root.path()),
            vec![FabricatedNode { minor: 128 }, FabricatedNode { minor: 129 }]
        );
        assert_eq!(fabricated_nodes(&root.path().join("missing")), Vec::new());
    }
}
