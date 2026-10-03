//! What a toplevel asks for through `Model::step`: placement, grabs, the dock, full screen and
//! activation. Every row runs with the invariants checked after each step.

mod support;

use casement_core::focus::TokenCheck;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::window::WindowState;
use casement_core::{
    Cause, CoreNote, Effect, GrabSpec, InputSerial, ResizeEdges, ToplevelRequest, WindowId,
};
use casement_protocols::vocab::{Attention, PlaceTarget, TileZone, Toggle};
use support::driver::{Run, last, map, plug, request, run};
use support::proven;

fn state_of(run: &Run, n: u64) -> WindowState {
    run.model.windows()[&WindowId(n)].state.clone()
}

fn size(w: u32, h: u32) -> LogicalSize {
    LogicalSize { w, h }
}

#[test]
fn requests_move_a_window_between_its_states() {
    use PlaceTarget as P;
    use WindowState as S;
    let centred = LogicalRect::new(LogicalPoint { x: 560, y: 240 }, size(800, 600));
    let cases: Vec<(&str, Vec<ToplevelRequest>, WindowState)> = vec![
        (
            "maximise an unplaced window fills, restoring to its centred rectangle",
            vec![ToplevelRequest::Maximise(Toggle::On)],
            S::Filled { restore: centred },
        ),
        (
            "maximise off restores it floating",
            vec![
                ToplevelRequest::Maximise(Toggle::On),
                ToplevelRequest::Maximise(Toggle::Off),
            ],
            S::Floating(centred),
        ),
        (
            "tile left",
            vec![ToplevelRequest::Place {
                target: P::Left,
                serial: InputSerial(1),
            }],
            S::Tiled {
                zone: TileZone::Left,
                restore: centred,
            },
        ),
        (
            "tile then fill keeps the first restore",
            vec![
                ToplevelRequest::Place {
                    target: P::BottomRight,
                    serial: InputSerial(1),
                },
                ToplevelRequest::Place {
                    target: P::Fill,
                    serial: InputSerial(2),
                },
            ],
            S::Filled { restore: centred },
        ),
        (
            "restore from a tile",
            vec![
                ToplevelRequest::Place {
                    target: P::Top,
                    serial: InputSerial(1),
                },
                ToplevelRequest::Place {
                    target: P::Restore,
                    serial: InputSerial(2),
                },
            ],
            S::Floating(centred),
        ),
        (
            "centre keeps the size",
            vec![ToplevelRequest::Place {
                target: P::Centre,
                serial: InputSerial(1),
            }],
            S::Floating(centred),
        ),
        (
            "restore of an unplaced window changes nothing",
            vec![ToplevelRequest::Place {
                target: P::Restore,
                serial: InputSerial(1),
            }],
            S::New(size(800, 600)),
        ),
        (
            "full screen and back",
            vec![
                ToplevelRequest::FullScreen {
                    toggle: Toggle::On,
                    output: None,
                },
                ToplevelRequest::FullScreen {
                    toggle: Toggle::Off,
                    output: None,
                },
            ],
            S::New(size(800, 600)),
        ),
    ];
    for (name, requests, want) in cases {
        let mail = proven("org.quire.Mail");
        let mut ops = vec![plug("HEADLESS-1"), map(1, &mail, TokenCheck::Valid)];
        ops.extend(requests.into_iter().map(|r| request(1, r)));
        let run = run(ops);
        assert_eq!(state_of(&run, 1), want, "{name}");
    }
}

#[test]
fn a_full_screen_window_is_not_placed() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        request(
            1,
            ToplevelRequest::FullScreen {
                toggle: Toggle::On,
                output: None,
            },
        ),
        request(
            1,
            ToplevelRequest::Place {
                target: PlaceTarget::Left,
                serial: InputSerial(1),
            },
        ),
        request(1, ToplevelRequest::Maximise(Toggle::On)),
        request(1, ToplevelRequest::Minimise),
    ]);
    assert!(matches!(
        state_of(&run, 1),
        WindowState::FullScreen {
            restore: casement_core::window::Restore::New(_),
            ..
        }
    ));
}

