//! Construction gestures live on the native world visual, below the native HUD.
//! Each touch contact retains its own identity: a pinch never turns into a tap
//! when the first finger is lifted, and capture loss cannot strand a drag.
use crate::gestures::{CameraGesture, GestureTracker};
use crate::runtime::Runtime;
use briccraft_ui::UiAction;
use ferroui_base::input::{IPointer, KeyModifiers, PointerType};
use ferroui_base::Point;
use ferroui_controls::Border;
use ferroui_view::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

#[derive(Default)]
pub struct PointerState {
    gesture: GestureTracker,
    captures: BTreeMap<i32, Rc<dyn IPointer>>,
}

impl PointerState {
    /// Capture loss is synchronous, so callers release after dropping the borrow.
    pub fn reset(&mut self) -> Vec<Rc<dyn IPointer>> {
        let captured = std::mem::take(&mut self.captures).into_values().collect();
        self.gesture = GestureTracker::default();
        captured
    }

    fn cancel(&mut self, id: i32) {
        self.captures.remove(&id);
        self.gesture.cancel(id);
    }
}

fn pick_at(runtime: &Runtime, view: &Border, point: Point) {
    let size = view.bounds();
    let mut game = runtime.session.borrow_mut();
    game.cursor = [
        (point.x / size.width.max(1.0) * 2.0 - 1.0) as f32,
        (1.0 - point.y / size.height.max(1.0) * 2.0) as f32,
    ];
    game.update_pick();
}

pub fn view(runtime: Rc<Runtime>) -> Element {
    let pressed = runtime.clone();
    let moved = runtime.clone();
    let released = runtime.clone();
    let wheel = runtime.clone();
    let lost = runtime.clone();
    let scene: Element = border()
        .name("WorldViewport")
        .background("#7CABB7")
        .focusable(true)
        .node_ref(runtime.viewport)
        .effect(runtime.world_effect)
        .on_pointer_pressed(move |event| {
            if pressed.state.modal_untracked() {
                return;
            }
            let Some(view) = pressed.viewport.get_untracked() else {
                return;
            };
            let point = event.get_position(Some(&view));
            let props = event.properties();
            let id = event.pointer().id();
            let touch = event.pointer().type_() == PointerType::Touch;
            let _ = view.focus();
            pick_at(&pressed, &view, point);
            if touch {
                pressed.state.touch.set_if_changed(true);
            }
            {
                let button = if props.is_right_button_pressed {
                    2
                } else if props.is_middle_button_pressed {
                    1
                } else {
                    0
                };
                let mut state = pressed.pointer.borrow_mut();
                state.captures.insert(id, event.pointer().clone());
                state.gesture.press(id, [point.x, point.y], touch, button);
            }
            event.pointer().capture(Some(&view.clone().upcast()));
            event.set_handled(true);
        })
        .on_pointer_moved(move |event| {
            if moved.state.modal_untracked() {
                return;
            }
            let Some(view) = moved.viewport.get_untracked() else {
                return;
            };
            let point = event.get_position(Some(&view));
            let id = event.pointer().id();
            let gesture = moved.pointer.borrow_mut().gesture.moved(
                id,
                [point.x, point.y],
                event.key_modifiers().contains(KeyModifiers::SHIFT),
            );
            if let Some(gesture) = gesture {
                let mut game = moved.session.borrow_mut();
                match gesture {
                    CameraGesture::Pinch { pan, zoom } => {
                        game.camera.zoom(zoom);
                        game.camera.pan(pan[0], pan[1]);
                    }
                    CameraGesture::Pan(dx, dy) => game.camera.pan(dx, dy),
                    CameraGesture::Orbit(dx, dy) => game.camera.orbit(dx, dy),
                }
            }
            pick_at(&moved, &view, point);
            event.set_handled(true);
        })
        .on_pointer_released(move |event| {
            let id = event.pointer().id();
            let action = {
                let mut state = released.pointer.borrow_mut();
                state.captures.remove(&id);
                state.gesture.release(id)
            };
            // A release may follow a press without a move event (mouse click or touch tap).
            if let Some(view) = released.viewport.get_untracked() {
                pick_at(&released, &view, event.get_position(Some(&view)));
            }
            event.pointer().capture(None);
            if let Some(button) = action.filter(|_| !released.state.modal_untracked()) {
                match button {
                    0 => released.dispatch(UiAction::Place),
                    2 => released.dispatch(UiAction::Remove),
                    _ => {
                        released.session.borrow_mut().eyedropper();
                        released.sync_hud();
                    }
                }
            }
            event.set_handled(true);
        })
        .on_pointer_capture_lost(move |event| {
            lost.pointer.borrow_mut().cancel(event.pointer().id())
        })
        .on_pointer_wheel_changed(move |event| {
            if wheel.state.modal_untracked() {
                return;
            }
            let delta = event.delta();
            let mut game = wheel.session.borrow_mut();
            if event.key_modifiers().contains(KeyModifiers::SHIFT)
                || delta.x.abs() > delta.y.abs() * 0.25
            {
                game.camera
                    .pan((delta.x * 30.0) as f32, (-delta.y * 30.0) as f32);
            } else {
                game.camera.zoom((-delta.y * 60.0) as f32);
            }
            game.update_pick();
            event.set_handled(true);
        })
        .into();

    // Keep the changing effect inside the cached parent. Its dirty rectangle
    // is collected while the parent's cache is active; caching the effect
    // visual itself misses that invalidation in the pinned compositor.
    border()
        .name("WorldResolutionLayer")
        .clip_to_bounds(true)
        .cache_mode(Some(
            runtime
                .world_cache
                .clone()
                .upcast::<ferroui_base::media::CacheMode>(),
        ))
        .child(scene)
        .into()
}
