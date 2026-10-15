//! Space pictures for the shell's Spaces bar (`quire_shell_v1.get_space_capture_source`): the
//! sources and sessions of the ext-image-copy-capture objects that show one Space of an output
//! as a small picture, and the damage gate that holds a frame request until something in the
//! Space changed.
//!
//! The book is plain data and the functions are pure; [`SpaceShotPart`] adds the smithay
//! objects whose drop is a protocol event (a session's `stopped`, a held frame's `failed`).

use casement_core::OutputKey;
use casement_core::geometry::PhysicalSize;
use porter_core::SpaceId;
use smithay::wayland::image_copy_capture::{Frame, Session};
use std::collections::{BTreeMap, BTreeSet};

/// The smallest side of a Space picture, in pixels.
pub const MIN_SIDE: u32 = 1;

/// The largest side of a Space picture, in pixels.
pub const MAX_SIDE: u32 = 1024;

/// One source of a Space picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpaceSourceId(pub u64);

/// One capture session on such a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpaceSessionId(pub u64);

/// What a source shows.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpaceShot {
    /// The output.
    pub output: OutputKey,
    /// The Space.
    pub space: SpaceId,
    /// The picture's size in pixels.
    pub size: PhysicalSize,
}

/// A fingerprint of what the Space shows (its windows, where they are and how often they
/// committed): equal fingerprints are the same picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Look(pub u64);

/// What the damage gate decided for a frame request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// The Space changed since the session's last frame (or it had none): draw it.
    Draw,
    /// Nothing changed: the request waits until [`SpaceShotBook::changed`] says so.
    Hold,
}

/// The sources made, the sessions running and what each was last given.
#[derive(Debug, Default)]
pub struct SpaceShotBook {
    sources: BTreeMap<SpaceSourceId, SpaceShot>,
    sessions: BTreeMap<SpaceSessionId, SpaceShot>,
    delivered: BTreeMap<SpaceSessionId, Look>,
    waiting: BTreeSet<SpaceSessionId>,
    next: u64,
}

/// `side` kept between [`MIN_SIDE`] and [`MAX_SIDE`].
///
/// ```
/// use casement_wayland::quire::space_shot::clamp_side;
/// assert_eq!((clamp_side(0), clamp_side(160), clamp_side(5000)), (1, 160, 1024));
/// ```
pub fn clamp_side(side: u32) -> u32 {
    side.clamp(MIN_SIDE, MAX_SIDE)
}

/// The fingerprint of a list of parts (a window's id, its rectangle and its commit count), in
/// order. FNV-1a, so the same list gives the same number on every run.
///
/// ```
/// use casement_wayland::quire::space_shot::look_of;
/// assert_eq!(look_of([1, 2, 3]), look_of([1, 2, 3]));
/// assert_ne!(look_of([1, 2, 3]), look_of([1, 3, 2]));
/// ```
pub fn look_of(parts: impl IntoIterator<Item = i64>) -> Look {
    const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let hash = parts.into_iter().fold(BASIS, |hash, part| {
        part.to_le_bytes().iter().fold(hash, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
        })
    });
    Look(hash)
}

