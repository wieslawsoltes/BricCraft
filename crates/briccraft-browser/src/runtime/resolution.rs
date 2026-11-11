//! Physical scene resolution and effect quality follow measured presentation time.
//! This module is deliberately independent of FerroUI so its policy can be tested
//! with rustc. The native HUD stays outside the world's cached render layer.

const START_PIXELS: f64 = 65_536.0;
const MIN_PIXELS: f64 = 16_384.0;
const MAX_PIXELS: [f64; 3] = [262_144.0, 1_048_576.0, 2_073_600.0];
const SAMPLE_SECONDS: f64 = 0.75;
const FAST_FRAME_SECONDS: f64 = 1.0 / 45.0;
const SLOW_FRAME_SECONDS: f64 = 1.0 / 24.0;
const TARGET_FRAME_SECONDS: f64 = 1.0 / 30.0;
const HEADROOM_SECONDS: f64 = 2.25;
const MAX_DIMENSION: f64 = 4_096.0;

#[derive(Clone, Copy, Debug)]
pub struct ResolutionStats {
    /// BitmapCache scale relative to the native display's device pixels.
    pub render_scale: f64,
    pub pixel_width: u32,
    pub pixel_height: u32,
    /// Conservative device bounds of the shader overlay inside the cache.
    pub effect_width: u32,
    pub effect_height: u32,
    pub pixel_budget: u32,
    pub requested_quality: u8,
    pub effective_quality: u8,
    pub presented_fps: f64,
}

pub struct ResolutionController {
    budget: f64,
    effective_quality: u8,
    last_presented: Option<u64>,
    sample_seconds: f64,
    sample_frames: u64,
    headroom_seconds: f64,
    quality_cooldown: f64,
    stats: ResolutionStats,
}

impl ResolutionController {
    pub fn new() -> Self {
        Self {
            budget: START_PIXELS,
            effective_quality: 0,
            last_presented: None,
            sample_seconds: 0.0,
            sample_frames: 0,
            headroom_seconds: 0.0,
            quality_cooldown: 0.0,
            stats: ResolutionStats {
                render_scale: 0.125,
                pixel_width: 0,
                pixel_height: 0,
                effect_width: 0,
                effect_height: 0,
                pixel_budget: START_PIXELS as u32,
                requested_quality: 1,
                effective_quality: 0,
                presented_fps: 0.0,
            },
        }
    }

    pub fn stats(&self) -> ResolutionStats {
        self.stats
    }

    pub fn update(
        &mut self,
        elapsed: f64,
        presented: u64,
        logical_size: [f64; 2],
        device_scale: f64,
        requested_quality: u8,
    ) -> ResolutionStats {
        let requested_quality = requested_quality.min(2);
        let width = extent(logical_size[0], 1.0, 32_768.0);
        let height = extent(logical_size[1], 1.0, 32_768.0);
        let device_scale = extent(device_scale, 0.125, 64.0);
        let native_width = width * device_scale;
        let native_height = height * device_scale;
        let native_pixels = native_width.ceil().max(1.0) * native_height.ceil().max(1.0);
        let maximum = MAX_PIXELS[requested_quality as usize].min(native_pixels);
        let minimum = MIN_PIXELS.min(maximum);

        if requested_quality != self.stats.requested_quality {
            self.headroom_seconds = 0.0;
        }
        self.effective_quality = self.effective_quality.min(requested_quality);
        self.budget = self.budget.clamp(minimum, maximum);

        if let Some(previous) = self.last_presented.replace(presented) {
            if elapsed.is_finite() && elapsed > 0.0 {
                self.quality_cooldown = (self.quality_cooldown - elapsed).max(0.0);
                self.sample_seconds += elapsed;
                let frames = presented.saturating_sub(previous);
                self.sample_frames = self.sample_frames.saturating_add(frames);
                if self.sample_seconds >= SAMPLE_SECONDS {
                    self.adapt(requested_quality);
                    self.sample_seconds = 0.0;
                    self.sample_frames = 0;
                }
            }
        }
        self.budget = self.budget.clamp(minimum, maximum);

        // The budget is in physical pixels. A fixed minimum scale would exceed
        // it on a large Retina display, so the floor is a pixel budget instead.
        // Fit integer overlay bounds as well: the cache currently truncates
        // its backing size, whereas ShaderLayer rounds its device bounds up.
        let budget = self.budget.floor().max(1.0);
        let target_height = (budget * native_height / native_width)
            .sqrt()
            .floor()
            .clamp(1.0, budget.min(MAX_DIMENSION));
        let target_width = (budget / target_height).floor().clamp(1.0, MAX_DIMENSION);
        let render_scale = (target_width / native_width)
            .min(target_height / native_height)
            .min(1.0)
            * (1.0 - 0.000_001);
        // The tiny inward margin also covers Skia's f32 matrix conversion at
        // an exact integer edge; it does not add a visible resolution step.
        self.stats = ResolutionStats {
            render_scale,
            pixel_width: (native_width * render_scale).floor() as u32,
            pixel_height: (native_height * render_scale).floor() as u32,
            effect_width: (native_width * render_scale).ceil().max(1.0) as u32,
            effect_height: (native_height * render_scale).ceil().max(1.0) as u32,
            pixel_budget: self.budget.floor() as u32,
            requested_quality,
            effective_quality: self.effective_quality,
            presented_fps: self.stats.presented_fps,
        };
        self.stats
    }

