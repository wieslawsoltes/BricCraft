//! Construction gestures live on the native world visual, below the native HUD.
//! Each touch contact retains its own identity: a pinch never turns into a tap
//! when the first finger is lifted, and capture loss cannot strand a drag.
use crate::runtime::Runtime;
use briccraft_ui::UiAction;
use ferroui_base::input::{IPointer, KeyModifiers, PointerType};
use ferroui_base::Point;
use ferroui_controls::Border;
use ferroui_view::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

#[derive(Default)]
pub struct PointerState {
    pub down: bool,
    pub position: [f64; 2],
    pub start: [f64; 2],
    pub button: u8,
    pub dragged: bool,
    primary: Option<i32>,
    touches: BTreeMap<i32, [f64; 2]>,
    captures: BTreeMap<i32, Rc<dyn IPointer>>,
    was_multitouch: bool,
}

impl PointerState {
    /// Release captures after dropping the RefCell borrow: capture loss is synchronous.
    pub fn reset(&mut self) -> Vec<Rc<dyn IPointer>> {
        let captured = std::mem::take(&mut self.captures).into_values().collect();
        *self = Self::default();
        captured
    }

    fn cancel(&mut self, id: i32) {
        self.captures.remove(&id);
        self.touches.remove(&id);
        if self.primary == Some(id) {
            if let Some((&next, &point)) = self.touches.first_key_value() {
                self.primary = Some(next);
                self.position = point;
                self.start = point;
                self.dragged = true;
            } else {
                self.down = false;
                self.primary = None;
            }
        }
        if self.touches.is_empty() {
            self.was_multitouch = false;
        }
    }
}

fn pair(touches: &BTreeMap<i32, [f64; 2]>) -> Option<([f64; 2], f64)> {
    let mut contacts = touches.values();
    let a = contacts.next()?;
    let b = contacts.next()?;
    Some((
        [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5],
        (b[0] - a[0]).hypot(b[1] - a[1]),
    ))
}

fn pinch_delta(before: f64, after: f64) -> f32 {
    if before < 4.0 || after < 4.0 || !before.is_finite() || !after.is_finite() {
        return 0.0;
    }
    // Camera zoom is exponential; this inverse maps distance directly to finger spacing.
    (-(after / before).ln() / 0.0015).clamp(-600.0, 600.0) as f32
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
    border()
        .name("WorldViewport")
        .background("#7CABB7")
        .focusable(true)
        .node_ref(runtime.viewport)
        .effect(runtime.world_effect)
        .cache_mode(Some(
            runtime
                .world_cache
                .clone()
                .upcast::<ferroui_base::media::CacheMode>(),
        ))
        .on_pointer_pressed(move |event| {
            let Some(view) = pressed.viewport.get_untracked() else {
                return;
            };
            let point = event.get_position(Some(&view));
            let props = event.properties();
            let id = event.pointer().id();
            let touch = event.pointer().type_() == PointerType::Touch;
            let _ = view.focus();
            pick_at(&pressed, &view, point);
            {
                let mut state = pressed.pointer.borrow_mut();
                state.captures.insert(id, event.pointer().clone());
                if touch {
                    state.touches.insert(id, [point.x, point.y]);
                }
                if !state.down {
                    state.down = true;
                    state.primary = Some(id);
                    state.position = [point.x, point.y];
                    state.start = state.position;
                    state.dragged = false;
                    state.was_multitouch = false;
                    state.button = if props.is_right_button_pressed {
                        2
                    } else if props.is_middle_button_pressed {
                        1
                    } else {
                        0
                    };
                }
                if state.touches.len() > 1 {
                    state.was_multitouch = true;
                    state.dragged = true;
                }
            }
            event.pointer().capture(Some(&view.clone().upcast()));
            event.set_handled(true);
        })
        .on_pointer_moved(move |event| {
            let Some(view) = moved.viewport.get_untracked() else {
                return;
            };
            let point = event.get_position(Some(&view));
            let id = event.pointer().id();
            let mut gesture = None;
            let mut drag = None;
            {
                let mut state = moved.pointer.borrow_mut();
                let old = pair(&state.touches);
                if let Some(contact) = state.touches.get_mut(&id) {
                    *contact = [point.x, point.y];
                }
                if let (Some((before, old_distance)), Some((after, distance))) =
                    (old, pair(&state.touches))
                {
                    gesture = Some((
                        (after[0] - before[0]) as f32,
                        (after[1] - before[1]) as f32,
                        pinch_delta(old_distance, distance),
                    ));
                } else if state.down && state.primary == Some(id) {
                    let dx = (point.x - state.position[0]) as f32;
                    let dy = (point.y - state.position[1]) as f32;
                    if (point.x - state.start[0]).hypot(point.y - state.start[1]) > 5.0 {
                        state.dragged = true;
                    }
                    if state.dragged {
                        drag = Some((
                            dx,
                            dy,
                            state.button == 1
                                || event.key_modifiers().contains(KeyModifiers::SHIFT),
                        ));
                    }
                    state.position = [point.x, point.y];
                }
            }
            {
                let mut game = moved.session.borrow_mut();
                if let Some((dx, dy, delta)) = gesture {
                    game.camera.zoom(delta);
                    game.camera.pan(dx, dy);
                }
                if let Some((dx, dy, pan)) = drag {
                    if pan {
                        game.camera.pan(dx, dy);
                    } else {
                        game.camera.orbit(dx, dy);
                    }
                }
            }
            pick_at(&moved, &view, point);
            event.set_handled(true);
        })
        .on_pointer_released(move |event| {
            let id = event.pointer().id();
            let action = {
                let mut state = released.pointer.borrow_mut();
                let tap = state.down
                    && state.primary == Some(id)
                    && !state.dragged
                    && !state.was_multitouch;
                let action = tap.then_some(state.button);
                state.cancel(id);
                action
            };
            // A release may follow a press without a move event (mouse click or touch tap).
            if let Some(view) = released.viewport.get_untracked() {
                pick_at(&released, &view, event.get_position(Some(&view)));
            }
            event.pointer().capture(None);
            if let Some(button) = action {
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
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spreading_contacts_zooms_in_and_reverses_exactly() {
        assert!(pinch_delta(100.0, 150.0) < 0.0);
        assert!((pinch_delta(100.0, 150.0) + pinch_delta(150.0, 100.0)).abs() < 0.001);
        assert_eq!(pinch_delta(0.0, 50.0), 0.0);
    }
    #[test]
    fn lifting_one_finger_preserves_the_remaining_drag_without_placing() {
        let mut state = PointerState {
            down: true,
            primary: Some(1),
            was_multitouch: true,
            dragged: true,
            ..Default::default()
        };
        state.touches.insert(1, [10.0, 20.0]);
        state.touches.insert(2, [50.0, 60.0]);
        state.cancel(1);
        assert!(state.down && state.dragged && state.was_multitouch);
        assert_eq!(state.primary, Some(2));
        assert_eq!(state.position, [50.0, 60.0]);
        state.cancel(2);
        assert!(!state.down);
        assert!(!state.was_multitouch);
    }
    #[test]
    fn contact_pair_is_stable_under_event_order_and_tracks_centroid() {
        let contacts = BTreeMap::from([(9, [80.0, 20.0]), (2, [20.0, 20.0])]);
        assert_eq!(pair(&contacts), Some(([50.0, 20.0], 60.0)));
    }
}
