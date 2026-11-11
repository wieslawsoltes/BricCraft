//! Responsive geometry shared by the native HUD and its platform host.
//! Values are logical pixels; display scale belongs to the world renderer.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiLayout {
    pub width: f64,
    pub height: f64,
    /// Left, top, right, bottom, including the visible keyboard occlusion.
    pub insets: [f64; 4],
    pub compact: bool,
    pub landscape: bool,
    pub touch: bool,
    pub keyboard: bool,
}

impl UiLayout {
    pub fn new(size: [f64; 2], safe: [f64; 4], occlusion: [f64; 2], touch: bool) -> Self {
        let finite = |value: f64| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        };
        let outer_width = finite(size[0]).max(1.0);
        let outer_height = finite(size[1]).max(1.0);
        let mut insets = safe.map(finite);
        insets[0] = insets[0].min(outer_width * 0.4);
        insets[2] = insets[2].min(outer_width * 0.4);
        insets[1] = insets[1].max(finite(occlusion[0])).min(outer_height * 0.8);
        insets[3] = insets[3]
            .max(finite(occlusion[1]))
            .min((outer_height - insets[1] - 1.0).max(0.0));
        let width = (outer_width - insets[0] - insets[2]).max(1.0);
        let height = (outer_height - insets[1] - insets[3]).max(1.0);
        Self {
            width,
            height,
            insets,
            compact: width < 900.0 || height < 620.0,
            landscape: width >= 600.0 && height < 560.0 && width > height * 1.3,
            touch: touch || width < 600.0,
            keyboard: finite(occlusion[1]) > 80.0,
        }
    }

    pub fn safe_margin(self) -> (f64, f64, f64, f64) {
        (
            self.insets[0],
            self.insets[1],
            self.insets[2],
            self.insets[3],
        )
    }

    pub fn header_bottom(self) -> f64 {
        if self.compact && !self.landscape {
            120.0
        } else {
            80.0
        }
    }

    pub fn panel_width(self) -> f64 {
        let available = (self.width - 24.0).max(1.0);
        if self.compact && !self.landscape {
            available.min(440.0)
        } else {
            available.min(420.0)
        }
    }

    pub fn panel_margin(self) -> (f64, f64, f64, f64) {
        let top = if self.keyboard || self.height < 420.0 {
            12.0
        } else {
            self.header_bottom() + 8.0
        };
        (12.0, top.min((self.height - 140.0).max(0.0)), 12.0, 12.0)
    }

    pub fn hotbar_bottom(self) -> f64 {
        if self.landscape {
            10.0
        } else if self.compact {
            64.0
        } else {
            56.0
        }
    }

    pub fn controls_bottom(self) -> f64 {
        if self.landscape {
            76.0
        } else {
            188.0
        }
    }

    pub fn hotbar_slots(self) -> usize {
        if self.compact {
            4
        } else {
            8
        }
    }
}

impl Default for UiLayout {
    fn default() -> Self {
        Self::new([1280.0, 800.0], [0.0; 4], [0.0; 2], false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_panels_fit_portrait_landscape_and_safe_areas() {
        for (size, safe) in [
            ([390.0, 844.0], [0.0, 47.0, 0.0, 34.0]),
            ([844.0, 390.0], [47.0, 0.0, 47.0, 21.0]),
            ([320.0, 568.0], [0.0; 4]),
        ] {
            let layout = UiLayout::new(size, safe, [0.0; 2], true);
            let (left, top, right, bottom) = layout.panel_margin();
            assert!(layout.panel_width() + left + right <= layout.width);
            assert!(layout.height - top - bottom >= 140.0);
            assert!(layout.touch && layout.compact);
            assert_eq!(layout.hotbar_slots(), 4);
        }
    }

    #[test]
    fn keyboard_uses_visible_space_without_double_counting_safe_bottom() {
        let layout = UiLayout::new([390.0, 844.0], [0.0, 47.0, 0.0, 34.0], [0.0, 320.0], true);
        assert_eq!(layout.height, 477.0);
        assert_eq!(layout.insets[3], 320.0);
        assert!(layout.keyboard);
        assert_eq!(layout.panel_margin().1, 12.0);
    }

    #[test]
    fn desktop_remains_spacious_and_invalid_measurements_stay_finite() {
        let desktop = UiLayout::default();
        assert!(!desktop.compact && !desktop.touch && !desktop.landscape);
        assert_eq!(desktop.hotbar_slots(), 8);
        let invalid = UiLayout::new(
            [f64::NAN, f64::INFINITY],
            [f64::NAN; 4],
            [-1.0, f64::INFINITY],
            false,
        );
        assert!(invalid.width.is_finite() && invalid.height.is_finite());
        assert!(invalid
            .insets
            .into_iter()
            .all(|x| x.is_finite() && x >= 0.0));
    }
}
