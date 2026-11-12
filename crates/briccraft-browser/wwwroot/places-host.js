import { createMapTileLoader, DEFAULT_MAP_TILES } from './map-tiles.js';
import { createMapRoamHost } from './map-roam-host.js';
import { loadMapRegion } from './map-region-source.js';

// Provider URLs can be changed without a game rebuild. No API keys are shipped
// in the application; deployment owners may provide their own HTTPS tile service.
export function mapProviders(store) {
    try {
        store ??= globalThis.localStorage;
        const value = JSON.parse(store?.getItem('briccraft.map-providers.v1') || '{}');
        return {
            raster: typeof value.raster === 'string' ? value.raster : DEFAULT_MAP_TILES,
            vector: typeof value.vector === 'string' ? value.vector : undefined,
        };
    } catch { return { raster: DEFAULT_MAP_TILES }; }
}

export function installPlaces(runtime, storage) {
    const tiles = createMapTileLoader();
    const roaming = createMapRoamHost(runtime, storage, { loadRegion: loadMapRegion,
        vectorUrl: () => mapProviders().vector });
    let timer, tileGeneration = 0, lastPoll = 0;
    globalThis.briccraftMapHost = (action, payload) => {
        const data = JSON.parse(payload || '{}');
        if (action === 'tiles-cancel') {
            tileGeneration++; clearTimeout(timer); tiles.cancel(); return;
        }
        if (action === 'tiles') {
            const generation = ++tileGeneration;
            clearTimeout(timer); tiles.cancel();
            // Dragging moves native images immediately. Network acquisition waits
            // briefly for a settled viewport and never fetches hidden zoom levels.
            timer = setTimeout(async () => {
                try {
                    const done = await tiles.load(data.tiles, (tile, bytes) => {
                        if (generation === tileGeneration)
                            runtime.briccraftMapTile(data.request, ...tile, bytes);
                    }, mapProviders().raster);
                    if (done && generation === tileGeneration)
                        runtime.briccraftMapTileStatus(data.request, 'Drag to explore · tap to choose your starting point');
                } catch (error) {
                    if (generation === tileGeneration)
                        runtime.briccraftMapTileStatus(data.request, `Map unavailable: ${error.message}. Use Retry map or choose coordinates.`);
                }
            }, 100);
            return;
        }
        // Never reenter a RefCell-borrowed native action from its host callback.
        queueMicrotask(() => {
            if (action === 'start') void roaming.start(data);
            else if (action === 'stop') roaming.stop();
            else if (action === 'retry') roaming.retry();
        });
    };
    return {
        tick(time) {
            if (time - lastPoll < 250 || document.visibilityState === 'hidden') return;
            lastPoll = time; roaming.tick();
        },
        dispose() { clearTimeout(timer); tiles.cancel(); roaming.dispose(); },
    };
}
