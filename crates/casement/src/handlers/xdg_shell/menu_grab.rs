//! A menu's pointer grab: a press on anything that is not one of the menu's own popups dismisses
//! the whole chain, whichever client owns what was pressed, and the press goes nowhere (a Mac
//! menu). smithay's grab dismisses only on another client's surface, so a shell's menu stayed up
//! over a press on its own dock, bar or desktop.

use crate::state::State;
use smithay::backend::input::ButtonState;
use smithay::desktop::{
    PopupGrab, PopupManager, PopupPointerGrab, PopupUngrabStrategy, find_popup_root_surface,
};
use smithay::input::SeatHandler;
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent, GesturePinchBeginEvent,
    GesturePinchEndEvent, GesturePinchUpdateEvent, GestureSwipeBeginEvent, GestureSwipeEndEvent,
    GestureSwipeUpdateEvent, GrabStartData, MotionEvent, PointerGrab, PointerInnerHandle,
    RelativeMotionEvent,
};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Point};
use smithay::wayland::compositor::get_parent;
use smithay::wayland::seat::WaylandFocus;

type Focus = <State as SeatHandler>::PointerFocus;

/// Where the grab is: the menu is up, or a press outside dismissed it and the grab stays only to
/// swallow that button's release.
enum Phase {
    Open,
    Swallowing(u32),
}

pub(super) struct MenuPointerGrab {
    inner: PopupPointerGrab<State>,
    popups: PopupGrab<State>,
    root: WlSurface,
    phase: Phase,
}

impl MenuPointerGrab {
    /// The grab of `popups`, whose chain hangs from `root`.
    pub(super) fn new(popups: &PopupGrab<State>, root: WlSurface) -> Self {
        MenuPointerGrab {
            inner: PopupPointerGrab::new(popups),
            popups: popups.clone(),
            root,
            phase: Phase::Open,
        }
    }
}

/// Whether `surface` (or the surface it is a subsurface of) is a popup hanging from `root`.
fn in_menu(manager: &PopupManager, root: &WlSurface, surface: &WlSurface) -> bool {
    let mut top = surface.clone();
    while let Some(parent) = get_parent(&top) {
        top = parent;
    }
    manager
        .find_popup(&top)
        .and_then(|popup| find_popup_root_surface(&popup).ok())
        .is_some_and(|found| &found == root)
}

impl PointerGrab<State> for MenuPointerGrab {
    fn motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        focus: Option<(Focus, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        match self.phase {
            Phase::Open => self.inner.motion(data, handle, focus, event),
            Phase::Swallowing(_) => handle.motion(data, focus, event),
        }
    }

    fn relative_motion(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        focus: Option<(Focus, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &ButtonEvent,
    ) {
        match self.phase {
            Phase::Swallowing(button) => {
                if event.button == button && event.state == ButtonState::Released {
                    handle.unset_grab(self, data, event.serial, event.time, true);
                }
            }
            // The client closed its menu itself (Escape: it destroys the popup and no
            // popup_done is sent), and the grab is still here: the grab ends and this press is
            // the person's next click, so it goes where it lands. smithay's grab would end at
            // this button and drop it, so the first click after the menu needed a second.
            Phase::Open if self.popups.has_ended() => {
                handle.unset_grab(self, data, event.serial, event.time, true);
                handle.button(data, event);
            }
            Phase::Open => {
                let outside = event.state == ButtonState::Pressed
                    && !handle
                        .current_focus()
                        .and_then(|(focus, _)| focus.wl_surface().map(|s| s.into_owned()))
                        .is_some_and(|surface| {
                            in_menu(&data.wl.xdg_shell.popups, &self.root, &surface)
                        });
                if !outside {
                    self.inner.button(data, handle, event);
                    return;
                }
                let _ = self.popups.ungrab(PopupUngrabStrategy::All);
                // The keyboard grab ends now, with the menu; the pointer's waits for the release.
                self.inner.unset(data);
                self.phase = Phase::Swallowing(event.button);
            }
        }
    }

    fn axis(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        details: AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn frame(&mut self, data: &mut State, handle: &mut PointerInnerHandle<'_, State>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event);
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event);
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event);
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event);
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event);
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event);
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event);
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut State,
        handle: &mut PointerInnerHandle<'_, State>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event);
    }

    fn start_data(&self) -> &GrabStartData<State> {
        self.inner.start_data()
    }

    fn unset(&mut self, data: &mut State) {
        if let Phase::Open = self.phase {
            self.inner.unset(data);
        }
    }
}
