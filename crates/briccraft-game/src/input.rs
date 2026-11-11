//! Held input is sampled at the fixed simulation step, never at the OS repeat rate.
#[derive(Clone, Debug, Default)]
pub struct Input {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub fast: bool,
    pub brake: bool,
}

impl Input {
    pub fn set_key(&mut self, key: &str, pressed: bool) {
        match key {
            "KeyW" | "ArrowUp" => self.forward = pressed,
            "KeyS" | "ArrowDown" => self.backward = pressed,
            "KeyA" | "ArrowLeft" => self.left = pressed,
            "KeyD" | "ArrowRight" => self.right = pressed,
            "Space" | "KeyE" => self.up = pressed,
            "KeyQ" | "ControlLeft" | "ControlRight" => self.down = pressed,
            "ShiftLeft" | "ShiftRight" => self.fast = pressed,
            "KeyB" => self.brake = pressed,
            _ => {}
        }
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn axes(&self) -> [f32; 3] {
        [
            self.right as u8 as f32 - self.left as u8 as f32,
            self.up as u8 as f32 - self.down as u8 as f32,
            self.forward as u8 as f32 - self.backward as u8 as f32,
        ]
    }
}
