//! Renderer-independent state and commands for the declarative game interface.
use ferroui_view::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct PartCard {
    pub id: u16,
    pub name: String,
    pub category: String,
    /// Width in studs, height in plates, depth in studs.
    pub size: [u8; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorSwatch {
    pub id: u8,
    pub name: String,
    pub hex: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    SelectPart(u16),
    SelectColor(u8),
    Rotate,
    Place,
    Remove,
    ToggleExplore,
    ResetCamera,
    SpawnBlueprint(String),
    ActivateVehicle(String),
    ExitVehicle,
    NewWorld(u64),
    Save,
    Load,
    Export,
    Import,
    Undo,
    Redo,
    SetDaylight(f64),
    SetSound(bool),
    SetQuality(u8),
    SetGrid(bool),
    SetPaused(bool),
    /// A short movement impulse for touch controls, in camera-local axes.
    Move {
        forward: f64,
        strafe: f64,
        up: f64,
    },
}

/// Shared by the drawn slots and keyboard input; order is part of the control contract.
pub const HOTBAR_PARTS: [u16; 8] = [3, 1, 23, 26, 50, 61, 80, 100];

pub type Dispatch = Rc<dyn Fn(UiAction)>;

/// Signals belong to the view owner. Simulation can update them independently of
/// its frame uniforms: changing the camera never rebuilds the HUD.
#[derive(Clone, Copy)]
pub struct UiState {
    pub selected_part: Signal<u16>,
    pub selected_color: Signal<u8>,
    pub selected_name: Signal<String>,
    pub selected_size: Signal<String>,
    pub mode: Signal<String>,
    pub status: Signal<String>,
    pub stats: Signal<String>,
    pub world_name: Signal<String>,
    pub mechanics: Signal<String>,
    pub catalog_open: Signal<bool>,
    pub blueprints_open: Signal<bool>,
    pub help_open: Signal<bool>,
    pub settings_open: Signal<bool>,
    pub sound: Signal<bool>,
    pub grid: Signal<bool>,
    pub compact: Signal<bool>,
    pub paused: Signal<bool>,
    pub fps: Signal<f64>,
    pub daylight: Signal<f64>,
    /// 0 = performance, 1 = balanced, 2 = cinematic.
    pub quality: Signal<u8>,
    pub seed: Signal<u64>,
}

impl UiState {
    /// Create under a retained `ferroui_view::Owner`.
    pub fn new() -> Self {
        Self {
            selected_part: signal(0),
            selected_color: signal(0),
            selected_name: signal("Brick 2 × 4".into()),
            selected_size: signal("2 × 4 studs · 3 plates".into()),
            mode: signal("BUILD".into()),
            status: signal("Choose a brick. Make something extraordinary.".into()),
            stats: signal(String::new()),
            world_name: signal("Sunlit archipelago".into()),
            mechanics: signal(
                "Connect a chassis, motor, axles and wheels to bring your build to life.".into(),
            ),
            catalog_open: signal(false),
            blueprints_open: signal(false),
            help_open: signal(false),
            settings_open: signal(false),
            sound: signal(true),
            grid: signal(false),
            compact: signal(false),
            paused: signal(false),
            fps: signal(0.0),
            daylight: signal(0.35),
            quality: signal(1),
            seed: signal(42),
        }
    }

    pub fn close_panels(self) {
        batch(|| {
            self.catalog_open.set(false);
            self.blueprints_open.set(false);
            self.help_open.set(false);
            self.settings_open.set(false);
        });
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
