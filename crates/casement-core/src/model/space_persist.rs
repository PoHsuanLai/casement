//! The Spaces across a restart: what the model stores and how it adopts a stored file.
//! P1-M2 fills it.

use super::Model;
use super::moved::Moved;
use crate::space::SpaceFile;

impl Model {
    /// The Spaces file for the model as it is (`SpaceFile::of_book`).
    pub fn space_file(&self) -> SpaceFile {
        todo!("P1-M2: Model::space_file")
    }

    /// Adopt a stored file: each stored Space keeps its id and place on its output once that
    /// output is plugged (a Space of an output not plugged yet waits); ids the file does not
    /// name are minted as before. Idempotent.
    pub(super) fn restore_spaces(self, file: SpaceFile) -> Moved {
        let _ = (self, file);
        todo!("P1-M2: Model::restore_spaces")
    }
}
