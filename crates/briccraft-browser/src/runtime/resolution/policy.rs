//! Display-relative quality floors and explicit player-selected tradeoffs.
pub const MAX_DIMENSION: f64 = 4096.0;
const AUTO_MAX_PIXELS: f64 = 4_194_304.0;
const QUALITY_MAX_PIXELS: f64 = 8_294_400.0;
const PERFORMANCE_MAX_PIXELS: f64 = 2_073_600.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionMode {
    Auto = 0,
    Quality = 1,
    Performance = 2,
}
impl ResolutionMode {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Quality,
            2 => Self::Performance,
            _ => Self::Auto,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Quality => "quality",
            Self::Performance => "performance",
        }
    }
}
pub fn samples(value: u8) -> u8 {
    match value {
        4 => 4,
        2 => 2,
        _ => 1,
    }
}
pub fn extent(value: f64, minimum: f64, maximum: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value.clamp(minimum, maximum)
    } else {
        minimum
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub minimum: f64,
    pub maximum: f64,
    pub native: [f64; 2],
}
pub fn limits(logical: [f64; 2], device_scale: f64, mode: ResolutionMode) -> Limits {
    let width = extent(logical[0], 1.0, 32_768.0);
    let height = extent(logical[1], 1.0, 32_768.0);
    let dpi = extent(device_scale, 0.125, 64.0);
    let native = [width * dpi, height * dpi];
    let native_pixels = native[0].ceil().max(1.0) * native[1].ceil().max(1.0);
    let dimension_scale = (MAX_DIMENSION / native[0])
        .min(MAX_DIMENSION / native[1])
        .min(1.0);
    let dimension_pixels = (native_pixels * dimension_scale * dimension_scale)
        .floor()
        .max(1.0);
    let logical_pixels = width * height;
    let (floor, ceiling) = match mode {
        // Auto begins at native density. Under sustained pressure it retains at
        // least 0.75 scene pixels per CSS pixel, subject to the global safety cap.
        ResolutionMode::Auto => (logical_pixels * 0.75_f64.powi(2), AUTO_MAX_PIXELS),
        ResolutionMode::Quality => (QUALITY_MAX_PIXELS, QUALITY_MAX_PIXELS),
        // This lower-density mode is an explicit choice, never an automatic
        // emergency fallback from Auto or Quality.
        ResolutionMode::Performance => (
            logical_pixels * 0.40_f64.powi(2),
            (logical_pixels * 0.75_f64.powi(2)).min(PERFORMANCE_MAX_PIXELS),
        ),
    };
    let maximum = native_pixels
        .min(dimension_pixels)
        .min(ceiling)
        .floor()
        .max(1.0);
    Limits {
        minimum: floor.min(maximum).floor().max(1.0),
        maximum,
        native,
    }
}
