//! `xdg_toplevel_icon` (P0-H2). Ignored until the fixture and the core loop are filled.

use crate::xdg_shell::boot;
use casement_core::geometry::PhysicalSize;
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::surface::toplevel::extras::{IconError, IconManager};
use casement_testclient::{Pattern, shm_buffer};

/// A toplevel icon with a buffer is accepted without error.
#[test]
#[ignore = "P0-B-headless: Fixture::start; P0-B-loop: core_loop, exec"]
fn an_icon_is_accepted() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let icons = IconManager::bind(fixture.client(id));
    let top = base.toplevel(fixture.client(id));
    let buffer = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 32, h: 32 },
        Pattern::Gradient,
    );
    let _icon = icons.set_icon(fixture.client(id), &top, &buffer, 1);
    top.commit_bare();
    fixture.pump();
    assert!(fixture.client(id).protocol_error().is_none());
    // The manager said `done` after its (possibly empty) list of sizes, as bound.
    assert_eq!(icons.log.with(|seen| seen.done), 1);
}

/// A buffer that is not square is `invalid_buffer`; a request on an icon already set is
/// `immutable`; a buffer destroyed before the icon is `no_buffer`.
#[test]
#[ignore = "P0-B-headless: Fixture::start; P0-B-loop: core_loop, exec"]
fn the_documented_icon_errors() {
    let rows = [
        ("non-square", IconError::InvalidBuffer),
        ("set after set_icon", IconError::Immutable),
        ("buffer destroyed first", IconError::NoBuffer),
    ];
    for (case, want) in rows {
        let (mut fixture, id) = boot();
        let base = WmBase::bind(fixture.client(id), 7);
        let icons = IconManager::bind(fixture.client(id));
        let top = base.toplevel(fixture.client(id));
        match want {
            IconError::InvalidBuffer => {
                let wide = shm_buffer(
                    fixture.client(id),
                    PhysicalSize { w: 32, h: 16 },
                    Pattern::Gradient,
                );
                let icon = icons.manager.create_icon(&fixture.client(id).qh(), ());
                icon.add_buffer(wide.buffer(), 1);
            }
            IconError::Immutable => {
                let square = shm_buffer(
                    fixture.client(id),
                    PhysicalSize { w: 32, h: 32 },
                    Pattern::Gradient,
                );
                let icon = icons.set_icon(fixture.client(id), &top, &square, 1);
                icon.set_name("late".to_owned());
            }
            _ => {
                let square = shm_buffer(
                    fixture.client(id),
                    PhysicalSize { w: 32, h: 32 },
                    Pattern::Gradient,
                );
                let icon = icons.manager.create_icon(&fixture.client(id).qh(), ());
                icon.add_buffer(square.buffer(), 1);
                square.buffer().destroy();
            }
        }
        fixture.pump();
        let error = fixture
            .client(id)
            .protocol_error()
            .expect("a protocol error");
        assert_eq!(error.code, want as u32, "{case}");
    }
}
