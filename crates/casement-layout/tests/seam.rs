//! The seam itself: any SceneLayout serves a consumer through `&dyn SceneLayout`.

mod support;

use casement_core::WindowId;
use casement_layout::{Arrangement, ConfigureNeed, SceneLayout, WindowTarget};
use support::fixed_scene::FixedScene;
use support::scene::{knobs, one_desktop, output_key, rect};

fn consume(layout: &dyn SceneLayout) -> Arrangement {
    layout.arrange(&one_desktop(rect(0, 0, 800, 600), Vec::new()), &knobs())
}

#[test]
fn a_fixed_scene_answers_through_the_trait_object() {
    let fixed = Arrangement {
        windows: vec![WindowTarget {
            window: WindowId(7),
            output: output_key("HEADLESS-1"),
            rect: rect(1, 2, 3, 4),
            size: ConfigureNeed::Keep,
        }],
        ..Arrangement::default()
    };
    assert_eq!(consume(&FixedScene(fixed.clone())), fixed);
    assert_eq!(
        consume(&FixedScene(Arrangement::default())),
        Arrangement::default()
    );
}
