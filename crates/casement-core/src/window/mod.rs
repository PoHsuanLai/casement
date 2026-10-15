//! Windows: role, place, state, parent, emphasis, limits, title and hints (03 §1.1).

mod frame;
mod frame_arm;
mod frame_draw;
mod frame_press;
mod hints;
mod limits;
mod menu;
mod menu_dwell;
mod request;
mod state;
mod title;
#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod window;

pub use frame::{FrameGeometry, FrameHit, FrameMetrics, Light};
pub use frame_arm::{ArmInput, ArmOutcome, ArmReach, LightArm, LightAt};
pub use frame_draw::{FrameColour, FrameLook, FrameStyle, LightDraw, ShadowLayer};
pub use frame_press::{
    ChromeAction, ClickTracker, Clicks, DOUBLE_CLICK_SLOP, DOUBLE_CLICK_WITHIN, chrome_action,
};
pub use hints::{A11yRoot, FirstMouse, FrameHint, WindowHints};
pub use limits::{MaxSize, SizeLimits};
pub use menu::{
    MenuAction, MenuEntry, MenuGroup, MenuItem, Offer, SplitSide, SplitSupport, enabled_mask,
    is_offered, menu_for, menu_point,
};
pub use menu_dwell::{Dwell, DwellInput, DwellOutcome, DwellTiming};
pub use request::WlrToplevelRequest;
pub use state::{Restore, WindowState};
pub use title::WindowTitle;
pub use window::{Parent, SpacePlace, StackRank, Window};
