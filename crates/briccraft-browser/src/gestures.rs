//! Pointer identity and gesture intent, independent of the native view system.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraGesture {
    Orbit(f32, f32),
    Pan(f32, f32),
    Pinch { pan: [f32; 2], zoom: f32 },
}

#[derive(Default)]
pub struct GestureTracker {
    primary: Option<i32>,
    position: [f64; 2],
    start: [f64; 2],
    button: u8,
    touch: bool,
    dragged: bool,
    was_multitouch: bool,
    contacts: BTreeMap<i32, [f64; 2]>,
}

impl GestureTracker {
    pub fn press(&mut self, id: i32, point: [f64; 2], touch: bool, button: u8) {
        if touch {
            self.contacts.insert(id, point);
        }
        if self.primary.is_none() {
            self.primary = Some(id);
            self.position = point;
            self.start = point;
            self.button = button;
            self.touch = touch;
            self.dragged = false;
            self.was_multitouch = false;
        }
        if self.contacts.len() > 1 {
            self.was_multitouch = true;
            self.dragged = true;
        }
    }

    pub fn moved(&mut self, id: i32, point: [f64; 2], pan: bool) -> Option<CameraGesture> {
        if !point.into_iter().all(f64::is_finite) {
            return None;
        }
        let before = pair(&self.contacts);
        if let Some(contact) = self.contacts.get_mut(&id) {
            *contact = point;
        }
        if let (Some((old_center, old_distance)), Some((center, distance))) =
            (before, pair(&self.contacts))
        {
            return Some(CameraGesture::Pinch {
                pan: [
                    (center[0] - old_center[0]) as f32,
                    (center[1] - old_center[1]) as f32,
                ],
                zoom: pinch_delta(old_distance, distance),
            });
        }
        if self.primary != Some(id) {
            return None;
        }
        let was_dragged = self.dragged;
        let threshold = if self.touch { 8.0 } else { 4.0 };
        if (point[0] - self.start[0]).hypot(point[1] - self.start[1]) > threshold {
            self.dragged = true;
        }
        let origin = if was_dragged {
            self.position
        } else {
            self.start
        };
        let delta = [(point[0] - origin[0]) as f32, (point[1] - origin[1]) as f32];
        self.position = point;
        self.dragged.then_some(if pan || self.button == 1 {
            CameraGesture::Pan(delta[0], delta[1])
        } else {
            CameraGesture::Orbit(delta[0], delta[1])
        })
    }

    /// Touch taps aim only. Editing uses an explicit native control, so a
    /// camera adjustment or a long press can never accidentally change a build.
    pub fn release(&mut self, id: i32) -> Option<u8> {
        let edit = self.primary == Some(id) && !self.touch && !self.dragged && !self.was_multitouch;
        let button = edit.then_some(self.button);
        self.cancel(id);
        button
    }

    pub fn cancel(&mut self, id: i32) {
        self.contacts.remove(&id);
        if self.primary == Some(id) {
            if let Some((&next, &point)) = self.contacts.first_key_value() {
                self.primary = Some(next);
                self.position = point;
                self.start = point;
                self.dragged = true;
            } else {
                self.primary = None;
            }
        }
        if self.contacts.is_empty() {
            self.was_multitouch = false;
        }
    }
}

fn pair(contacts: &BTreeMap<i32, [f64; 2]>) -> Option<([f64; 2], f64)> {
    let mut points = contacts.values();
    let a = points.next()?;
    let b = points.next()?;
    Some((
        [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5],
        (b[0] - a[0]).hypot(b[1] - a[1]),
    ))
}

fn pinch_delta(before: f64, after: f64) -> f32 {
    if before < 4.0 || after < 4.0 || !before.is_finite() || !after.is_finite() {
        return 0.0;
    }
    (-(after / before).ln() / 0.0015).clamp(-600.0, 600.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_taps_and_small_finger_jitter_aim_without_editing() {
        let mut gesture = GestureTracker::default();
        gesture.press(1, [20.0, 30.0], true, 0);
        assert_eq!(gesture.moved(1, [23.0, 34.0], false), None);
        assert_eq!(gesture.release(1), None);
        gesture.press(2, [20.0, 30.0], false, 0);
        assert_eq!(gesture.release(2), Some(0));
    }

    #[test]
    fn pinch_lift_transitions_to_drag_without_a_camera_jump_or_edit() {
        let mut gesture = GestureTracker::default();
        gesture.press(1, [20.0, 30.0], true, 0);
        gesture.press(2, [100.0, 30.0], true, 0);
        let Some(CameraGesture::Pinch { pan, zoom }) = gesture.moved(2, [140.0, 30.0], false)
        else {
            panic!("pinch expected")
        };
        assert_eq!(pan, [20.0, 0.0]);
        assert!(zoom < 0.0);
        assert_eq!(gesture.release(1), None);
        assert_eq!(
            gesture.moved(2, [145.0, 33.0], false),
            Some(CameraGesture::Orbit(5.0, 3.0))
        );
        assert_eq!(gesture.release(2), None);
    }

    #[test]
    fn cancelled_contacts_cannot_strand_a_drag_or_trigger_an_edit() {
        let mut gesture = GestureTracker::default();
        gesture.press(1, [0.0, 0.0], false, 2);
        gesture.cancel(1);
        assert_eq!(gesture.release(1), None);
        assert_eq!(gesture.moved(1, [20.0, 20.0], false), None);
        gesture.press(2, [10.0, 10.0], false, 2);
        assert_eq!(gesture.release(2), Some(2));
    }

    #[test]
    fn zoom_is_reversible_and_invalid_contacts_are_ignored() {
        assert!((pinch_delta(100.0, 150.0) + pinch_delta(150.0, 100.0)).abs() < 0.001);
        assert_eq!(pinch_delta(0.0, 50.0), 0.0);
        let mut gesture = GestureTracker::default();
        gesture.press(1, [0.0, 0.0], true, 0);
        assert_eq!(gesture.moved(1, [f64::NAN, 3.0], false), None);
    }
}
