// Only the native map's visible tiles are requested. Browser HTTP caching retains
// the provider's Cache-Control/ETag semantics; this is not an offline downloader.
export const DEFAULT_MAP_TILES = 'https://tile.openstreetmap.org/{z}/{x}/{y}.png';
const MAX_BYTES = 1024 * 1024;
const MAX_VISIBLE = 24;

export function tileUrl(template, tile) {
    const [z, x, y] = tile;
    if (![z, x, y].every(Number.isSafeInteger) || z < 0 || z > 19
        || x < 0 || y < 0 || x >= 2 ** z || y >= 2 ** z)
        throw new Error('Invalid map tile coordinates');
    if (!['{z}', '{x}', '{y}'].every(token => template.includes(token)))
        throw new Error('Map tile URLs must contain {z}, {x} and {y}');
    const url = new URL(template.replaceAll('{z}', z).replaceAll('{x}', x).replaceAll('{y}', y));
    if (url.protocol !== 'https:' || url.username || url.password)
        throw new Error('Map tiles require an HTTPS URL without embedded credentials');
    return url.href;
}

async function readPng(response) {
    if (!response.ok) throw new Error(`Map tiles returned HTTP ${response.status}`);
    if (Number(response.headers.get('Content-Length')) > MAX_BYTES)
        throw new Error('Map tile exceeds 1 MiB');
    const reader = response.body.getReader(), chunks = [];
    let length = 0;
    try {
        for (;;) {
            const { done, value } = await reader.read();
            if (done) break;
            length += value.byteLength;
            if (length > MAX_BYTES) { await reader.cancel(); throw new Error('Map tile exceeds 1 MiB'); }
            chunks.push(value);
        }
    } finally { reader.releaseLock(); }
    const bytes = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
    if (length < 24 || ![137, 80, 78, 71, 13, 10, 26, 10].every((v, i) => bytes[i] === v))
        throw new Error('The map provider did not return a PNG tile');
    const header = new DataView(bytes.buffer);
    if (header.getUint32(16) !== 256 || header.getUint32(20) !== 256)
        throw new Error('The map provider must return 256 × 256 tiles');
    return bytes;
}

export function createMapTileLoader(fetcher = globalThis.fetch, cacheLimit = 64) {
    const memory = new Map();
    let generation = 0, controller;
    const cancel = () => { generation++; controller?.abort(); controller = undefined; };
    return {
        cancel,
        async load(tiles, onTile, template = DEFAULT_MAP_TILES) {
            cancel();
            if (!Array.isArray(tiles) || tiles.length > MAX_VISIBLE)
                throw new Error('The visible map exceeds its tile budget');
            const entries = tiles.map(tile => ({ tile, url: tileUrl(template, tile) }));
            const token = generation, request = new AbortController();
            controller = request;
            let next = 0;
            const errors = [];
            async function worker() {
                while (token === generation) {
                    const entry = entries[next++];
                    if (!entry) break;
                    try {
                        let bytes = memory.get(entry.url);
                        if (bytes) { memory.delete(entry.url); memory.set(entry.url, bytes); }
                        else {
                            const response = await fetcher(entry.url, {
                                signal: request.signal, credentials: 'omit', cache: 'default',
                            });
                            bytes = await readPng(response);
                            if (token !== generation) return;
                            memory.set(entry.url, bytes);
                            while (memory.size > cacheLimit) memory.delete(memory.keys().next().value);
                        }
                        if (token === generation) onTile(entry.tile, bytes);
                    } catch (error) {
                        if (token === generation) errors.push(error);
                    }
                }
            }
            await Promise.all(Array.from({ length: Math.min(3, entries.length) }, worker));
            if (token !== generation) return false;
            if (errors.length) throw errors[0];
            return true;
        },
    };
}