#[test]
fn a_move_or_resize_starts_a_grab_and_frees_a_tiled_window() {
    let mail = proven("org.quire.Mail");
    let tiled = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        request(
            1,
            ToplevelRequest::Place {
                target: PlaceTarget::Left,
                serial: InputSerial(1),
            },
        ),
        request(
            1,
            ToplevelRequest::Move {
                serial: InputSerial(7),
            },
        ),
    ]);
    assert!(last(&tiled).contains(&Effect::StartGrab(GrabSpec::Move {
        window: WindowId(1),
        serial: InputSerial(7)
    })));
    assert!(matches!(state_of(&tiled, 1), WindowState::Floating(_)));

    let resized = support::driver::run_from(
        tiled.model,
        vec![request(
            1,
            ToplevelRequest::Resize {
                serial: InputSerial(8),
                edges: ResizeEdges::Left,
            },
        )],
    );
    assert!(
        last(&resized).contains(&Effect::StartGrab(GrabSpec::Resize {
            window: WindowId(1),
            serial: InputSerial(8),
            edges: ResizeEdges::Left
        }))
    );
}

#[test]
fn a_move_request_from_a_full_screen_window_starts_nothing() {
    let mail = proven("org.quire.Mail");
    let run = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        request(
            1,
            ToplevelRequest::FullScreen {
                toggle: Toggle::On,
                output: None,
            },
        ),
        request(
            1,
            ToplevelRequest::Move {
                serial: InputSerial(7),
            },
        ),
    ]);
    assert!(!last(&run).iter().any(|e| matches!(e, Effect::StartGrab(_))));
}

#[test]
fn minimising_the_key_window_moves_the_key_and_activation_brings_it_back() {
    let mail = proven("org.quire.Mail");
    let key = support::headless("HEADLESS-1").key;
    let minimised = run(vec![
        plug("HEADLESS-1"),
        map(1, &mail, TokenCheck::Valid),
        map(2, &mail, TokenCheck::Valid),
        request(2, ToplevelRequest::Minimise),
    ]);
    assert!(matches!(
        state_of(&minimised, 2),
        WindowState::Minimised { .. }
    ));
    assert_eq!(minimised.model.focus().key, Some(WindowId(1)));
    assert!(last(&minimised).iter().any(|e| matches!(
        e,
        Effect::Relayout(Cause::Minimise(target)) if target.output == key
    )));

    let back = support::driver::run_from(
        minimised.model,
        vec![request(2, ToplevelRequest::Activate(TokenCheck::Valid))],
    );
    assert!(matches!(state_of(&back, 2), WindowState::New(_)));
    assert_eq!(back.model.focus().key, Some(WindowId(2)));
}

#[test]
fn an_activation_without_a_valid_token_asks_for_attention_and_is_noted() {
    let (mail, web) = (proven("org.quire.Mail"), proven("org.example.Web"));
    for token in [TokenCheck::Stale, TokenCheck::Missing] {
        let run = run(vec![
            plug("HEADLESS-1"),
            map(1, &mail, TokenCheck::Valid),
            map(2, &web, TokenCheck::Missing),
            request(2, ToplevelRequest::Activate(token)),
        ]);
        assert_eq!(run.model.focus().key, Some(WindowId(1)), "{token:?}");
        let effects = last(&run);
        assert!(
            effects.contains(&Effect::Note(CoreNote::RefusedActivation(WindowId(2)))),
            "{token:?}"
        );
        assert!(
            effects.iter().any(|e| matches!(
                e,
                Effect::Attention {
                    level: Attention::Informational,
                    ..
                }
            )) || run.model.apps()[&web].attention == Attention::Informational,
            "{token:?}"
        );
    }
}
