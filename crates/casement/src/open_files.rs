//! The open-file limit (NEW-CAS-1). Every client connection costs the compositor descriptors
//! (its socket, the handle on its process, the buffers it shares), and a user service starts
//! with a soft limit of 1024. The soft limit is raised at start, never past the hard limit: a
//! limit of a million would make every program the compositor starts (which inherits it) pay
//! for it.

use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};

/// The soft limit asked for: the connection cap (two descriptors each) and the device,
/// buffer and timer descriptors of a busy session fit with room to spare, and it stays far
/// from limits that slow programs that close every descriptor up to the limit.
pub const WANTED_SOFT_LIMIT: u64 = 8192;

/// What the limit should become, or `None` when it already is enough (or has no limit): the
/// soft limit raised to `wanted`, or to the hard limit when that is lower.
pub fn raised(now: Rlimit, wanted: u64) -> Option<Rlimit> {
    let target = now.maximum.map_or(wanted, |hard| hard.min(wanted));
    match now.current {
        Some(soft) if soft < target => Some(Rlimit {
            current: Some(target),
            maximum: now.maximum,
        }),
        Some(_) | None => None,
    }
}

/// Raises the soft limit on open files to [`WANTED_SOFT_LIMIT`] (or the hard limit). A refusal is
/// logged: the connection caps and the accept rest still keep the session alive.
pub fn raise() {
    let Some(next) = raised(getrlimit(Resource::Nofile), WANTED_SOFT_LIMIT) else {
        return;
    };
    match setrlimit(Resource::Nofile, next) {
        Ok(()) => tracing::info!(soft = ?next.current, "the open-file limit is raised"),
        Err(error) => tracing::warn!(%error, "the open-file limit could not be raised"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(soft: Option<u64>, hard: Option<u64>) -> Rlimit {
        Rlimit {
            current: soft,
            maximum: hard,
        }
    }

    #[test]
    fn the_soft_limit_rises_to_the_wish_or_the_hard_limit_whichever_is_lower() {
        let want = WANTED_SOFT_LIMIT;
        assert_eq!(
            raised(limits(Some(1024), Some(524_288)), want),
            Some(limits(Some(want), Some(524_288)))
        );
        assert_eq!(
            raised(limits(Some(1024), Some(4096)), want),
            Some(limits(Some(4096), Some(4096)))
        );
        assert_eq!(
            raised(limits(Some(1024), None), want),
            Some(limits(Some(want), None))
        );
    }

    #[test]
    fn a_limit_that_is_enough_or_absent_is_left_alone() {
        assert_eq!(
            raised(
                limits(Some(WANTED_SOFT_LIMIT), Some(1 << 20)),
                WANTED_SOFT_LIMIT
            ),
            None
        );
        assert_eq!(
            raised(limits(Some(4096), Some(4096)), WANTED_SOFT_LIMIT),
            None
        );
        assert_eq!(raised(limits(None, None), WANTED_SOFT_LIMIT), None);
    }
}
