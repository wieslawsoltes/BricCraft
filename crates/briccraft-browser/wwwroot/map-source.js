const CACHE='briccraft-map-sources-v1';
const MAX_BYTES=32*1024*1024;
export function mapBounds(latitude,longitude,radius) {
    if(!Number.isFinite(latitude)||!Number.isFinite(longitude)||!Number.isFinite(radius)
        ||Math.abs(latitude)>85||Math.abs(longitude)>180||radius<20||radius>1500)
        throw new Error('The requested map coordinates or radius are outside the supported range');
    const dy=radius/6378137*180/Math.PI, dx=dy/Math.cos(latitude*Math.PI/180);
    if(longitude-dx < -180 || longitude+dx > 180) throw new Error('Choose a location away from the date line');
    return [latitude-dy,longitude-dx,latitude+dy,longitude+dx];
}
export function overpassQuery(bounds) {
    const b=bounds.map(v => v.toFixed(7)).join(',');
    const selectors=['way[building]','relation[building]','way["building:part"]','relation["building:part"]',
        'way[highway]','way[natural=water]','relation[natural=water]','way[waterway]',
        'relation[waterway=riverbank]','way[landuse]','relation[landuse]',
        'way[natural~"^(wood|scrub|grassland|heath|sand|beach|bare_rock|wetland|glacier)$"]',
        'relation[natural~"^(wood|scrub|grassland|heath|sand|beach|bare_rock|wetland|glacier)$"]',
        'way[leisure~"^(park|garden|golf_course|pitch|recreation_ground)$"]',
        'relation[leisure~"^(park|garden|golf_course|pitch|recreation_ground)$"]'];
    return `[out:json][timeout:25][maxsize:16777216];(${selectors.map(s=>`${s}(${b});`).join('')});out tags geom;`;
}
export async function fetchBounded(url,options={},maximum=MAX_BYTES) {
    const response=await fetch(url,options);
    if(!response.ok) {
        const retry=response.headers.get('Retry-After');
        throw new Error(`Source returned HTTP ${response.status}${retry ? `; retry after ${retry}` : ''}`);
    }
    const tooLarge=()=>new Error(`Source exceeds the ${Math.ceil(maximum/1024/1024)} MB input limit`);
    if(Number(response.headers.get('Content-Length'))>maximum) throw tooLarge();
    const reader=response.body.getReader(), chunks=[]; let size=0;
    try {
        for(;;) {
            const {done,value}=await reader.read(); if(done) break;
            size+=value.byteLength;
            if(size>maximum) { await reader.cancel(); throw tooLarge(); }
            chunks.push(value);
        }
    } finally { reader.releaseLock(); }
    const bytes=new Uint8Array(size); let offset=0;
    for(const chunk of chunks) { bytes.set(chunk,offset); offset+=chunk.length; }
    return bytes.buffer;
}
export async function fetchMap(data,signal) {
    const endpoint=new URL(data.endpoint);
    if(endpoint.protocol!=='https:') throw new Error('Use an HTTPS Overpass endpoint');
    const query=overpassQuery(mapBounds(data.latitude,data.longitude,data.radius));
    const key=new Request(`${location.origin}${location.pathname}map-cache/${encodeURIComponent(endpoint.href)}/${encodeURIComponent(query)}`);
    let cache;
    try { cache=await caches.open(CACHE); const hit=await cache.match(key); if(hit) return hit.arrayBuffer(); } catch { /* Storage may be disabled. */ }
    const bytes=await fetchBounded(endpoint.href,{method:'POST',body:new URLSearchParams({data:query}),signal});
    if(cache) {
        try {
            const keys=await cache.keys();
            for(const old of keys.slice(0,Math.max(0,keys.length-7))) await cache.delete(old);
            await cache.put(key,new Response(bytes,{headers:{'Content-Type':'application/json'}}));
        } catch { /* Cache quota never prevents a valid import. */ }
    }
    return bytes;
}
