use super::*;
use ferroui_controls::{TextBox, TopLevel};

impl Runtime {
    pub fn dispatch(&self, action: UiAction) {
        if self.state.world_busy.get_untracked() {
            return;
        }
        if self.state.import_busy.get_untracked()
            && !matches!(&action, UiAction::CancelImport | UiAction::OpenSource(_))
        {
            return;
        }
        if self.dispatch_world(&action) {
            return;
        }
        let mut sound = None;
        let return_to_world = matches!(
            &action,
            UiAction::ToggleExplore
                | UiAction::ResetCamera
                | UiAction::SpawnBlueprint(_)
                | UiAction::ActivateVehicle(_)
                | UiAction::ExitVehicle
        );
        {
            let mut game = self.session.borrow_mut();
            match action {
                UiAction::SelectPart(id) => game.select_part(id),
                UiAction::SelectColor(id) => game.select_color(id),
                UiAction::Rotate => game.rotate(),
                UiAction::Place => {
                    game.place();
                    sound = Some("place");
                }
                UiAction::Remove => {
                    game.remove();
                    sound = Some("remove");
                }
                UiAction::ToggleExplore => game.toggle_explore(),
                UiAction::ResetCamera => game.camera = Default::default(),
                UiAction::SpawnBlueprint(name) => {
                    game.spawn_blueprint(&name);
                    sound = Some("build");
                }
                UiAction::ActivateVehicle(kind) => game.activate_vehicle(&kind),
                UiAction::ExitVehicle => game.park_vehicle(),
                UiAction::NewWorld(seed) => {
                    *game = Session::new(seed);
                    self.renderer.borrow_mut().invalidate();
                }
                UiAction::Undo => game.undo(),
                UiAction::Redo => game.redo(),
                UiAction::SetDaylight(value) => game.daylight = value.clamp(0.0, 1.0) as f32,
                UiAction::SetQuality(value) => game.quality = value.min(2),
                UiAction::SetSound(value) => game.sound = value,
                UiAction::SetGrid(value) => game.grid = value,
                UiAction::SetPaused(value) => {
                    game.paused = value;
                    game.input.clear();
                }
                UiAction::SetExpedition(enabled) => game.set_expedition(enabled),
                UiAction::Move {
                    forward,
                    strafe,
                    up,
                } => {
                    let offset = game.camera.forward() * forward as f32
                        + game.camera.right() * strafe as f32;
                    game.camera.position += offset;
                    game.camera.target += offset;
                    game.camera.position.y += up as f32;
                    game.camera.target.y += up as f32;
                }
                UiAction::TouchAxis { axis, value } => match axis {
                    0 => {
                        game.input.left = value < -0.1;
                        game.input.right = value > 0.1;
                    }
                    1 => {
                        game.input.down = value < -0.1;
                        game.input.up = value > 0.1;
                    }
                    2 => {
                        game.input.backward = value < -0.1;
                        game.input.forward = value > 0.1;
                    }
                    3 => game.input.brake = value > 0.1,
                    _ => {}
                },
                UiAction::Save => match game.to_json() {
                    Ok(json) => {
                        game.status = "Saving your world…".into();
                        crate::bridge::save(&json);
                    }
                    Err(e) => game.status = e.to_string(),
                },
                UiAction::Load => {
                    game.status = "Opening your saved world…".into();
                    self.state.world_busy.set(true);
                    crate::bridge::load();
                }
                UiAction::Export => match game.to_json() {
                    Ok(json) => crate::bridge::export(&json),
                    Err(e) => game.status = e.to_string(),
                },
                UiAction::Import => crate::bridge::import(),
                _ => unreachable!("World service commands are handled before simulation actions"),
            }
            if !game.sound {
                sound = None;
            }
        }
        if let Some(kind) = sound {
            crate::bridge::play(kind);
        }
        if return_to_world {
            self.focus_world_unless_editing();
        }
        self.sync_hud();
    }

    /// Camera and vehicle buttons hand keyboard input back to the world. A host
    /// command issued while a native editor is active must preserve that editor.
    fn focus_world_unless_editing(&self) {
        let Some(view) = self.viewport.get_untracked() else {
            return;
        };
        let editing = TopLevel::get_top_level(Some(&view))
            .and_then(|top| top.focus_manager().get_focused_element())
            .is_some_and(|focused| {
                focused.is_effectively_visible() && focused.cast::<TextBox>().is_some()
            });
        if !editing {
            self.state.close_panels();
            let _ = view.focus();
        }
    }
}
