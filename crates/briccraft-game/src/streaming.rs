use crate::{CameraMode, Session, WorldInfo};
use briccraft_stream::{
    BiomePreset, GlobalChange, PartKey, StreamError, StreamStats, StreamWorld, WindowOrigin,
    WorldSnapshot,
};
use glam::Vec3;

pub struct StreamSession {
    pub stream: StreamWorld,
    pub snapshot: WorldSnapshot,
    pub pending_origin: Option<WindowOrigin>,
    pub(crate) last_blueprint_keys: Vec<PartKey>,
    pub(crate) pending_import: Option<crate::importing::PendingImport>,
    pub(crate) import_focus: Option<briccraft_stream::GlobalPos>,
    pub(crate) import_result: Option<Result<String, String>>,
    pub(crate) import_success: Option<String>,
}

impl StreamSession {
    pub(crate) fn new(stream: StreamWorld, snapshot: WorldSnapshot) -> Self {
        Self {
            stream,
            snapshot,
            pending_origin: None,
            last_blueprint_keys: Vec::new(),
            pending_import: None,
            import_focus: None,
            import_result: None,
            import_success: None,
        }
    }
}

impl Session {
    pub fn new_streamed(seed: u64, preset: BiomePreset) -> Result<Self, StreamError> {
        let mut stream = StreamWorld::new(seed, preset, briccraft_stream::DEFAULT_CACHE_CAPACITY)?;
        let snapshot = stream.prepare_window(WindowOrigin::default())?;
        let mut session = Self::with_world(snapshot.world.clone());
        let ground = session.world.surface_height(48.0, 48.0).max(2.4);
        session.camera.focus([48.0, ground + 2.0, 48.0], 54.0);
        session.world_info = WorldInfo::streamed(seed, preset);
        session.status = format!("{} · an open brick world. Explore to stream new terrain; your constructions stay saved.", preset.name());
        session.streaming = Some(StreamSession::new(stream, snapshot));
        Ok(session)
    }

    pub fn is_streamed(&self) -> bool {
        self.streaming.is_some()
    }
    pub fn stream_stats(&self) -> Option<StreamStats> {
        self.streaming.as_ref().map(|state| state.stream.stats())
    }
    pub fn world_origin(&self) -> WindowOrigin {
        self.streaming
            .as_ref()
            .map_or(WindowOrigin::default(), |state| state.snapshot.origin)
    }
    pub fn global_camera_position(&self) -> [f64; 3] {
        self.world_origin()
            .global_position(self.camera.position.to_array())
    }
    pub fn can_undo(&self) -> bool {
        self.streaming
            .as_ref()
            .map_or_else(|| self.history.can_undo(), |state| state.stream.can_undo())
    }
    pub fn can_redo(&self) -> bool {
        self.streaming
            .as_ref()
            .map_or_else(|| self.history.can_redo(), |state| state.stream.can_redo())
    }

    pub(crate) fn stream_anchor(&self) -> [f32; 3] {
        self.active_vehicle.as_ref().map_or_else(
            || {
                if self.camera.mode == CameraMode::Orbit {
                    self.camera.target.to_array()
                } else {
                    self.camera.position.to_array()
                }
            },
            |active| active.vehicle.position,
        )
    }

    /// Capture after each user command. Any global validation failure restores the last captured
    /// local World rather than leaving unsaved or overlapping local geometry on screen.
    pub fn capture_streamed_edits(&mut self) -> Result<GlobalChange, StreamError> {
        let Some(state) = &mut self.streaming else {
            return Ok(GlobalChange::default());
        };
        match state.stream.capture_edits(&mut state.snapshot, &self.world) {
            Ok(change) => {
                if !self.last_blueprint.is_empty() {
                    state.last_blueprint_keys = self
                        .last_blueprint
                        .iter()
                        .filter_map(|&id| state.snapshot.part_key(id))
                        .collect();
                }
                Ok(change)
            }
            Err(error) => {
                self.world = state.snapshot.world.clone();
                self.history.clear();
                self.scene_generation = self.scene_generation.wrapping_add(1);
                Err(error)
            }
        }
    }

    pub(crate) fn publish_streamed_snapshot(
        &mut self,
        state: &mut StreamSession,
        snapshot: WorldSnapshot,
    ) {
        let old = state.snapshot.origin;
        let delta = Vec3::new(
            (old.x as i128 - snapshot.origin.x as i128) as f32,
            0.0,
            (old.z as i128 - snapshot.origin.z as i128) as f32,
        );
        self.camera.position += delta;
        self.camera.target += delta;
        if let Some(active) = &mut self.active_vehicle {
            for axis in [0, 2] {
                active.vehicle.position[axis] += delta[axis];
            }
            // Body geometry and pivot intentionally stay in their fixed construction frame. The
            // renderer transforms that frame by the current local physics pose, preserving its Rc.
            active.instance_ids = active
                .part_keys
                .iter()
                .filter_map(|&key| snapshot.local_id(key))
                .collect();
        }
        self.last_blueprint = state
            .last_blueprint_keys
            .iter()
            .filter_map(|&key| snapshot.local_id(key))
            .collect();
        self.world = snapshot.world.clone();
        state.snapshot = snapshot;
        self.history.clear();
        self.hit = None;
        self.preview = None;
        self.scene_generation = self.scene_generation.wrapping_add(1);
    }

