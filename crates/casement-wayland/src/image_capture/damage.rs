//! What changed on an output, frame by frame, so that a capture can say what changed since the
//! frame its client had last. Both capture protocols read it: `ext_image_copy_capture` for
//! every session, `zwlr_screencopy_v1` for `copy_with_damage`. Pure data, fed by the
//! composition root with each frame an output presents.

use casement_core::OutputKey;
use casement_core::geometry::PhysicalRect;
use casement_core::time::Mono;
use std::collections::{BTreeMap, VecDeque};

/// How many frames with damage an output's log keeps; a client older than that gets the whole
/// area.
const KEPT: usize = 32;

/// More rectangles than this become their bounding box.
const MAX_RECTS: usize = 16;

/// What changed since a client's last frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Since {
    /// Everything: the client had no frame, or its frame is older than the log.
    Whole,
    /// These rectangles of the frame (none: nothing changed).
    Parts(Vec<PhysicalRect>),
}

impl Since {
    /// What changed inside `area`, in pixels from `area`'s top-left corner: the rectangles cut
    /// to it and moved, or all of it for [`Since::Whole`]. Empty when nothing changed there.
    ///
    /// ```
    /// use casement_core::geometry::PhysicalRect;
    /// use casement_wayland::image_capture::Since;
    ///
    /// let area = PhysicalRect { x: 10, y: 10, w: 100, h: 50 };
    /// let hit = PhysicalRect { x: 0, y: 0, w: 20, h: 20 };
    /// let inside = Since::Parts(vec![hit]).within(area);
    /// assert_eq!(inside, vec![PhysicalRect { x: 0, y: 0, w: 10, h: 10 }]);
    /// assert_eq!(Since::Whole.within(area).len(), 1);
    /// ```
    pub fn within(&self, area: PhysicalRect) -> Vec<PhysicalRect> {
        match self {
            Since::Whole => vec![PhysicalRect {
                x: 0,
                y: 0,
                w: area.w,
                h: area.h,
            }],
            Since::Parts(rects) => rects
                .iter()
                .filter_map(|rect| overlap(*rect, area))
                .map(|rect| PhysicalRect {
                    x: rect.x.saturating_sub(area.x),
                    y: rect.y.saturating_sub(area.y),
                    ..rect
                })
                .collect(),
        }
    }
}

/// One output's recent damage.
#[derive(Debug, Clone, Default)]
pub struct DamageLog {
    entries: VecDeque<(Mono, Vec<PhysicalRect>)>,
    newest: Option<Mono>,
    forgotten: Option<Mono>,
}

impl DamageLog {
    /// Notes that the frame presented at `at` changed `damage`. A frame at or before the newest
    /// one noted is ignored (each frame is noted once).
    pub fn record(&mut self, at: Mono, damage: Vec<PhysicalRect>) {
        if self.newest.is_some_and(|newest| at <= newest) {
            return;
        }
        self.newest = Some(at);
        let damage = merged(damage);
        if damage.is_empty() {
            return;
        }
        self.entries.push_back((at, damage));
        while self.entries.len() > KEPT {
            if let Some((dropped, _)) = self.entries.pop_front() {
                self.forgotten = Some(dropped);
            }
        }
    }

    /// What changed in the frames after the one presented at `last` (none: the client has no
    /// frame yet).
    pub fn since(&self, last: Option<Mono>) -> Since {
        let Some(last) = last else {
            return Since::Whole;
        };
        if self.forgotten.is_some_and(|forgotten| last < forgotten) {
            return Since::Whole;
        }
        let after = self
            .entries
            .iter()
            .filter(|(at, _)| *at > last)
            .flat_map(|(_, rects)| rects.iter().copied())
            .collect();
        Since::Parts(merged(after))
    }
}

/// The logs of every output.
#[derive(Debug, Default)]
pub struct DamageLogs {
    logs: BTreeMap<OutputKey, DamageLog>,
}

impl DamageLogs {
    /// Notes a frame of `output`.
    pub fn record(&mut self, output: &OutputKey, at: Mono, damage: Vec<PhysicalRect>) {
        self.logs
            .entry(output.clone())
            .or_default()
            .record(at, damage);
    }

    /// What changed on `output` since `last`; everything for an output with no log.
    pub fn since(&self, output: &OutputKey, last: Option<Mono>) -> Since {
        match self.logs.get(output) {
            Some(log) => log.since(last),
            None => Since::Whole,
        }
    }

    /// `output` went away.
    pub fn forget(&mut self, output: &OutputKey) {
        self.logs.remove(output);
    }
}

/// The part of `rect` inside `area`, none when they do not meet.
fn overlap(rect: PhysicalRect, area: PhysicalRect) -> Option<PhysicalRect> {
    let edge = |origin: i32, length: u32| i64::from(origin) + i64::from(length);
    let left = i64::from(rect.x).max(i64::from(area.x));
    let top = i64::from(rect.y).max(i64::from(area.y));
    let right = edge(rect.x, rect.w).min(edge(area.x, area.w));
    let bottom = edge(rect.y, rect.h).min(edge(area.y, area.h));
    if right <= left || bottom <= top {
        return None;
    }
    Some(PhysicalRect {
        x: i32::try_from(left).ok()?,
        y: i32::try_from(top).ok()?,
        w: u32::try_from(right - left).ok()?,
        h: u32::try_from(bottom - top).ok()?,
    })
}

