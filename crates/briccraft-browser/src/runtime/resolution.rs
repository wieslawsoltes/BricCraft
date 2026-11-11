//! Image density is a quality contract, independent of the native HUD.
//! Shader compilation and idle menus must not drive the world into a tiny bitmap.
//! This policy has no FerroUI dependency and is tested directly with rustc.
#[path = "resolution/policy.rs"]
mod policy;
use policy::{extent, limits, samples, ResolutionMode, MAX_DIMENSION};

const SAMPLE_SECONDS: f64 = 1.0;
const FAST_FRAME_SECONDS: f64 = 1.0 / 55.0;
const SLOW_FRAME_SECONDS: f64 = 1.0 / 28.0;
const HEADROOM_SECONDS: f64 = 3.0;

#[derive(Clone, Copy, Debug)]
pub struct ResolutionStats {
    pub render_scale: f64,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub effect_width: u32,
    pub effect_height: u32,
    pub pixel_budget: u32,
    pub minimum_pixels: u32,
    pub requested_quality: u8,
    pub effective_quality: u8,
    pub requested_samples: u8,
    pub samples: u8,
    pub mode: u8,
    pub presented_fps: f64,
}
impl ResolutionStats {
    pub fn mode_name(&self) -> &'static str {
        ResolutionMode::from_u8(self.mode).name()
    }
}

pub struct ResolutionController {
    mode: ResolutionMode,
    requested_samples: u8,
    budget: f64,
    effective_quality: u8,
    effective_samples: u8,
    configuration_changed: bool,
    last_surface: Option<[f64; 3]>,
    last_presented: Option<u64>,
    warmup_frames: u64,
    sample_seconds: f64,
    sample_frames: u64,
    headroom_seconds: f64,
    stats: ResolutionStats,
}
impl ResolutionController {
    pub fn new() -> Self {
        Self {
            mode: ResolutionMode::Auto,
            requested_samples: 2,
            budget: 0.0,
            effective_quality: 1,
            effective_samples: 2,
            configuration_changed: true,
            last_surface: None,
            last_presented: None,
            warmup_frames: 4,
            sample_seconds: 0.0,
            sample_frames: 0,
            headroom_seconds: 0.0,
            stats: ResolutionStats {
                render_scale: 1.0,
                pixel_width: 0,
                pixel_height: 0,
                effect_width: 0,
                effect_height: 0,
                pixel_budget: 0,
                minimum_pixels: 0,
                requested_quality: 1,
                effective_quality: 1,
                requested_samples: 2,
                samples: 2,
                mode: 0,
                presented_fps: 0.0,
            },
        }
    }
    pub fn stats(&self) -> ResolutionStats {
        self.stats
    }
    pub fn mode(&self) -> u8 {
        self.mode as u8
    }
    pub fn requested_samples(&self) -> u8 {
        self.requested_samples
    }
    pub fn set_mode(&mut self, value: u8) {
        let mode = ResolutionMode::from_u8(value);
        if mode != self.mode {
            self.mode = mode;
            self.stats.mode = mode as u8;
            self.configuration_changed = true;
        }
    }
    pub fn set_samples(&mut self, value: u8) {
        let value = samples(value);
        if value != self.requested_samples {
            self.requested_samples = value;
            self.stats.requested_samples = value;
            self.configuration_changed = true;
        }
    }
    fn reset_measurement(&mut self) {
        self.sample_seconds = 0.0;
        self.sample_frames = 0;
        self.headroom_seconds = 0.0;
    }

