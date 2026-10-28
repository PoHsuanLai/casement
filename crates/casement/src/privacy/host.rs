//! The privacy files and the running compositor: the book and the floor are read when casement
//! starts, the floor again when asked to, the book is written whenever the model says it
//! changed, and the Privacy list Settings reads is kept in step with the model.

use crate::core_loop;
use crate::state::State;
use casement_core::Event;
use casement_core::privilege::{Book, Edit};
use casement_privacy::{load_book, load_floor, save_book};

/// Reads the book and the floor and tells the model. Called once, before any client connects.
pub fn start(state: &mut State) {
    let now = state.seams.wall.now();
    let book = load_book(&mut *state.seams.privacy, now);
    let floor = load_floor(&mut *state.seams.privacy);
    core_loop::feed(
        state,
        vec![
            Event::FloorLoaded(Box::new(floor)),
            Event::BookLoaded(Box::new(book)),
        ],
    );
    publish(state);
}

/// Reads the administrator's floor again (the book is never read again: the model's copy is
/// the authority and the file follows it).
pub fn reload_floor(state: &mut State) {
    let floor = load_floor(&mut *state.seams.privacy);
    core_loop::feed(state, vec![Event::FloorLoaded(Box::new(floor))]);
    publish(state);
    tracing::info!("the administrator's privacy settings were read again");
}

/// Keeps `book` (the model's `Effect::SaveBook`), then lets Settings see the new list.
pub fn keep(state: &mut State, book: &Book) {
    save_book(&mut *state.seams.privacy, book);
    publish(state);
}

/// A change Settings made to the list, as the model hears it, stamped with the date.
pub fn edited(state: &mut State, edit: Edit) {
    let at = state.seams.wall.now();
    core_loop::feed(state, vec![Event::PrivilegeEdited { edit, at }]);
    // The list Settings was shown after its own change was a guess; the model's replaces it,
    // also when the model changed nothing (a program whose answers are never kept).
    publish(state);
}

/// The model's list of answers, to the board Settings reads (nothing when casement serves no
/// bus; the board does nothing when the list is the one it has).
pub fn publish(state: &State) {
    if let Some(board) = &state.ipc.board {
        board.publish(state.core.model.privilege().rows());
    }
}
