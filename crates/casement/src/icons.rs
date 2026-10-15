//! The app icons under the windows in Mission Control: where the desktop entries are looked
//! for ([`IconSearch`]) and the icons found for the windows on show ([`IconBook`]). The finding,
//! decoding and caching is `casement-icons`; the drawing is `casement-render`.

pub(crate) mod book;
mod search;

pub use book::IconBook;
pub use search::IconSearch;
