//! Spaces: each output's row of desktops, full-screen and Split View Spaces and agent Spaces;
//! casement mints their ids (C16).

mod book;
mod mint;
#[allow(
    clippy::module_inception,
    reason = "the freeze plan names the record file after its concept"
)]
mod space;

pub use book::SpaceBook;
pub use mint::{SpaceSeed, mint_space_id};
pub use space::{Space, SpaceContent, SpaceIndex, SplitPair, TaskToken};
