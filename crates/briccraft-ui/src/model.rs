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
pub struct WorldSummary {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub updated: String,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SourceCredit {
    pub label: String,
    pub url: String,
    pub license: String,
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
    CaptureDesign(String),
    PlaceDesign(u32),
    RenameDesign {
        id: u32,
        name: String,
    },
    DeleteDesign(u32),
    ExportDesign(u32),
    ImportDesign,
    ActivateVehicle(String),
    ExitVehicle,
    NewWorld(u64),
    ShowWorlds,
    ShowPlaces,
    /// The view and logical viewport size are available in UiState.
    MapViewportChanged,
    StartMapRoam {
        latitude: f64,
        longitude: f64,
        meters_per_stud: f64,
        terrain: bool,
    },
    RetryMapRoam,
    StopMapRoam,
    CreateWorld {
        preset: String,
        name: String,
        seed: u64,
    },
    OpenWorld(String),
    SaveWorldAs(String),
    ArchiveWorld {
        id: String,
        archived: bool,
    },
    ImportMap {
        latitude: f64,
        longitude: f64,
        radius: f64,
        meters_per_stud: f64,
        endpoint: String,
        local: bool,
        terrain: bool,
        fit_height: bool,
        vertical_exaggeration: f64,
    },
    ImportModel {
        size: f64,
        sample: bool,
    },
    AppendModel {
        size: f64,
        map_scale: bool,
    },
    ImportLandmark,
    ImportTerrainSample,
    CancelImport,
    ExportReport,
    OpenSource(String),
    Save,
    Load,
    Export,
    Import,
    Undo,
    Redo,
    SetDaylight(f64),
    SetSound(bool),
    SetQuality(u8),
    /// Resolution policy: 0 = Auto, 1 = Quality, 2 = Performance.
    SetResolutionMode(u8),
    /// Requested pixel samples: 1, 2 or 4. Quality honors the selected count.
    SetAntialiasing(u8),
    SetGrid(bool),
    SetPaused(bool),
    /// Switch the ruleset while keeping this world's balance and objective history.
    SetExpedition(bool),
    /// Set held movement in camera-local axes; all zero releases the input.
    Move {
        forward: f64,
        strafe: f64,
        up: f64,
    },
    /// Set one held touch axis without changing the others. Zero releases it.
    /// Axes: 0 = strafe / steer, 1 = rise / descend, 2 = forward / reverse,
    /// 3 = brake (positive values hold the brake).
    TouchAxis {
        axis: u8,
        value: f64,
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
    /// Active transport: car, plane, train, boat, or empty while building.
    pub vehicle_kind: Signal<String>,
    pub status: Signal<String>,
    pub stats: Signal<String>,
    pub world_name: Signal<String>,
    pub world_id: Signal<String>,
    pub streamed: Signal<bool>,
    pub worlds: Signal<Vec<WorldSummary>>,
    pub coordinates: Signal<String>,
    pub attribution: Signal<String>,
    pub attribution_url: Signal<String>,
    pub credits: Signal<Vec<SourceCredit>>,
    pub import_status: Signal<String>,
    pub import_busy: Signal<bool>,
    pub world_busy: Signal<bool>,
    pub import_progress: Signal<f64>,
    pub mechanics: Signal<String>,
    pub gameplay: Signal<crate::gameplay_model::GameplayView>,
    pub designs: Signal<crate::design_model::DesignLibraryView>,
    pub catalog_open: Signal<bool>,
    pub blueprints_open: Signal<bool>,
    /// 0 = starter plans, 1 = this world's saved designs.
    pub blueprint_section: Signal<u8>,
    pub help_open: Signal<bool>,
    pub settings_open: Signal<bool>,
    pub worlds_open: Signal<bool>,
    pub imports_open: Signal<bool>,
    pub places_open: Signal<bool>,
    pub map_view: Signal<crate::MapView>,
    pub map_tiles: Signal<Vec<crate::MapTile>>,
    pub map_status: Signal<String>,
    pub roam_active: Signal<bool>,
    pub roam_status: Signal<String>,
    pub credits_open: Signal<bool>,
    pub gameplay_open: Signal<bool>,
    pub world_details_open: Signal<bool>,
    pub sound: Signal<bool>,
    pub grid: Signal<bool>,
    pub compact: Signal<bool>,
    pub layout: Signal<crate::layout::UiLayout>,
    pub touch: Signal<bool>,
    pub paused: Signal<bool>,
    pub fps: Signal<f64>,
    pub daylight: Signal<f64>,
    /// 0 = performance, 1 = balanced, 2 = cinematic.
    pub quality: Signal<u8>,
    pub resolution_mode: Signal<u8>,
    pub antialiasing: Signal<u8>,
    pub render_status: Signal<String>,
    /// A local presentation preference; diagnostics stay out of the normal HUD.
    pub show_metrics: Signal<bool>,
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
            vehicle_kind: signal(String::new()),
            status: signal("Choose a brick. Make something extraordinary.".into()),
            stats: signal(String::new()),
            world_name: signal("Sunlit archipelago".into()),
            world_id: signal(String::new()),
            streamed: signal(false),
            worlds: signal(Vec::new()),
            coordinates: signal(String::new()),
            attribution: signal(String::new()),
            attribution_url: signal(String::new()),
            credits: signal(Vec::new()),
            import_status: signal(String::new()),
            import_busy: signal(false),
            world_busy: signal(false),
            import_progress: signal(0.0),
            mechanics: signal(
                "Connect a chassis, motor, axles and wheels to bring your build to life.".into(),
            ),
            gameplay: signal(crate::gameplay_model::GameplayView::default()),
            designs: signal(crate::design_model::DesignLibraryView::default()),
            catalog_open: signal(false),
            blueprints_open: signal(false),
            blueprint_section: signal(0),
            help_open: signal(false),
            settings_open: signal(false),
            worlds_open: signal(false),
            imports_open: signal(false),
            places_open: signal(false),
            map_view: signal(crate::MapView::default()),
            map_tiles: signal(Vec::new()),
            map_status: signal("Choose a place on the map.".into()),
            roam_active: signal(false),
            roam_status: signal(String::new()),
            credits_open: signal(false),
            gameplay_open: signal(false),
            world_details_open: signal(false),
            sound: signal(true),
            grid: signal(false),
            compact: signal(false),
            layout: signal(crate::layout::UiLayout::default()),
            touch: signal(false),
            paused: signal(false),
            fps: signal(0.0),
            daylight: signal(0.35),
            quality: signal(1),
            resolution_mode: signal(0),
            antialiasing: signal(2),
            render_status: signal("Preparing the world display…".into()),
            show_metrics: signal(false),
            seed: signal(42),
        }
    }

