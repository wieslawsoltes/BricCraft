use crate::effect::{BalancedBrickWorldEffect, FastBrickWorldEffect};
use crate::BrickWorldEffect;
use ferroui_base::media::effects::{IEffect, ImmutableCustomEffect};
use std::rc::Rc;

/// Retains only the latest complete snapshot. Image handles compare by identity;
/// every uniform is covered by BrickWorldEffect's derived PartialEq.
#[derive(Default)]
pub(crate) struct EffectCache {
    current: Option<(BrickWorldEffect, Rc<dyn IEffect>)>,
    reused: u64,
}

impl EffectCache {
    pub fn resolve(&mut self, value: BrickWorldEffect) -> (Rc<dyn IEffect>, bool) {
        if let Some((previous, effect)) = &self.current {
            if previous == &value {
                self.reused += 1;
                return (effect.clone(), false);
            }
        }
        let effect: Rc<dyn IEffect> = match value.quality {
            0 => ImmutableCustomEffect::new(FastBrickWorldEffect(value.clone())).into(),
            1 => ImmutableCustomEffect::new(BalancedBrickWorldEffect(value.clone())).into(),
            _ => ImmutableCustomEffect::new(value.clone()).into(),
        };
        self.current = Some((value, effect.clone()));
        (effect, true)
    }

    pub fn invalidate(&mut self) {
        self.current = None;
    }

    pub fn reused(&self) -> u64 {
        self.reused
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{Rect, Size};

    #[test]
    fn stationary_world_reuses_identity_until_explicit_invalidation() {
        let value = BrickWorldEffect {
            quality: 1,
            sample_count: 1,
            ..Default::default()
        };
        let mut cache = EffectCache::default();
        let (first, published) = cache.resolve(value.clone());
        assert!(published);
        let (same, published) = cache.resolve(value.clone());
        assert!(!published);
        assert!(Rc::ptr_eq(&first, &same));
        assert_eq!(cache.reused(), 1);
        cache.invalidate();
        let (fresh, published) = cache.resolve(value);
        assert!(published);
        assert!(!Rc::ptr_eq(&first, &fresh));
    }

    #[test]
    fn visible_changes_publish_a_new_snapshot_then_stabilize() {
        // Covers the values whose changes can otherwise be hidden by a stale
        // retained world layer, including overlay edits and moving assemblies.
        let changes: [fn(&mut BrickWorldEffect); 10] = [
            |v| v.camera_position = Rect::new(1.0, 2.0, 3.0, 4.0),
            |v| v.camera_forward = Rect::new(0.0, 0.0, 1.0, 0.5),
            |v| v.camera_up = Rect::new(0.0, 1.0, 0.0, 0.3),
            |v| v.selection = 23,
            |v| v.preview_origin = Rect::new(4.0, 5.0, 6.0, 1.0),
            |v| v.vehicle_position = Rect::new(7.0, 8.0, 9.0, 1.0),
            |v| v.vehicle_motion = Size::new(0.7, 1.3),
            |v| v.quality = 2,
            |v| v.sample_count = 4,
            |v| v.construction_grid = 1,
        ];
        for change in changes {
            let value = BrickWorldEffect {
                quality: 1,
                sample_count: 1,
                ..Default::default()
            };
            let mut cache = EffectCache::default();
            let (before, _) = cache.resolve(value.clone());
            let mut changed = value;
            change(&mut changed);
            let (after, published) = cache.resolve(changed.clone());
            assert!(published);
            assert!(!Rc::ptr_eq(&before, &after));
            let (stable, published) = cache.resolve(changed);
            assert!(!published);
            assert!(Rc::ptr_eq(&after, &stable));
        }
    }
}
