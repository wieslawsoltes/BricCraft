// Network fixtures for browser regressions. Mapped world generation uses the
// recorded real Shortbread bytes. Picker images use an intentionally synthetic
// raster pattern to verify native PNG decoding/hit testing without tile scraping.
import fs from 'node:fs';
import zlib from 'node:zlib';

function crc32(bytes) {
    let value = 0xffffffff;
    for (const byte of bytes) {
        value ^= byte;
        for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ ((value & 1) ? 0xedb88320 : 0);
    }
    return (value ^ 0xffffffff) >>> 0;
}
function chunk(type, bytes) {
    const name = Buffer.from(type), head = Buffer.alloc(4), tail = Buffer.alloc(4);
    head.writeUInt32BE(bytes.length); tail.writeUInt32BE(crc32(Buffer.concat([name, bytes])));
    return Buffer.concat([head, name, bytes, tail]);
}
function mapTile() {
    const rows = Buffer.alloc(256 * (1 + 256 * 3));
    for (let y = 0; y < 256; y++) for (let x = 0; x < 256; x++) {
        const road = Math.abs(x - 128) < 12 || Math.abs(y - 128) < 12;
        const river = x > 216 && x < 246;
        const block = (x % 64 > 9 && x % 64 < 50) && (y % 64 > 9 && y % 64 < 50);
        const color = river ? [143,193,207] : road ? [255,254,244] : block ? [204,192,169] : [220,230,205];
        rows.set(color, y * 769 + 1 + x * 3);
    }
    const header = Buffer.alloc(13); header.writeUInt32BE(256); header.writeUInt32BE(256,4); header[8]=8;header[9]=2;
    return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),
        chunk('IDAT',zlib.deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
}

export async function installMapFixtures(page) {
    const base = 'tests/fixtures/map-regions';
    const provenance = JSON.parse(fs.readFileSync(`${base}/provenance.json`,'utf8'));
    const vectors = Object.fromEntries(provenance.files.map(file=>[
        file.url, fs.readFileSync(`${base}/${file.file}`).toString('base64'),
    ]));
    const area = fs.readFileSync('crates/briccraft-browser/wwwroot/assets/maps/brandenburg.json').toString('base64');
    const raster = mapTile().toString('base64');
    await page.evaluate(`(() => {
        const original=globalThis.fetch;
        const vectors=${JSON.stringify(vectors)},raster=${JSON.stringify(raster)},area=${JSON.stringify(area)};
        const bytes=value=>Uint8Array.from(atob(value),c=>c.charCodeAt(0));
        globalThis.__mapRequests=[];
        globalThis.fetch=async (input,options) => {
            const url=String(input?.url ?? input);
            if(url.startsWith('https://tile.openstreetmap.org/')) {
                __mapRequests.push({kind:'raster',url});
                return new Response(bytes(raster),{headers:{'Content-Type':'image/png','Cache-Control':'public,max-age=604800'}});
            }
            if(url.startsWith('https://vector.openstreetmap.org/')) {
                __mapRequests.push({kind:'vector',url});
                if(!vectors[url]) throw new Error('The browser regression requested a vector tile outside its recorded source fixtures: '+url);
                return new Response(bytes(vectors[url]),{headers:{'Content-Type':'application/vnd.mapbox-vector-tile','Cache-Control':'public,max-age=604800'}});
            }
            if(url.startsWith('https://overpass-api.de/api/interpreter')) {
                __mapRequests.push({kind:'area',url,body:String(options?.body || '')});
                return new Response(bytes(area),{headers:{'Content-Type':'application/json'}});
            }
            return original(input,options);
        };
        globalThis.__restoreMapFetch=()=>{globalThis.fetch=original;delete globalThis.__restoreMapFetch;};
    })()`);
    return provenance;
}