    pub fn update(
        &mut self,
        elapsed: f64,
        presented: u64,
        logical_size: [f64; 2],
        device_scale: f64,
        requested_quality: u8,
        measure: bool,
    ) -> ResolutionStats {
        let requested_quality = requested_quality.min(2);
        let logical = [
            extent(logical_size[0], 1.0, 32_768.0),
            extent(logical_size[1], 1.0, 32_768.0),
        ];
        let dpi = extent(device_scale, 0.125, 64.0);
        let surface = [logical[0], logical[1], dpi];
        let limits = limits(logical, dpi, self.mode);
        let configuration_changed = self.configuration_changed
            || self.last_surface.is_none()
            || requested_quality != self.stats.requested_quality;
        let surface_changed = self.last_surface != Some(surface);
        let reset = configuration_changed || surface_changed;
        if configuration_changed {
            self.budget = if self.mode == ResolutionMode::Performance {
                limits.minimum
            } else {
                limits.maximum
            };
            self.effective_quality = if self.mode == ResolutionMode::Performance {
                0
            } else {
                requested_quality
            };
            self.effective_samples = if self.mode == ResolutionMode::Performance {
                1
            } else {
                self.requested_samples
            };
        } else if surface_changed {
            // Keep the observed cost tradeoff through resizing/keyboard changes.
            // A one-pixel resize must not restore expensive AA and lighting.
            let previous = self.last_surface.expect("an existing surface was checked");
            let previous_area = previous[0] * previous[1] * previous[2].powi(2);
            self.budget *= limits.native[0] * limits.native[1] / previous_area;
        }
        if reset {
            self.configuration_changed = false;
            self.last_surface = Some(surface);
            self.warmup_frames = 4;
            self.reset_measurement();
            self.last_presented = Some(presented);
        }
        self.effective_quality = self.effective_quality.min(requested_quality);
        self.effective_samples = self.effective_samples.min(self.requested_samples);
        if let Some(previous) = self.last_presented.replace(presented) {
            if presented < previous {
                self.warmup_frames = 4;
                self.reset_measurement();
            } else if !measure {
                // UI-only/paused frames are not evidence of world-rendering cost.
                self.reset_measurement();
            } else if elapsed.is_finite() && elapsed > 0.0 && !reset {
                let frames = presented - previous;
                if self.warmup_frames > 0 {
                    self.warmup_frames = self.warmup_frames.saturating_sub(frames);
                    self.reset_measurement();
                } else {
                    self.sample_seconds += elapsed;
                    self.sample_frames = self.sample_frames.saturating_add(frames);
                    if self.sample_seconds >= SAMPLE_SECONDS {
                        self.adapt(requested_quality, limits.maximum);
                        self.sample_seconds = 0.0;
                        self.sample_frames = 0;
                    }
                }
            }
        }
        if self.mode == ResolutionMode::Quality {
            self.budget = limits.maximum;
            self.effective_quality = requested_quality;
            self.effective_samples = self.requested_samples;
        }
        self.budget = self.budget.clamp(limits.minimum, limits.maximum);
        let budget = self.budget.floor().max(1.0);
        let native_width = limits.native[0];
        let native_height = limits.native[1];
        let target_height = (budget * native_height / native_width)
            .sqrt()
            .floor()
            .clamp(1.0, budget.min(MAX_DIMENSION));
        let target_width = (budget / target_height).floor().clamp(1.0, MAX_DIMENSION);
        let ideal = (target_width / native_width)
            .min(target_height / native_height)
            .min(1.0);
        // A native-sized surface needs no inward margin; fractional cache sizes
        // must account for Skia f32 matrices and its floor/ceil bound conventions.
        let native_fits = native_width.ceil() * native_height.ceil() <= budget
            && native_width.ceil() <= MAX_DIMENSION
            && native_height.ceil() <= MAX_DIMENSION;
        let render_scale = if native_fits {
            1.0
        } else {
            ideal * (1.0 - 0.000_001)
        };
        self.stats = ResolutionStats {
            render_scale,
            pixel_width: (native_width * render_scale).floor().max(1.0) as u32,
            pixel_height: (native_height * render_scale).floor().max(1.0) as u32,
            effect_width: (native_width * render_scale).ceil().max(1.0) as u32,
            effect_height: (native_height * render_scale).ceil().max(1.0) as u32,
            pixel_budget: budget as u32,
            minimum_pixels: limits.minimum as u32,
            requested_quality,
            effective_quality: self.effective_quality,
            requested_samples: self.requested_samples,
            samples: self.effective_samples,
            mode: self.mode as u8,
            presented_fps: self.stats.presented_fps,
        };
        self.stats
    }

    fn adapt(&mut self, requested_quality: u8, maximum: f64) {
        self.stats.presented_fps = self.sample_frames as f64 / self.sample_seconds;
        if self.sample_frames == 0 {
            self.headroom_seconds = 0.0;
            return;
        }
        if self.mode == ResolutionMode::Quality {
            return;
        }
        let frame_seconds = self.sample_seconds / self.sample_frames as f64;
        if frame_seconds > SLOW_FRAME_SECONDS {
            self.headroom_seconds = 0.0;
            // Protect silhouette/detail density before reducing image dimensions.
            if self.effective_samples > 1 {
                self.effective_samples /= 2;
            } else if self.effective_quality > 0 {
                self.effective_quality -= 1;
            } else {
                self.budget *= 0.90;
            }
        } else if frame_seconds < FAST_FRAME_SECONDS {
            self.headroom_seconds += self.sample_seconds;
            if self.headroom_seconds >= HEADROOM_SECONDS {
                self.headroom_seconds = 0.0;
                // Recover resolution before spending spare work on extra lighting
                // and samples. Tiny surfaces cannot win by merely raising effects.
                if self.budget < maximum * 0.995 {
                    self.budget = (self.budget * 1.15).min(maximum);
                } else if self.effective_quality < requested_quality {
                    self.effective_quality += 1;
                } else if self.effective_samples < self.requested_samples {
                    self.effective_samples =
                        (self.effective_samples * 2).min(self.requested_samples);
                }
            }
        } else {
            self.headroom_seconds = 0.0;
        }
    }
}
#[cfg(test)]
#[path = "resolution/tests.rs"]
mod tests;
