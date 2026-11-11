use std::fmt;

/// Distances are world units, speeds are world units/second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharacterConfig {
    pub radius: f32,
    pub height: f32,
    pub eye_height: f32,
    pub step_height: f32,
    pub gravity: f32,
    pub jump_speed: f32,
    pub terminal_speed: f32,
    pub max_speed: f32,
    pub ground_acceleration: f32,
    pub air_acceleration: f32,
    /// Only maintains an existing ground contact; it does not pull a falling player down.
    pub ground_snap: f32,
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            radius: 0.32,
            height: 2.5,
            eye_height: 2.2,
            step_height: 0.8,
            gravity: 18.0,
            jump_speed: 7.0,
            terminal_speed: 48.0,
            max_speed: 18.0,
            ground_acceleration: 16.0,
            air_acceleration: 5.0,
            ground_snap: 0.06,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigError(pub &'static str);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ConfigError {}

impl CharacterConfig {
    pub fn validate(self) -> Result<Self, ConfigError> {
        let positive = [
            self.radius,
            self.height,
            self.eye_height,
            self.gravity,
            self.jump_speed,
            self.terminal_speed,
            self.max_speed,
            self.ground_acceleration,
            self.air_acceleration,
        ];
        if positive
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err(ConfigError(
                "Character dimensions and movement rates must be finite and positive.",
            ));
        }
        if self.eye_height >= self.height {
            return Err(ConfigError("The character's eyes must be below its head."));
        }
        if !self.step_height.is_finite()
            || self.step_height < 0.0
            || self.step_height >= self.height
            || !self.ground_snap.is_finite()
            || self.ground_snap < 0.0
            || self.ground_snap > self.height
        {
            return Err(ConfigError(
                "Step and ground-snap distances must fit the character height.",
            ));
        }
        Ok(self)
    }
}