    pub fn close_panels(self) {
        batch(|| {
            self.catalog_open.set(false);
            self.blueprints_open.set(false);
            self.help_open.set(false);
            self.settings_open.set(false);
            self.worlds_open.set(false);
            self.imports_open.set(false);
            self.places_open.set(false);
            self.credits_open.set(false);
            self.gameplay_open.set(false);
            self.world_details_open.set(false);
        });
    }

    pub fn modal(self) -> bool {
        self.catalog_open.get()
            || self.blueprints_open.get()
            || self.help_open.get()
            || self.settings_open.get()
            || self.worlds_open.get()
            || self.imports_open.get()
            || self.places_open.get()
            || self.credits_open.get()
            || self.gameplay_open.get()
            || self.world_details_open.get()
            || self.import_busy.get()
            || self.world_busy.get()
    }

    pub fn modal_untracked(self) -> bool {
        self.catalog_open.get_untracked()
            || self.blueprints_open.get_untracked()
            || self.help_open.get_untracked()
            || self.settings_open.get_untracked()
            || self.worlds_open.get_untracked()
            || self.imports_open.get_untracked()
            || self.places_open.get_untracked()
            || self.credits_open.get_untracked()
            || self.gameplay_open.get_untracked()
            || self.world_details_open.get_untracked()
            || self.import_busy.get_untracked()
            || self.world_busy.get_untracked()
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