    fn adapt(&mut self, requested_quality: u8) {
        self.stats.presented_fps = self.sample_frames as f64 / self.sample_seconds;
        if self.sample_frames == 0 {
            self.headroom_seconds = 0.0;
            return;
        }
        let frame_seconds = self.sample_seconds / self.sample_frames as f64;
        if frame_seconds > SLOW_FRAME_SECONDS {
            self.headroom_seconds = 0.0;
            if self.effective_quality > 0 {
                self.effective_quality -= 1;
                self.quality_cooldown = 8.0;
            }
            // At most a fourfold area reduction per sample; a long actual draw
            // must remain in the measurement instead of being clamped away.
            self.budget *= (TARGET_FRAME_SECONDS / frame_seconds).clamp(0.25, 0.8);
        } else if frame_seconds < FAST_FRAME_SECONDS {
            self.headroom_seconds += self.sample_seconds;
            if self.headroom_seconds >= HEADROOM_SECONDS {
                self.headroom_seconds = 0.0;
                if self.effective_quality < requested_quality
                    && self.quality_cooldown <= 0.0
                    && self.budget >= START_PIXELS
                {
                    self.effective_quality += 1;
                    self.quality_cooldown = 3.0;
                } else {
                    self.budget *= 1.5;
                }
            }
        } else {
            self.headroom_seconds = 0.0;
        }
    }
}

fn extent(value: f64, minimum: f64, maximum: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value.clamp(minimum, maximum)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance(
        controller: &mut ResolutionController,
        counter: &mut u64,
        frames: usize,
        seconds: f64,
        quality: u8,
    ) -> ResolutionStats {
        let mut stats = controller.stats();
        for _ in 0..frames {
            *counter += 1;
            stats = controller.update(seconds, *counter, [1_280.0, 800.0], 1.0, quality);
        }
        stats
    }

    #[test]
    fn startup_budget_is_physical_and_bounded_on_retina_displays() {
        for (size, dpi) in [
            ([1_280.0, 800.0], 1.0),
            ([3_840.0, 2_160.0], 2.0),
            ([3_840.0, 2_160.0], 3.0),
        ] {
            let mut controller = ResolutionController::new();
            let stats = controller.update(0.0, 0, size, dpi, 2);
            assert!(stats.pixel_width as u64 * stats.pixel_height as u64 <= 65_536);
            assert!(stats.effect_width as u64 * stats.effect_height as u64 <= 65_536);
            assert!(stats.render_scale > 0.0 && stats.render_scale <= 1.0);
            assert_eq!(stats.effective_quality, 0);
            assert_eq!(stats.requested_quality, 2);
        }
    }

    #[test]
    fn actual_stalls_reduce_the_budget_and_recovery_requires_sustained_headroom() {
        let mut controller = ResolutionController::new();
        let mut counter = 0;
        controller.update(0.0, counter, [1_280.0, 800.0], 1.0, 1);
        let slow = advance(&mut controller, &mut counter, 1, 2.0, 1);
        assert_eq!(slow.pixel_budget, MIN_PIXELS as u32);
        assert_eq!(slow.presented_fps, 0.5);
        let briefly_fast = advance(&mut controller, &mut counter, 60, 1.0 / 60.0, 1);
        assert_eq!(briefly_fast.pixel_budget, MIN_PIXELS as u32);
        let recovered = advance(&mut controller, &mut counter, 180, 1.0 / 60.0, 1);
        assert!(recovered.pixel_budget > MIN_PIXELS as u32);
        assert!(recovered.pixel_budget < START_PIXELS as u32);
    }

    #[test]
    fn quality_has_hysteresis_and_a_downgrade_cooldown() {
        let mut controller = ResolutionController::new();
        let mut counter = 0;
        controller.update(0.0, counter, [1_280.0, 800.0], 1.0, 1);
        let brief = advance(&mut controller, &mut counter, 100, 1.0 / 60.0, 1);
        assert_eq!(brief.effective_quality, 0);
        let stable = advance(&mut controller, &mut counter, 100, 1.0 / 60.0, 1);
        assert_eq!(stable.effective_quality, 1);
        let slow = advance(&mut controller, &mut counter, 1, 1.0, 1);
        assert_eq!(slow.effective_quality, 0);
        let early = advance(&mut controller, &mut counter, 180, 1.0 / 60.0, 1);
        assert_eq!(early.effective_quality, 0);
    }

    #[test]
    fn effects_never_exceed_the_requested_quality() {
        let mut controller = ResolutionController::new();
        let mut counter = 0;
        controller.update(0.0, counter, [1_280.0, 800.0], 1.0, 2);
        let high = advance(&mut controller, &mut counter, 900, 1.0 / 60.0, 2);
        assert_eq!(high.effective_quality, 2);
        let low = controller.update(0.0, counter, [1_280.0, 800.0], 1.0, 0);
        assert_eq!(low.effective_quality, 0);
        assert_eq!(low.requested_quality, 0);
        assert!(low.pixel_budget <= MAX_PIXELS[0] as u32);
        let stable = advance(&mut controller, &mut counter, 900, 1.0 / 60.0, 0);
        assert_eq!(stable.effective_quality, 0);
    }
}
