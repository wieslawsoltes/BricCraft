// A single asynchronous region producer. Rust owns geometry, collisions, source
// identity and edit persistence; this host owns I/O lifetime and publication order.
export function createMapRoamHost(runtime, storage, {
    loadRegion, newId = () => crypto.randomUUID(), vectorUrl = () => undefined,
} = {}) {
    let generation = 0, controller, busy = false, initial;
    let observedKey = '', stoppedKey = '', failedKey = '';
    const read = () => JSON.parse(runtime.briccraftMapState());
    const key = state => `${state.epoch}:${state.world_id}`;
    const cancel = () => { generation++; controller?.abort(); controller = undefined; busy = false; };
    const report = (state, text, active = true) =>
        runtime.briccraftMapProgress(state.epoch, state.world_id, text, active);
    const message = error => String(error?.message || error).slice(0, 520);
    const matches = (state, token) => token === generation && key(read()) === key(state);
    const progress = (state, token) => p => {
        if (matches(state, token)) report(state, `${p.stage}${p.total ? ` · ${p.completed} / ${p.total}` : ''}`);
    };
    function request(anchor, clip, first = false) {
        return {
            latitude: anchor.latitude, longitude: anchor.longitude,
            meters_per_stud: anchor.meters_per_stud, terrain: anchor.terrain !== false,
            origin: anchor.origin || [64, 0, 64], clip, vector_url: vectorUrl(),
            ...(first ? {} : { vertical_scale: {
                reference_elevation_m: anchor.reference_elevation_m,
                requested_meters_per_plate: anchor.meters_per_stud * 0.4,
                effective_meters_per_plate: anchor.meters_per_plate,
            } }),
        };
    }

    async function start(data) {
        cancel();
        initial = { ...data };
        const state = read(), token = generation;
        if (state.epoch !== data.epoch || state.world_id !== data.world_id) { initial = undefined; return; }
        observedKey = key(state); stoppedKey = ''; failedKey = ''; busy = true;
        const abort = new AbortController(); controller = abort;
        report(state, 'Loading your starting area…');
        let locked = false;
        try {
            const result = await loadRegion(request(data, [0, 0, 128, 128], true), abort.signal, progress(state, token));
            if (!matches(state, token)) return;
            if (!runtime.briccraftMapLock(state.epoch, state.world_id))
                throw new Error('Finish the current world operation, then choose Retry in Places');
            locked = true;
            // Save the current state at the publication boundary, not an older
            // snapshot captured before an arbitrarily long network request.
            const snapshot = runtime.briccraftSerializeWorld();
            if (!snapshot) throw new Error('The current world could not be saved');
            await storage.saveBeforeImport(snapshot);
            if (!matches(state, token)) return;
            const terrain = result.report.terrain;
            const anchor = {
                latitude: data.latitude, longitude: data.longitude,
                meters_per_stud: data.meters_per_stud, terrain: data.terrain !== false,
                origin: [64, 0, 64], region_size: 128,
                reference_elevation_m: terrain?.reference_elevation_m ?? 0,
                meters_per_plate: terrain?.effective_meters_per_plate ?? data.meters_per_stud * 0.4,
            };
            if (anchor.terrain && (!terrain || !Number.isFinite(anchor.reference_elevation_m)
                || !Number.isFinite(anchor.meters_per_plate)))
                throw new Error('The generated area did not include a valid elevation reference');
            const name = data.name?.trim() && data.name !== 'Selected location'
                ? `${data.name.trim()} · Free roam`
                : `Free roam ${data.latitude.toFixed(4)}, ${data.longitude.toFixed(4)}`;
            if (!runtime.briccraftBeginMapWorld(state.epoch, state.world_id, newId(), name,
                JSON.stringify(anchor), result.source))
                throw new Error(read().status || 'The starting area could not be published');
            initial = undefined;
            const opened = read(); observedKey = key(opened);
            report(opened, 'Free roam · walk to discover nearby mapped terrain');
            try { await storage.finishImport(); }
            catch (error) {
                if (matches(opened, token))
                    report(opened, `World is ready; automatic save failed: ${message(error)}`);
            }
        } catch (error) {
            if (matches(state, token)) {
                failedKey = key(state);
                report(state, `Starting area: ${message(error)}. Choose Retry in Places.`, false);
            }
        } finally {
            if (locked) runtime.briccraftMapUnlock(state.epoch);
            if (token === generation) { busy = false; controller = undefined; }
        }
    }

    async function extend(state, bounds) {
        busy = true;
        const token = generation, abort = new AbortController(); controller = abort;
        report(state, 'Loading nearby map terrain…');
        try {
            const result = await loadRegion(request(state.anchor, bounds), abort.signal, progress(state, token));
            if (!matches(state, token)) return;
            const current = read();
            // Storage/import operations own the world until their atomic commit.
            // Keep this result only while the same world is still eligible.
            if (current.world_busy || current.import_busy) return;
            if (!runtime.briccraftAppendMapRegion(state.epoch, state.world_id, JSON.stringify(bounds), result.source))
                throw new Error(read().status || 'The map region could not be published');
            const updated = read();
            report(updated, `Free roam · ${updated.regions.length} mapped regions ready`);
        } catch (error) {
            if (matches(state, token)) {
                failedKey = key(state);
                report(state, `Nearby terrain: ${message(error)}. Explore the loaded area or choose Retry in Places.`);
            }
        } finally {
            if (token === generation) { busy = false; controller = undefined; }
        }
    }

    return {
        start,
        tick() {
            const state = read();
            if (!state.world_id) return;
            const currentKey = key(state);
            if (currentKey !== observedKey) {
                cancel(); initial = undefined; failedKey = ''; stoppedKey = ''; observedKey = currentKey;
                if (state.anchor) report(state, 'Free roam · restoring nearby map coverage');
                else report(state, '', false);
            }
            if (busy || initial || !state.anchor || stoppedKey === currentKey || failedKey === currentKey
                || state.world_busy || state.import_busy) return;
            if (!state.active) report(state, 'Free roam · nearby map terrain loads as you walk');
            const next = state.demands?.[0];
            if (next) void extend(state, next);
        },
        stop() {
            const state = read();
            cancel(); initial = undefined; stoppedKey = key(state); failedKey = '';
            report(state, state.anchor ? 'Map streaming paused · loaded terrain and your builds remain available' : 'Map generation cancelled', false);
        },
        retry() {
            const state = read();
            if (busy || state.world_busy || state.import_busy) return;
            if (initial && initial.epoch === state.epoch && initial.world_id === state.world_id) {
                void start(initial); return;
            }
            if (!state.anchor) return;
            failedKey = ''; stoppedKey = ''; report(state, 'Resuming map streaming…');
            this.tick();
        },
        dispose() { cancel(); initial = undefined; },
    };
}
