//! The data-to-wire derivations implemented at the freeze.

mod support;

use casement_core::WindowId;
use casement_core::app::{AppLife, QuitPhase};
use casement_core::space::{SpaceContent, SplitPair, TaskToken};
use casement_protocols::vocab::{AppLifeKind, SpaceKind};
use ds_core::vocab::Fraction;
use support::ms;

#[test]
fn app_lives_name_their_kind() {
    let cases = [
        (AppLife::Launching { since: ms(1) }, AppLifeKind::Launching),
        (AppLife::Running, AppLifeKind::Running),
        (AppLife::Windowless, AppLifeKind::Windowless),
        (
            AppLife::Quitting(QuitPhase::Stopping),
            AppLifeKind::Quitting,
        ),
        (
            AppLife::NotResponding { since: ms(2) },
            AppLifeKind::NotResponding,
        ),
    ];
    for (life, kind) in cases {
        assert_eq!(life.kind(), kind, "{life:?}");
    }
}

#[test]
fn space_contents_name_their_kind() {
    let pair = SplitPair {
        leading: WindowId(1),
        trailing: WindowId(2),
        divider: Fraction(500),
    };
    let cases = [
        (SpaceContent::Desktop, SpaceKind::Desktop),
        (SpaceContent::FullScreen(WindowId(1)), SpaceKind::FullScreen),
        (SpaceContent::SplitView(pair), SpaceKind::SplitView),
        (
            SpaceContent::Agent(TaskToken("t-1".to_owned())),
            SpaceKind::Agent,
        ),
    ];
    for (content, kind) in cases {
        assert_eq!(content.kind(), kind, "{content:?}");
    }
}
