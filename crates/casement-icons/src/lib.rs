//! App icons for casement's own drawing (Mission Control's icon under each window): which icon an
//! app has, found in the order the person would expect, and decoded to premultiplied RGBA at the
//! exact pixel edge the output needs.
//!
//! The order is the toplevel's own pixels (`xdg_toplevel_icon`), its icon name, the app's desktop
//! entry (`Icon=`), then the app id itself as a theme name; the caller draws [`plate`] when none
//! of them gives an icon. Looking an icon up reads files, so a caller asks when the overview
//! opens or an app's icon changes, never per frame, and keeps the answers in an [`IconCache`].
//!
//! Nothing here reads the environment or the real system's folders by itself: the folders to
//! search are an argument ([`Finder`]), and the icon theme sits behind [`ThemeLookup`]
//! ([`SystemTheme`] is the real one, over `freedesktop-icons`).

mod cache;
mod decode;
mod desktop_entry;
mod error;
mod finder;
mod pixels;
mod plate;
mod theme;
mod want;

pub use cache::{IconCache, IconKey, Slot};
pub use decode::{load, scale_straight};
pub use desktop_entry::{DesktopEntry, entry_for, parse as parse_desktop_entry};
pub use error::IconError;
pub use finder::Finder;
pub use pixels::Pixels;
pub use plate::plate;
pub use theme::{SystemTheme, ThemeLookup, theme_file, theme_named};
pub use want::{RgbaBuffer, Want};
