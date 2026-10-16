//! `xdg_toplevel_icon` (P0-H2). Ignored until the fixture and the core loop are filled.

use crate::protocol::xdg_shell::boot;
use casement_core::geometry::PhysicalSize;
use casement_testclient::surface::toplevel::WmBase;
use casement_testclient::surface::toplevel::extras::{IconError, IconManager};
use casement_testclient::{Pattern, shm_buffer};

/// A toplevel icon with a buffer is accepted without error.
#[test]
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

/// A mistake after `immutable` changes nothing and aborts nothing: the client is disconnected
/// with the one error, and the compositor goes on serving the next client.
#[test]
fn a_refused_request_leaves_the_compositor_serving() {
    let (mut fixture, id) = boot();
    let base = WmBase::bind(fixture.client(id), 7);
    let icons = IconManager::bind(fixture.client(id));
    let top = base.toplevel(fixture.client(id));
    let square = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 32, h: 32 },
        Pattern::Gradient,
    );
    let icon = icons.set_icon(fixture.client(id), &top, &square, 1);
    icon.set_name("late".to_owned());
    icon.add_buffer(square.buffer(), 2);
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.code, IconError::Immutable as u32);
    let other = fixture.connect(casement_core::policy::ConnectionRole::Public);
    fixture.pump();
    assert!(fixture.client(other).protocol_error().is_none());
}

/// A second buffer of the same size and scale replaces the first, which may then go away; the
/// one that stays must outlive the icon (`no_buffer`).
#[test]
fn a_replaced_buffer_may_be_destroyed_and_the_kept_one_may_not() {
    let (mut fixture, id) = boot();
    let icons = IconManager::bind(fixture.client(id));
    let make = |fixture: &mut casement_testkit::Fixture| {
        shm_buffer(
            fixture.client(id),
            PhysicalSize { w: 32, h: 32 },
            Pattern::Gradient,
        )
    };
    let (first, second) = (make(&mut fixture), make(&mut fixture));
    let icon = icons.manager.create_icon(&fixture.client(id).qh(), ());
    icon.add_buffer(first.buffer(), 1);
    icon.add_buffer(second.buffer(), 1);
    first.buffer().destroy();
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "the replaced buffer is nobody's"
    );
    second.buffer().destroy();
    fixture.pump();
    let error = fixture
        .client(id)
        .protocol_error()
        .expect("a protocol error");
    assert_eq!(error.code, IconError::NoBuffer as u32);
}

/// CASW-06: a client may hold `MAX_ICON_BUFFERS` icon buffers (every buffer destroy of every
/// client scans the table they sit in); the next `add_buffer` cuts it with
/// `wl_display.no_memory` (code 2).
#[test]
fn a_client_that_adds_icon_buffers_without_end_is_cut() {
    let (mut fixture, id) = boot();
    let icons = IconManager::bind(fixture.client(id));
    let square = shm_buffer(
        fixture.client(id),
        PhysicalSize { w: 32, h: 32 },
        Pattern::Gradient,
    );
    let mut made = Vec::new();
    for _ in 0..casement_wayland::limits::MAX_ICON_BUFFERS {
        let icon = icons.manager.create_icon(&fixture.client(id).qh(), ());
        icon.add_buffer(square.buffer(), 1);
        made.push(icon);
    }
    fixture.pump();
    assert!(
        fixture.client(id).protocol_error().is_none(),
        "up to the cap is allowed"
    );
    let icon = icons.manager.create_icon(&fixture.client(id).qh(), ());
    icon.add_buffer(square.buffer(), 1);
    made.push(icon);
    fixture.pump();
    // The cut closes the connection (the system backend sends no error first).
    assert!(
        !fixture.client(id).is_connected(),
        "the client past the cap is cut"
    );
}
