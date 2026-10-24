//! What casement draws for a window that draws its own frame: corners clipped at a radius and,
//! unless the client has drawn one, the shadow a framed window has. Which windows get which is
//! decided in `casement-core` (`window::round_of`); this is the data the scene draws from.

use super::Rounding;
use casement_core::window::FrameStyle;

/// Who casts the shadow of a window casement rounds, and how it looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientShadow {
    /// Casement casts the shadow of this style (its radius is the window's).
    Cast(FrameStyle),
    /// The client has drawn its own.
    Client,
}

/// A window with no frame of ours whose corners are clipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientRound {
    /// The corner radius, in logical pixels.
    pub radius: u32,
    /// The shadow.
    pub shadow: ClientShadow,
}

impl ClientRound {
    /// The style of the shadow casement casts, when it casts one.
    pub fn cast(&self) -> Option<&FrameStyle> {
        match &self.shadow {
            ClientShadow::Cast(style) => Some(style),
            ClientShadow::Client => None,
        }
    }
}

/// The clip a window is drawn through: a radius in logical pixels and which corners it rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipAsk {
    /// The corner radius, in logical pixels.
    pub radius: u32,
    /// Which corners.
    pub rounding: Rounding,
}
