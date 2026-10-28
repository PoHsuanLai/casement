//! Windows: role, place, state, parent, emphasis, limits, title and hints (03 §1.1).

mod frame;
mod frame_arm;
mod frame_draw;
mod frame_press;
mod hints;
mod limits;
mod menu;
mod menu_dwell;
mod origin;
mod pip;
mod request;
mod round;
mod sheet;
mod state;
mod tile_drag;
mod title;
#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod window;

pub use frame::{FrameGeometry, FrameHit, FrameMetrics, Light};
pub use frame_arm::{ArmInput, ArmOutcome, LightArm, LightAt};
pub use frame_draw::{FrameColour, FrameLook, FrameStyle, LightDraw, ShadowLayer};
pub use frame_press::{
    ChromeAction, ClickTracker, Clicks, DOUBLE_CLICK_SLOP, DOUBLE_CLICK_WITHIN, chrome_action,
};
pub use hints::{A11yRoot, FrameChoice, FrameHint, WindowHints};
pub use limits::SizeLimits;
pub use menu::{
    MenuAction, MenuGroup, MenuItem, enabled_mask, entry_action, entry_group, entry_label,
    is_offered, menu_for, menu_point,
};
pub use menu_dwell::{Dwell, DwellInput, DwellOutcome, DwellTiming};
pub use origin::WindowOrigin;
pub use pip::{
    PIP_MARGIN, PIP_MAX_AREA, PipSource, fit_aspect, floats_above, pip_source_of, snap_corner,
};
pub use request::WlrToplevelRequest;
pub use round::{Cast, Round, round_of};
pub use sheet::{attach_rect, sheet_parent, slide_clip, slide_start};
pub use state::{Restore, WindowState};
pub use tile_drag::{
    CORNER_REACH, EDGE_BAND, TileDrag, TileDrop, TileKnobs, TileScene, TileTarget, target_at,
};
pub use title::WindowTitle;
pub use window::{SpacePlace, StackRank, Window};
