use briccraft_game::Session;
use briccraft_render::{Camera, PlacementPreview, RenderFrame, Renderer, VehicleRender};
use briccraft_ui::{ColorSwatch, PartCard, UiAction, UiState};
use ferroui_base::media::effects::IEffect;
use ferroui_controls::Border;
use ferroui_view::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub struct Runtime {
    pub session: RefCell<Session>,
    pub renderer: RefCell<Renderer>,
    pub state: UiState,
    pub viewport: NodeRef<Border>,
    pub world_effect: Signal<Option<Rc<dyn IEffect>>>,
    pub pointer: RefCell<crate::viewport::PointerState>,
    pub owner: Owner,
    last_frame: Cell<f64>,
    last_hud: Cell<f64>,
    fps: Cell<f64>,
    fps_elapsed: Cell<f64>,
    fps_frames: Cell<u64>,
    presented_frames: Cell<u64>,
    scene_generation: Cell<u64>,
    pub import_anchor: Cell<Option<briccraft_stream::GlobalPos>>,
    pub import_world_id: RefCell<String>,
}

impl Runtime {
    pub fn new(owner: Owner) -> Self {
        let session = Session::new(42);
        let state = UiState::new();
        state.world_busy.set(true);
        Self {
            session: RefCell::new(session),
            renderer: RefCell::new(Renderer::new()),
            state,
            viewport: NodeRef::new(),
            world_effect: signal(None),
            pointer: RefCell::new(Default::default()),
            owner,
            last_frame: Cell::new(0.0),
            last_hud: Cell::new(0.0),
            fps: Cell::new(0.0),
            fps_elapsed: Cell::new(0.0),
            fps_frames: Cell::new(0),
            presented_frames: Cell::new(0),
            scene_generation: Cell::new(0),
            import_anchor: Cell::new(None),
            import_world_id: RefCell::new(String::new()),
        }
    }
    pub fn view(self: &Rc<Self>) -> Element {
        let parts = Rc::new(
            briccraft_parts::CATALOG
                .iter()
                .map(|p| PartCard {
                    id: p.id,
                    name: p.name.into(),
                    category: p.category.name().into(),
                    size: p.size,
                })
                .collect(),
        );
        let colors = Rc::new(
            briccraft_parts::PALETTE
                .iter()
                .enumerate()
                .map(|(i, c)| ColorSwatch {
                    id: i as u8,
                    name: briccraft_parts::COLOR_NAMES[i].into(),
                    hex: format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2]),
                })
                .collect(),
        );
        self.sync_hud();
        let dispatch = {
            let runtime = self.clone();
            Rc::new(move |action| runtime.dispatch(action))
        };
        briccraft_ui::view(
            crate::viewport::view(self.clone()),
            parts,
            colors,
            self.state,
            dispatch,
        )
    }
}

mod actions;
mod frame;
mod hud;
mod world_actions;
