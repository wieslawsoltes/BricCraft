//! Map-specific pointer accounting: dragging/pinching never becomes a location tap.
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct Contact {
    start: [f64; 2],
    point: [f64; 2],
    tap: bool,
}

#[derive(Default)]
pub struct MapGestures {
    contacts: BTreeMap<i32, Contact>,
    pinch_log: f64,
}

pub struct MapMovement {
    pub pan: [f64; 2],
    pub anchor: [f64; 2],
    pub zoom: i8,
}

/// Framework wheel deltas are notches, including fractional trackpad movement.
#[derive(Default)]
pub struct MapWheel {
    remainder: f64,
}

impl MapWheel {
    pub fn consume(&mut self, delta: f64) -> i8 {
        if !delta.is_finite() {
            return 0;
        }
        let mut accumulated = (self.remainder + delta.clamp(-4.0, 4.0)).clamp(-4.0, 4.0);
        if (accumulated - accumulated.round()).abs() < 1e-9 {
            accumulated = accumulated.round();
        }
        let steps = accumulated.trunc() as i8;
        self.remainder = accumulated - f64::from(steps);
        steps
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn pair(contacts: &BTreeMap<i32, Contact>) -> Option<([f64; 2], f64)> {
    let mut values = contacts.values();
    let a = values.next()?.point;
    let b = values.next()?.point;
    Some(([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5], distance(a, b)))
}

impl MapGestures {
    pub fn press(&mut self, id: i32, point: [f64; 2]) {
        self.contacts.insert(
            id,
            Contact {
                start: point,
                point,
                tap: true,
            },
        );
        self.pinch_log = 0.0;
        if self.contacts.len() > 1 {
            for contact in self.contacts.values_mut() {
                contact.tap = false;
            }
        }
    }

    pub fn moved(&mut self, id: i32, point: [f64; 2]) -> Option<MapMovement> {
        let old_pair = pair(&self.contacts);
        let contact = self.contacts.get_mut(&id)?;
        let old_point = contact.point;
        contact.point = point;
        if distance(contact.start, point) > 5.0 {
            contact.tap = false;
        }
        let tap = contact.tap;
        if let (Some((old_center, old_distance)), Some((center, new_distance))) =
            (old_pair, pair(&self.contacts))
        {
            if old_distance > 8.0 && new_distance > 8.0 {
                self.pinch_log += (new_distance / old_distance).log2();
            }
            // Round to the closest logical zoom, carrying the residual through the gesture.
            let zoom = if self.pinch_log >= 0.5 {
                1
            } else if self.pinch_log <= -0.5 {
                -1
            } else {
                0
            };
            self.pinch_log -= f64::from(zoom);
            return Some(MapMovement {
                pan: [center[0] - old_center[0], center[1] - old_center[1]],
                anchor: center,
                zoom,
            });
        }
        (!tap).then_some(MapMovement {
            pan: [point[0] - old_point[0], point[1] - old_point[1]],
            anchor: point,
            zoom: 0,
        })
    }

    pub fn release(&mut self, id: i32) -> bool {
        self.pinch_log = 0.0;
        self.contacts.remove(&id).is_some_and(|contact| contact.tap)
    }

    pub fn cancel(&mut self, id: i32) {
        self.contacts.remove(&id);
        self.pinch_log = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_and_pinch_releases_do_not_select_places() {
        let mut input = MapGestures::default();
        input.press(1, [100.0, 100.0]);
        assert!(input.moved(1, [150.0, 100.0]).is_some());
        assert!(!input.release(1));
        input.press(1, [100.0, 100.0]);
        input.press(2, [160.0, 100.0]);
        assert_eq!(input.moved(2, [200.0, 100.0]).unwrap().zoom, 1);
        assert!(!input.release(2));
        assert!(input.moved(1, [105.0, 100.0]).is_some());
        assert!(!input.release(1));
        input.press(3, [80.0, 70.0]);
        assert!(input.release(3));
    }

    #[test]
    fn capture_loss_cannot_complete_a_tap() {
        let mut input = MapGestures::default();
        input.press(2, [20.0, 30.0]);
        input.cancel(2);
        assert!(!input.release(2));
        assert!(input.moved(2, [100.0, 100.0]).is_none());
    }

    #[test]
    fn trackpad_fractional_wheel_events_accumulate_without_zoom_jumps() {
        let mut wheel = MapWheel::default();
        let mut total = 0;
        for _ in 0..20 {
            total += wheel.consume(0.05);
        }
        assert_eq!(total, 1);
        assert_eq!(wheel.consume(1.0), 1);
        assert_eq!(wheel.consume(-1.0), -1);
        assert_eq!(wheel.consume(f64::NAN), 0);
        assert_eq!(wheel.consume(f64::INFINITY), 0);
    }
}