    pub(crate) fn refresh_streamed_world(&mut self) -> Result<(), StreamError> {
        let Some(mut state) = self.streaming.take() else {
            return Ok(());
        };
        let result = state.stream.prepare_window(state.snapshot.origin);
        match result {
            Ok(snapshot) => {
                self.publish_streamed_snapshot(&mut state, snapshot);
                if let Some(pending) = state.pending_origin {
                    if let Err(error) = state.stream.request_window(pending) {
                        self.streaming = Some(state);
                        return Err(error);
                    }
                }
                self.streaming = Some(state);
                self.update_pick();
                Ok(())
            }
            Err(error) => {
                self.streaming = Some(state);
                Err(error)
            }
        }
    }

    pub(crate) fn change_streamed_history(&mut self, redo: bool) {
        if let Err(error) = self.capture_streamed_edits() {
            self.status = error.to_string();
            return;
        }
        let permit = {
            let state = self.streaming.as_ref().expect("streamed history caller");
            let change = if redo {
                state.stream.redo_change()
            } else {
                state.stream.undo_change()
            };
            match change
                .map(|change| self.approve_global_history(change, redo))
                .transpose()
            {
                Ok(permit) => permit,
                Err(error) => {
                    self.status = error;
                    return;
                }
            }
        };
        let state = self.streaming.as_mut().expect("streamed history caller");
        let result = if redo {
            state.stream.redo()
        } else {
            state.stream.undo()
        };
        self.status = match result {
            Ok(Some(_)) => {
                self.gameplay
                    .commit(permit.expect("global history exists"))
                    .expect("approved global history wallet");
                match self.refresh_streamed_world() {
                    Ok(()) => {
                        if redo {
                            "Construction restored across the open world.".into()
                        } else {
                            "Construction undone across the open world.".into()
                        }
                    }
                    Err(error) => error.to_string(),
                }
            }
            Ok(None) => {
                if redo {
                    "No construction to redo.".into()
                } else {
                    "No earlier construction to undo.".into()
                }
            }
            Err(error) => error.to_string(),
        };
        self.update_pick();
    }

    /// One bounded preparation step per display frame, independent of the fixed simulation loop.
    pub fn advance_streaming(&mut self) {
        self.constrain_map_camera();
        let Some(mut state) = self.streaming.take() else {
            return;
        };
        let was_importing = state.pending_import.is_some() || state.import_focus.is_some();
        let result = self.advance_stream_state(&mut state);
        if let Err(error) = result {
            state.pending_origin = None;
            if let Some(pending) = state.pending_import.take() {
                self.world_info = pending.previous_info;
            }
            state.import_focus = None;
            state.import_success = None;
            if was_importing {
                state.import_result = Some(Err(error.to_string()));
            }
            self.status = format!("World streaming: {error}");
        }
        self.streaming = Some(state);
    }

    fn advance_stream_state(&mut self, state: &mut StreamSession) -> Result<(), StreamError> {
        // Import collision work and render-window generation share the same two-chunk frame budget.
        let generating_budget = if state.pending_import.is_some() { 1 } else { 2 };
        self.advance_import_state(state, 1)?;
        if state.import_focus.is_none() {
            if let Some(rebase) = state.snapshot.rebase_around(self.stream_anchor())? {
                if state.pending_origin != Some(rebase.origin) {
                    state
                        .stream
                        .capture_edits(&mut state.snapshot, &self.world)?;
                    if !self.last_blueprint.is_empty() {
                        state.last_blueprint_keys = self
                            .last_blueprint
                            .iter()
                            .filter_map(|&id| state.snapshot.part_key(id))
                            .collect();
                    }
                    state.stream.request_window(rebase.origin)?;
                    state.pending_origin = Some(rebase.origin);
                }
            } else if state.stream.map_anchor().is_some() && state.pending_origin.take().is_some() {
                // The player returned to the resident view while I/O was pending.
                // Withdraw the obsolete window demand instead of fetching beyond
                // the currently selected geography or publishing a needless rebase.
                state.stream.request_window(state.snapshot.origin)?;
            }
        }
        let Some(origin) = state.pending_origin else {
            return Ok(());
        };
        let progress = state.stream.advance_generation(generating_budget)?;
        if progress.complete {
            let snapshot = state.stream.snapshot(origin)?;
            self.publish_streamed_snapshot(state, snapshot);
            state.pending_origin = None;
            if let Some(focus) = state.import_focus.take() {
                self.camera
                    .focus(state.snapshot.local_position(focus)?, 58.0);
                if let Some(message) = state.import_success.take() {
                    self.status = message.clone();
                    state.import_result = Some(Ok(message));
                }
            }
        }
        Ok(())
    }
}