impl SpaceShotBook {
    fn mint(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    /// A client made a source of `space` on `output`, `width` by `height` (clamped).
    ///
    /// ```
    /// use casement_wayland::quire::space_shot::SpaceShotBook;
    /// let mut book = SpaceShotBook::default();
    /// assert!(book.sessions().is_empty());
    /// ```
    pub fn on_source(
        &mut self,
        output: OutputKey,
        space: SpaceId,
        width: u32,
        height: u32,
    ) -> SpaceSourceId {
        let id = SpaceSourceId(self.mint());
        let size = PhysicalSize {
            w: clamp_side(width),
            h: clamp_side(height),
        };
        self.sources.insert(
            id,
            SpaceShot {
                output,
                space,
                size,
            },
        );
        id
    }

    /// What the source shows, while it lives.
    pub fn source(&self, source: SpaceSourceId) -> Option<&SpaceShot> {
        self.sources.get(&source)
    }

    /// What the session shows, while it lives.
    pub fn session(&self, session: SpaceSessionId) -> Option<&SpaceShot> {
        self.sessions.get(&session)
    }

    /// The live sessions.
    pub fn sessions(&self) -> Vec<SpaceSessionId> {
        self.sessions.keys().copied().collect()
    }

    /// The source went (its sessions go on).
    pub fn on_source_destroyed(&mut self, source: SpaceSourceId) {
        self.sources.remove(&source);
    }

    /// A session started on `source`; none when the source is gone.
    pub fn on_session(&mut self, source: SpaceSourceId) -> Option<SpaceSessionId> {
        let shot = self.sources.get(&source)?.clone();
        let id = SpaceSessionId(self.mint());
        self.sessions.insert(id, shot);
        Some(id)
    }

    /// The session is over.
    pub fn drop_session(&mut self, session: SpaceSessionId) {
        self.sessions.remove(&session);
        self.delivered.remove(&session);
        self.waiting.remove(&session);
    }

    /// A frame of `session` was asked for while the Space looks like `look`.
    ///
    /// ```
    /// use casement_wayland::quire::space_shot::{Look, SpaceSessionId, SpaceShotBook};
    /// let mut book = SpaceShotBook::default();
    /// assert!(book.request(SpaceSessionId(9), Look(1)).is_none());
    /// ```
    pub fn request(&mut self, session: SpaceSessionId, look: Look) -> Option<Verdict> {
        if !self.sessions.contains_key(&session) {
            return None;
        }
        match self.delivered.get(&session) == Some(&look) {
            true => {
                self.waiting.insert(session);
                Some(Verdict::Hold)
            }
            false => Some(Verdict::Draw),
        }
    }

    /// The frame of `session` showing `look` was sent.
    pub fn delivered(&mut self, session: SpaceSessionId, look: Look) {
        self.waiting.remove(&session);
        self.delivered.insert(session, look);
    }

    /// The next frame of `session` is drawn even if the Space looks the same (its size changed
    /// or the last frame failed).
    pub fn forget(&mut self, session: SpaceSessionId) {
        self.delivered.remove(&session);
    }

    /// The sessions with a held request that `look_of_session` says changed since their last
    /// frame; each is no longer waiting, so the caller draws it now.
    pub fn changed(&mut self, look_of_session: impl Fn(&SpaceShot) -> Look) -> Vec<SpaceSessionId> {
        let due: Vec<SpaceSessionId> = self
            .waiting
            .keys()
            .copied()
            .filter(|id| match self.sessions.get(id) {
                Some(shot) => self.delivered.get(id) != Some(&look_of_session(shot)),
                None => false,
            })
            .collect();
        for id in &due {
            self.waiting.remove(id);
        }
        due
    }

    /// Whether any session waits or runs (the loop need not fingerprint anything otherwise).
    pub fn is_watching(&self) -> bool {
        !self.sessions.is_empty()
    }
}

/// The book and the smithay objects it stands for.
#[derive(Debug, Default)]
pub struct SpaceShotPart {
    /// The pure book.
    pub book: SpaceShotBook,
    held: BTreeMap<SpaceSessionId, Session>,
    frames: BTreeMap<SpaceSessionId, Frame>,
    commits: BTreeMap<casement_core::WindowId, u64>,
}

impl SpaceShotPart {
    /// Keeps `session` until its client destroys it or its Space goes.
    pub fn hold(&mut self, id: SpaceSessionId, session: Session) {
        self.held.insert(id, session);
    }

    /// The held session.
    pub fn session(&self, id: SpaceSessionId) -> Option<&Session> {
        self.held.get(&id)
    }

    /// Keeps the frame request of `id` the damage gate holds.
    pub fn hold_frame(&mut self, id: SpaceSessionId, frame: Frame) {
        self.frames.insert(id, frame);
    }

