//! A new model is empty, unlocked and holds its settings and epoch.

mod support;

use casement_core::Model;
use casement_core::focus::Focus;
use casement_core::lock::LockState;
use casement_core::settings::CasementSettings;
use casement_core::space::SpaceSeed;
use support::ms;

#[test]
fn a_new_model_is_an_empty_desktop() {
    let model = Model::new(
        CasementSettings::defaults(),
        ms(5),
        SpaceSeed::from_entropy([1; 16]),
    );
    assert!(model.windows().is_empty());
    assert!(model.apps().is_empty());
    assert!(model.outputs().0.is_empty());
    assert_eq!(model.lock(), &LockState::Unlocked);
    assert_eq!(model.focus(), &Focus::none());
    assert_eq!(model.timers().next(), None);
    assert_eq!(model.settings(), &CasementSettings::defaults());
    assert_eq!(model.now(), ms(5));
}
