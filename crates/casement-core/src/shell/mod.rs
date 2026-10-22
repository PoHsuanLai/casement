//! What `quire_shell_v1` asks of the compositor and what the compositor tells the shell: the
//! surface roles, the minimise targets, and the system events only the compositor sees
//! (modifiers, edge reveals) (P2-Z).

mod note;
mod request;

pub use note::{ShellNote, SplitChoices, WindowMenuShow};
pub use request::ShellRequest;
