//! Reading the laid-out document back into an [`Arrangement`]: each element's final box in
//! logical pixels.

use crate::document::SceneDocument;
use casement_layout::{Arrangement, SceneSpec};

/// The arrangement the laid-out `document` describes for `spec`.
pub fn arrangement(document: &SceneDocument, spec: &SceneSpec) -> Arrangement {
    let _ = (document, spec);
    todo!("P3-CSS")
}