    /// The held request of `id`.
    pub fn take_frame(&mut self, id: SpaceSessionId) -> Option<Frame> {
        self.frames.remove(&id)
    }

    /// The client destroyed the session.
    pub fn release(&mut self, id: SpaceSessionId) {
        self.book.drop_session(id);
        self.held.remove(&id);
        self.frames.remove(&id);
    }

    /// The sessions end: dropping each tells its client `stopped`.
    pub fn stop(&mut self, ids: &[SpaceSessionId]) {
        for id in ids {
            self.held.remove(id);
            self.frames.remove(id);
            self.book.drop_session(*id);
        }
    }

    /// `window` committed: its commit count moves (only while a session watches).
    pub fn note_commit(&mut self, window: casement_core::WindowId) {
        if self.book.is_watching() {
            let count = self.commits.entry(window).or_default();
            *count = count.saturating_add(1);
        }
    }

    /// How often `window` committed while a session watched.
    pub fn commits_of(&self, window: casement_core::WindowId) -> u64 {
        self.commits.get(&window).copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casement_core::OutputName;
    use casement_core::output::{EdidFacts, EdidSerial};

    fn output() -> OutputKey {
        let edid = EdidFacts {
            make: String::new(),
            model: String::new(),
            serial: EdidSerial::Unknown,
        };
        OutputKey::of(&edid, &OutputName::parse("HDMI-A-1").expect("a name"))
    }

    fn book_with_session() -> (SpaceShotBook, SpaceSessionId) {
        let mut book = SpaceShotBook::default();
        let space = SpaceId::parse("s-a").expect("an id");
        let source = book.on_source(output(), space, 160, 90);
        let session = book.on_session(source).expect("a session");
        (book, session)
    }

    #[test]
    fn sizes_are_clamped_to_the_picture_bounds() {
        let mut book = SpaceShotBook::default();
        let space = SpaceId::parse("s-a").expect("an id");
        // name, asked, kept
        let cases = [
            ("zero", (0, 0), (1, 1)),
            ("as asked", (160, 90), (160, 90)),
            ("huge", (9000, 2000), (1024, 1024)),
        ];
        for (name, (w, h), (kw, kh)) in cases {
            let id = book.on_source(output(), space.clone(), w, h);
            let size = book.source(id).expect("a source").size;
            assert_eq!((size.w, size.h), (kw, kh), "{name}");
        }
    }

    #[test]
    fn the_first_frame_is_drawn_and_the_same_look_is_held_until_it_changes() {
        let (mut book, session) = book_with_session();
        assert_eq!(book.request(session, Look(1)), Some(Verdict::Draw));
        book.delivered(session, Look(1));
        assert_eq!(book.request(session, Look(1)), Some(Verdict::Hold));
        assert!(book.changed(|_| Look(1)).is_empty(), "nothing changed");
        assert_eq!(book.changed(|_| Look(2)), vec![session]);
        assert!(book.changed(|_| Look(2)).is_empty(), "released once");
    }

    #[test]
    fn a_forgotten_frame_is_drawn_again_at_the_same_look() {
        let (mut book, session) = book_with_session();
        book.delivered(session, Look(1));
        book.forget(session);
        assert_eq!(book.request(session, Look(1)), Some(Verdict::Draw));
    }

    #[test]
    fn a_session_on_a_destroyed_source_is_refused_and_one_before_it_goes_on() {
        let mut book = SpaceShotBook::default();
        let source = book.on_source(output(), SpaceId::parse("s-a").expect("id"), 8, 8);
        let session = book.on_session(source).expect("a session");
        book.on_source_destroyed(source);
        assert!(book.on_session(source).is_none());
        assert!(book.session(session).is_some());
        book.drop_session(session);
        assert!(book.request(session, Look(0)).is_none());
    }

    #[test]
    fn the_look_depends_on_order_and_value() {
        assert_eq!(look_of([]), look_of([]));
        assert_ne!(look_of([1]), look_of([2]));
        assert_ne!(look_of([1, 2]), look_of([2, 1]));
    }
}
