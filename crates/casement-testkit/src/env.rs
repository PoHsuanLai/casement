//! The golden environment: whether goldens are blessed (`CASEMENT_BLESS=1`, deliberate,
//! CONVENTIONS addendum 12) and where differing frames are written. The only reader of
//! `CASEMENT_BLESS`.

use std::path::PathBuf;

/// The golden environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    /// Whether differing goldens are replaced.
    pub bless: bool,
    /// Where actual and diff images go (`$CASEMENT_GOLDEN_OUT`, else the system temp dir's
    /// `casement-golden`).
    pub out_dir: PathBuf,
}

impl Env {
    /// The environment of this test process.
    pub fn of_process() -> Env {
        Env::of(
            std::env::var("CASEMENT_BLESS").ok().as_deref(),
            std::env::var_os("CASEMENT_GOLDEN_OUT").map(PathBuf::from),
        )
    }

    /// The environment `bless` and `out` describe (`CASEMENT_BLESS` is on only as `1`).
    pub fn of(bless: Option<&str>, out: Option<PathBuf>) -> Env {
        Env {
            bless: bless == Some("1"),
            out_dir: out.unwrap_or_else(|| std::env::temp_dir().join("casement-golden")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blessing_is_deliberate() {
        assert!(Env::of(Some("1"), None).bless);
        for value in [None, Some("0"), Some("yes"), Some("")] {
            assert!(!Env::of(value, None).bless);
        }
        assert_eq!(
            Env::of(None, Some(PathBuf::from("/x"))).out_dir,
            PathBuf::from("/x")
        );
    }
}
