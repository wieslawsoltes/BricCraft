import { planMapRegion } from './map-region-plan.js';
import { MapTileCache } from './map-tile-cache.js';
import { fetchElevationPlan } from './elevation-source.js';
import { abortError, SourceHttpError, throwIfAborted } from './source-errors.js';

function delay(milliseconds, signal) {
    if (milliseconds <= 0) { throwIfAborted(signal); return Promise.resolve(); }
    return new Promise((resolve, reject) => {
        const abort = () => { clearTimeout(timer); reject(signal?.reason ?? abortError()); };
        const timer = setTimeout(() => { signal?.removeEventListener('abort', abort); resolve(); }, milliseconds);
        signal?.addEventListener('abort', abort, { once: true });
        if (signal?.aborted) abort();
    });
}

function convert(workerFactory, request, vectors, elevation, signal, progress) {
    throwIfAborted(signal);
    return new Promise((resolve, reject) => {
        const worker = workerFactory();
        let finished = false;
        const finish = (error, value) => {
            if (finished) return;
            finished = true;
            signal?.removeEventListener('abort', abort);
            worker.terminate();
            if (error) reject(error); else resolve(value);
        };
        const abort = () => finish(signal?.reason ?? abortError());
        signal?.addEventListener('abort', abort, { once: true });
        worker.onerror = event => finish(new Error(event.message || 'Mapped terrain conversion failed'));
        worker.onmessage = ({ data }) => {
            if (finished) return;
            try {
                throwIfAborted(signal);
                if (data.type === 'progress') progress(data.progress);
                else if (data.type === 'error') finish(new Error(data.message));
                else if (data.type === 'complete') {
                    if (typeof data.source !== 'string' || !data.report || typeof data.report !== 'object')
                        throw new Error('The map conversion worker returned an invalid plan');
                    finish(null, { source: data.source, report: data.report, name: data.name });
                }
            } catch (error) { finish(error); }
        };
        if (signal?.aborted) { abort(); return; }
        const transfers = [vectors.bytes];
        if (elevation?.bytes) transfers.push(elevation.bytes);
        try { worker.postMessage({ request, vectors, elevation }, transfers); }
        catch (error) { finish(error); }
    });
}

export function createMapRegionLoader({
    cache = new MapTileCache(), elevationSource = fetchElevationPlan,
    workerFactory = () => new Worker(new URL('./map-region-worker.js', import.meta.url), { type: 'module' }),
    now = () => Date.now(), wait = delay, minimumIntervalMs = 250,
} = {}) {
    // A single conversion/fetch job is active. Cancellation reaches both the
    // request bodies and worker; a queued, cancelled region never contacts a source.
    let tail = Promise.resolve(), nextStart = 0;
    return function load(request, signal, progress = () => {}) {
        const snapshot = { ...request, origin: request.origin?.slice(), clip: request.clip?.slice(),
            vertical_scale: request.vertical_scale ? { ...request.vertical_scale } : undefined };
        const job = tail.then(async () => {
            throwIfAborted(signal);
            const plan = planMapRegion(snapshot);
            await wait(Math.max(0, nextStart - now()), signal);
            throwIfAborted(signal);
            nextStart = now() + minimumIntervalMs;
            try {
                const buffers = [];
                let total = 0;
                for (const [index, tile] of plan.tiles.entries()) {
                    progress({ stage: 'Loading mapped streets and buildings', completed: index, total: plan.tiles.length });
                    const bytes = await cache.get(tile.url, signal);
                    throwIfAborted(signal);
                    total += bytes.byteLength;
                    if (total > 8 * 1024 * 1024) throw new Error('Vector tiles exceed the 8 MB region budget');
                    buffers.push(bytes);
                }
                const bytes = new Uint8Array(total);
                let offset = 0;
                const tiles = plan.tiles.map((tile, index) => {
                    const input = new Uint8Array(buffers[index]);
                    bytes.set(input, offset);
                    const descriptor = { z: tile.z, x: tile.x, y: tile.y, offset, length: input.length };
                    offset += input.length;
                    return descriptor;
                });
                const elevation = plan.elevation ? await elevationSource(plan.elevation, signal, (completed, total) => {
                    progress({ stage: 'Loading measured terrain', completed, total });
                }) : null;
                throwIfAborted(signal);
                progress({ stage: 'Building the next mapped region', completed: 0, total: 1 });
                return await convert(workerFactory, { ...snapshot, bounds: plan.bounds, tiles },
                    { bytes: bytes.buffer }, elevation, signal, progress);
            } catch (error) {
                if (error instanceof SourceHttpError && [429, 503].includes(error.status))
                    nextStart = Math.max(nextStart, now() + Math.max(30_000, error.retryAfterMs));
                throw error;
            }
        });
        tail = job.catch(() => {});
        return job;
    };
}

export const loadMapRegion = createMapRegionLoader();