/// `rects` without the empty ones and the repeats; one bounding box when there are too many.
fn merged(rects: Vec<PhysicalRect>) -> Vec<PhysicalRect> {
    let mut kept: Vec<PhysicalRect> = Vec::new();
    for rect in rects.into_iter().filter(|rect| rect.w > 0 && rect.h > 0) {
        if !kept.contains(&rect) {
            kept.push(rect);
        }
    }
    if kept.len() <= MAX_RECTS {
        return kept;
    }
    let left = kept.iter().map(|r| i64::from(r.x)).min().unwrap_or(0);
    let top = kept.iter().map(|r| i64::from(r.y)).min().unwrap_or(0);
    let right = kept
        .iter()
        .map(|r| i64::from(r.x) + i64::from(r.w))
        .max()
        .unwrap_or(0);
    let bottom = kept
        .iter()
        .map(|r| i64::from(r.y) + i64::from(r.h))
        .max()
        .unwrap_or(0);
    let bounding = (
        i32::try_from(left),
        i32::try_from(top),
        u32::try_from(right - left),
        u32::try_from(bottom - top),
    );
    match bounding {
        (Ok(x), Ok(y), Ok(w), Ok(h)) => vec![PhysicalRect { x, y, w, h }],
        _ => kept,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(millis: u64) -> Mono {
        Mono(Duration::from_millis(millis))
    }

    fn rect(x: i32, y: i32, w: u32, h: u32) -> PhysicalRect {
        PhysicalRect { x, y, w, h }
    }

    #[test]
    fn a_client_without_a_frame_gets_the_whole_area() {
        let mut log = DamageLog::default();
        log.record(at(16), vec![rect(0, 0, 4, 4)]);
        assert_eq!(log.since(None), Since::Whole);
    }

    #[test]
    fn damage_accumulates_over_the_frames_after_the_clients_last() {
        // (frames recorded as (millis, rects), the client's last, what it is told)
        let table = [
            (vec![], Some(0), vec![]),
            (
                vec![(16, vec![rect(0, 0, 2, 2)])],
                Some(0),
                vec![rect(0, 0, 2, 2)],
            ),
            (vec![(16, vec![rect(0, 0, 2, 2)])], Some(16), vec![]),
            (
                vec![(16, vec![rect(0, 0, 2, 2)]), (32, vec![rect(5, 5, 1, 1)])],
                Some(16),
                vec![rect(5, 5, 1, 1)],
            ),
            (
                vec![(16, vec![rect(0, 0, 2, 2)]), (32, vec![rect(5, 5, 1, 1)])],
                Some(0),
                vec![rect(0, 0, 2, 2), rect(5, 5, 1, 1)],
            ),
            // the same rectangle twice is told once
            (
                vec![(16, vec![rect(1, 1, 3, 3)]), (32, vec![rect(1, 1, 3, 3)])],
                Some(0),
                vec![rect(1, 1, 3, 3)],
            ),
            // a frame with no damage adds nothing
            (
                vec![(16, vec![]), (32, vec![rect(2, 2, 2, 2)])],
                Some(0),
                vec![rect(2, 2, 2, 2)],
            ),
            // an empty rectangle is no damage
            (vec![(16, vec![rect(2, 2, 0, 5)])], Some(0), vec![]),
        ];
        for (frames, last, expected) in table {
            let mut log = DamageLog::default();
            for (millis, rects) in &frames {
                log.record(at(*millis), rects.clone());
            }
            let told = log.since(last.map(at));
            assert_eq!(told, Since::Parts(expected), "{frames:?} since {last:?}");
        }
    }

    #[test]
    fn a_frame_noted_twice_counts_once() {
        let mut log = DamageLog::default();
        log.record(at(16), vec![rect(0, 0, 1, 1)]);
        log.record(at(16), vec![rect(9, 9, 1, 1)]);
        log.record(at(8), vec![rect(7, 7, 1, 1)]);
        assert_eq!(log.since(Some(at(0))), Since::Parts(vec![rect(0, 0, 1, 1)]));
    }

    #[test]
    fn a_client_older_than_the_log_gets_the_whole_area() {
        let mut log = DamageLog::default();
        for n in 1..=40_u64 {
            log.record(at(n * 16), vec![rect(0, 0, 1, 1)]);
        }
        assert_eq!(log.since(Some(at(16))), Since::Whole);
        let recent = log.since(Some(at(39 * 16)));
        assert_eq!(recent, Since::Parts(vec![rect(0, 0, 1, 1)]));
    }

    #[test]
    fn too_many_rectangles_become_their_bounding_box() {
        let mut log = DamageLog::default();
        let many = (0..20).map(|n| rect(n * 10, n * 5, 2, 2)).collect();
        log.record(at(16), many);
        assert_eq!(
            log.since(Some(at(0))),
            Since::Parts(vec![rect(0, 0, 192, 97)])
        );
    }

    #[test]
    fn damage_is_cut_to_the_area_and_moved_to_its_corner() {
        let area = rect(100, 50, 200, 100);
        let table = [
            (rect(0, 0, 50, 50), vec![]),
            (rect(90, 40, 20, 20), vec![rect(0, 0, 10, 10)]),
            (rect(150, 80, 10, 10), vec![rect(50, 30, 10, 10)]),
            (rect(290, 140, 50, 50), vec![rect(190, 90, 10, 10)]),
            (rect(300, 50, 10, 10), vec![]),
        ];
        for (damage, expected) in table {
            let told = Since::Parts(vec![damage]).within(area);
            assert_eq!(told, expected, "{damage:?}");
        }
        assert_eq!(Since::Whole.within(area), vec![rect(0, 0, 200, 100)]);
    }
}
