import { fetchBounded } from './map-source.js';
import { throwIfAborted } from './source-errors.js';

// Memory is bounded independently from the browser's normal HTTP cache. The
// latter honors Cache-Control/ETag and keeps repeat views off the tile service.
export class MapTileCache {
    constructor({ entries = 64, bytes = 32 * 1024 * 1024, fetchTile = fetchBounded, now = () => Date.now() } = {}) {
        this.entries = entries;
        this.maximumBytes = bytes;
        this.fetchTile = fetchTile;
        this.now = now;
        this.items = new Map();
        this.bytes = 0;
    }
    async get(url, signal) {
        throwIfAborted(signal);
        const cached = this.items.get(url);
        if (cached && cached.expires > this.now()) {
            this.items.delete(url); this.items.set(url, cached);
            return cached.bytes;
        }
        if (cached) { this.items.delete(url); this.bytes -= cached.bytes.byteLength; }
        // Browser defaults preserve its identifying Referer and HTTP validators.
        let lifetime = 7 * 24 * 60 * 60 * 1000;
        const bytes = await this.fetchTile(url, { signal, credentials: 'omit' }, 2 * 1024 * 1024, response => {
            const control = response.headers.get('Cache-Control') ?? '';
            const maximumAge = /(?:^|,)\s*max-age\s*=\s*"?(\d+)/i.exec(control);
            const expires = Date.parse(response.headers.get('Expires') ?? '');
            if (/(?:^|,)\s*(?:no-cache|no-store)\b/i.test(control)) lifetime = 0;
            else if (maximumAge) lifetime = Math.max(0, Number(maximumAge[1]) - (Number(response.headers.get('Age')) || 0)) * 1000;
            else if (Number.isFinite(expires)) lifetime = Math.max(0, expires - this.now());
        });
        throwIfAborted(signal);
        if (bytes.byteLength > 2 * 1024 * 1024) throw new Error('Vector tile exceeds the 2 MB input limit');
        const old = this.items.get(url);
        if (old) this.bytes -= old.bytes.byteLength;
        this.items.delete(url); this.items.set(url, { bytes, expires: this.now() + lifetime }); this.bytes += bytes.byteLength;
        while (this.items.size > this.entries || this.bytes > this.maximumBytes) {
            const key = this.items.keys().next().value;
            this.bytes -= this.items.get(key).bytes.byteLength; this.items.delete(key);
        }
        return bytes;
    }
}
