//! How strongly the blur behind a layer surface frosts what lies under it.
//!
//! ext-background-effect carries no parameters, so a client cannot say how much; the shell
//! role it was given through `quire_shell_v1` already says what the surface is, and that picks
//! a strength here (the material of quire's design/03 section 17.3). A surface with no role
//! (a client's own layer surface) gets [`BlurStrength::Regular`], today's one look. Pure; the
//! renderer turns a strength into a look.

use super::ShellRoleTag;
use casement_protocols::vocab::ShellRole;

/// How strongly a blur frosts, lightest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum BlurStrength {
    /// A small, quick surface that should keep the picture behind it readable.
    Light,
    /// The one look every blur had before surfaces could differ (sigma 22, design/23).
    #[default]
    Regular,
    /// A large panel that should stand clear of a busy picture.
    Strong,
}

impl BlurStrength {
    /// The strength for a layer surface with this role tag.
    ///
    /// ```
    /// use casement_core::layer::{BlurStrength, ShellRoleTag};
    ///
    /// assert_eq!(BlurStrength::of(ShellRoleTag::Unassigned), BlurStrength::Regular);
    /// ```
    pub fn of(role: ShellRoleTag) -> BlurStrength {
        match role {
            ShellRoleTag::Unassigned => BlurStrength::Regular,
            ShellRoleTag::Assigned(role) => BlurStrength::of_role(role),
        }
    }

    /// The strength for a shell surface with this role. UNMEASURED: design/23 fixes only the
    /// widget's look (the Regular one); the others are a judgement, one table row each.
    pub fn of_role(role: ShellRole) -> BlurStrength {
        match role {
            ShellRole::Osd => BlurStrength::Light,
            ShellRole::Launcher | ShellRole::ControlCentre => BlurStrength::Strong,
            ShellRole::Bar
            | ShellRole::Dock
            | ShellRole::Switcher
            | ShellRole::Overview
            | ShellRole::Notifications
            | ShellRole::Wallpaper
            | ShellRole::Widget => BlurStrength::Regular,
        }
    }
}
