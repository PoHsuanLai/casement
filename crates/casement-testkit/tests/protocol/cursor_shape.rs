//! `cursor_shape` (P0-H4).

use casement_core::OutputName;
use casement_core::geometry::{LogicalPoint, LogicalRect, LogicalSize};
use casement_core::input::DeviceClass;
use casement_core::policy::ConnectionRole;
use casement_testclient::seat::pointer::Pointer;
use casement_testclient::surface::cursor::{CursorShape, Shape};
use casement_testkit::{Fixture, FixtureSpec, FrameCount, ShotTarget, TestOrigin};

use crate::data_device::{focused, map};
use casement_testclient::surface::layer::Layer;
use std::time::Duration;

/// Where the pointer rests: over the middle of the layer the client maps.
const REST: LogicalPoint = LogicalPoint { x: 960, y: 540 };

/// A shot of the cursor: the square around the pointer's hotspot, where nothing but the
/// layer's flat colour and the cursor is drawn.
fn cursor_shot(fixture: &mut Fixture) -> image::RgbaImage {
    let output = OutputName::parse("HEADLESS-1").expect("a valid output name");
    fixture.advance(Duration::from_secs(1));
    fixture.frames(&output, FrameCount(2));
    let around = LogicalRect::new(
        LogicalPoint {
            x: REST.x - 32,
            y: REST.y - 32,
        },
        LogicalSize { w: 64, h: 64 },
    );
    fixture.shot(ShotTarget::Region(output, around))
}

/// Set_shape(text) with the pointer's enter serial sets the named cursor.
#[test]
fn a_shape_request_sets_the_named_cursor() {
    let mut fixture = Fixture::start(FixtureSpec::desktop()).expect("fixture");
    let mouse = fixture.input().plug(
        DeviceClass::Pointer,
        TestOrigin::Physical(DeviceClass::Pointer),
    );
    let id = fixture.connect(ConnectionRole::Public);
    fixture.pump();
    let pointer = Pointer::bind(fixture.client(id)).expect("a pointer");
    let shapes = CursorShape::bind(fixture.client(id), pointer.pointer()).expect("a shape device");
    let _layer = map(&mut fixture, id, &focused(Layer::Top, "canvas"));
    fixture.input().pointer_to(mouse, REST);
    let serial = pointer
        .enter_serial()
        .expect("the pointer entered the layer");
    let arrow = cursor_shot(&mut fixture);
    let first = arrow.get_pixel(0, 0);
    assert!(
        arrow.pixels().any(|pixel| pixel != first),
        "the default arrow is drawn over the layer"
    );

    // The named cursor replaces the arrow.
    shapes.set_shape(fixture.client(id), serial, Shape::Text);
    fixture.pump();
    let text = cursor_shot(&mut fixture);
    assert_ne!(text, arrow, "the text cursor is not the arrow");

    // And another name another picture.
    shapes.set_shape(fixture.client(id), serial, Shape::Pointer);
    fixture.pump();
    let hand = cursor_shot(&mut fixture);
    assert_ne!(hand, text, "the pointing hand is not the text cursor");
    assert_ne!(hand, arrow, "the pointing hand is not the arrow");

    // A serial that is not the enter's changes nothing: the request is stale.
    shapes.set_shape(
        fixture.client(id),
        serial.wrapping_add(1000),
        Shape::Default,
    );
    fixture.pump();
    assert_eq!(cursor_shot(&mut fixture), hand, "a stale serial is ignored");
    assert!(fixture.client(id).protocol_error().is_none());

    // The default name brings the arrow back, pixel for pixel.
    shapes.set_shape(fixture.client(id), serial, Shape::Default);
    fixture.pump();
    assert_eq!(
        cursor_shot(&mut fixture),
        arrow,
        "default is the arrow again"
    );
}
