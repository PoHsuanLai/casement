//! The caps and clocks of the consent machine. Each is a named constant with its reason, and
//! whoever exceeds one is refused, never allowed to wedge the compositor.

use std::time::Duration;

/// The most grants the person's book holds: the file is read whole at start and rewritten
/// whole after every change, so it stays small. The oldest answer makes room for a new one.
pub const MAX_GRANTS: usize = 512;

/// The most rules the administrator's floor holds, for the same reason.
pub const MAX_FLOOR_RULES: usize = 512;

/// The most extra "never remember" programs the floor names.
pub const MAX_NEVER_REMEMBER: usize = 256;

/// The longest subject, in bytes: the wire's cap on a path or an app id.
pub const MAX_SUBJECT_BYTES: usize = 4096;

/// The longest sandboxed app id, in bytes.
pub const MAX_APP_BYTES: usize = 255;

/// The longest role name, in bytes.
pub const MAX_ROLE_BYTES: usize = 64;

/// The largest book or floor text read: the file reader's cap (64 KiB), so a damaged or
/// hostile file cannot make start-up slow.
pub const MAX_FILE_BYTES: usize = 64 * 1024;

/// The most requests one client may have held while its question is open: the cap the
/// protocol layer holds requests to, so a client cannot grow the compositor by asking.
pub const HELD_REQUESTS: usize = 256;

/// The most questions open at once, the one on screen included: a program cannot queue the
/// person's attention. Further ones are refused without being remembered.
pub const OPEN_ASKS: usize = 4;

/// The most different powers one client may ask for in [`RATE_WINDOW`]: a program cycling
/// through powers is refused.
pub const ASKS_PER_MINUTE: usize = 3;

/// The window [`ASKS_PER_MINUTE`] counts in.
pub const RATE_WINDOW: Duration = Duration::from_secs(60);

/// How long a question on screen waits for the person before it counts as dismissed.
pub const ASK_TTL: Duration = Duration::from_secs(120);

/// How long a question waits for a shell to ask it: the shell starts after the compositor,
/// and a shell that does not come back is not waited for.
pub const SHELL_WAIT: Duration = Duration::from_secs(30);

/// How long a program that was dismissed, or did not get an answer in time, is not asked
/// again: a tool that retries in a loop must not nag.
pub const REASK_AFTER: Duration = Duration::from_secs(10 * 60);

/// The most "not asked again" entries kept.
pub const MAX_COOLDOWNS: usize = 64;

/// The most clients that may wait on one question.
pub const MAX_WAITERS: usize = 256;
