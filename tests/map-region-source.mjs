// Deterministic live-map transport checks. No public network requests are made.
import assert from 'node:assert/strict';
import test from 'node:test';
import { setImmediate as settle } from 'node:timers/promises';
import { planMapRegion, regionCoordinate } from '../crates/briccraft-browser/wwwroot/map-region-plan.js';
import { MapTileCache } from '../crates/briccraft-browser/wwwroot/map-tile-cache.js';
import { createMapRegionLoader } from '../crates/briccraft-browser/wwwroot/map-region-source.js';
import { SourceHttpError } from '../crates/briccraft-browser/wwwroot/source-errors.js';
import { fetchBounded } from '../crates/briccraft-browser/wwwroot/map-source.js';

const request = { latitude: 52.516275, longitude: 13.377704, meters_per_stud: 4,
    origin: [64,0,64], clip: [0,0,128,128], terrain: false };
const success = { type: 'complete', source: '{"placements":[]}', report: { parts: 1 }, name: 'Test map' };

function workerFactory(workers, automatic = true) {
    return () => {
        const worker = {
            terminated: false, postMessage(data, transfers) {
                this.data = data; this.transfers = transfers;
                if (automatic) queueMicrotask(() => this.onmessage?.({ data: success }));
            }, terminate() { this.terminated = true; },
        };
        workers.push(worker); return worker;
    };
}
const tileCache = () => new MapTileCache({ fetchTile: async () => new Uint8Array([26,0]).buffer });
const loaderOptions = workers => ({ cache: tileCache(), workerFactory: workerFactory(workers), minimumIntervalMs: 0 });

test('adjacent region bounds use one projection with exact shared edges, including negative regions', () => {
    const a = planMapRegion(request), b = planMapRegion({ ...request, clip: [128,0,256,128] });
    assert.equal(a.bounds.east, b.bounds.west);
    assert.equal(a.bounds.north, b.bounds.north);
    assert.deepEqual(a.tiles.map(t => [t.z,t.x,t.y]), [[14,8800,5373],[14,8801,5373]]);
    const west = planMapRegion({ ...request, clip: [-128,0,0,128] });
    assert.equal(west.bounds.east, a.bounds.west);
    assert.deepEqual(regionCoordinate(request, 64, 64), [request.latitude, request.longitude]);
});

test('shared DEM sample lattice, zoom and resolution do not drift between neighboring regions', () => {
    const a = planMapRegion({ ...request, terrain: true }).elevation;
    const b = planMapRegion({ ...request, terrain: true, clip: [128,0,256,128] }).elevation;
    assert(a.width <= 513 && a.height <= 513 && a.tiles.length <= 16);
    assert.equal(a.resolution_m, b.resolution_m);
    assert.equal(a.tiles[0].z, b.tiles[0].z);
    const dx = (a.bounds.east-a.bounds.west)/(a.width-1);
    const dz = (a.bounds.north-a.bounds.south)/(a.height-1);
    assert(Math.abs(dx - (b.bounds.east-b.bounds.west)/(b.width-1)) < 1e-12);
    assert(Math.abs(dz - (b.bounds.north-b.bounds.south)/(b.height-1)) < 1e-12);
    const columns = (b.bounds.west-a.bounds.west)/dx;
    assert(Math.abs(columns-Math.round(columns)) < 1e-8);
});

test('invalid extents, provider templates and excessive tile coverage fail before transport', () => {
    for (const change of [{ latitude: NaN }, { clip: [1,0,129,128] }, { clip: [0,0,1024,1024] },
        { vector_url: 'http://example.test/{z}/{x}/{y}' }, { vector_url: 'https://example.test/map' },
        { vector_url: 'https://user:secret@example.test/{z}/{x}/{y}' }, { latitude:85, meters_per_stud:100 }])
        assert.throws(() => planMapRegion({ ...request, ...change }));
    assert(planMapRegion({ ...request, vector_url: 'https://example.test/map/{z}/{x}/{y}.mvt' })
        .tiles.every(t => t.url.startsWith('https://example.test/map/14/')));
});

test('tile cache honors HTTP freshness, bounded memory and least-recently-used eviction', async () => {
    let now = 0, calls = 0;
    const cache = new MapTileCache({ entries: 2, bytes: 4, now: () => now,
        fetchTile: async (url, options, maximum, inspect) => {
            calls++; assert.equal(options.credentials, 'omit'); assert.equal(maximum,2*1024*1024);
            assert.equal(options.cache, undefined);
            inspect(new Response(null, { headers: { 'Cache-Control': 'public,max-age=10', Age:'2' } }));
            return new Uint8Array([1,2]).buffer;
        } });
    await cache.get('a'); await cache.get('b'); await cache.get('a'); await cache.get('c');
    assert.deepEqual([...cache.items.keys()], ['a','c']); assert.equal(cache.bytes,4);
    assert.equal(calls,3); now=8000; await cache.get('a'); assert.equal(calls,4);
});

test('a cancelled tile read cannot populate memory or bypass the per-tile byte budget', async () => {
    let complete;
    const controller=new AbortController();
    const cache=new MapTileCache({fetchTile:()=>new Promise(resolve=>complete=resolve)});
    const job=cache.get('a',controller.signal); controller.abort(); complete(new ArrayBuffer(4));
    await assert.rejects(job,{name:'AbortError'}); assert.equal(cache.items.size,0);
    const large=new MapTileCache({fetchTile:async()=>new ArrayBuffer(2*1024*1024+1)});
    await assert.rejects(large.get('a'),/2 MB/);
});

test('loader serializes jobs, reuses source bytes and transfers only copies to conversion', async () => {
    const workers=[]; let fetches=0;
    const cache=new MapTileCache({fetchTile:async()=>{fetches++;return new Uint8Array([26,0]).buffer;}});
    const load=createMapRegionLoader({...loaderOptions(workers),cache});
    const [a,b]=await Promise.all([load(request),load({...request,clip:[128,0,256,128]})]);
    assert.equal(a.source,success.source);assert.equal(b.source,success.source);
    assert.equal(fetches,2,'The same two source tiles serve adjacent regions');
    assert.equal(workers.length,2);assert(workers.every(w=>w.terminated));
    assert.equal(workers[0].data.request.clip[0],0);assert.equal(workers[1].data.request.clip[0],128);
    assert.equal(workers[0].data.vectors.bytes.byteLength,4);
    assert([...cache.items.values()].every(v=>v.bytes.byteLength===2));
});

test('queued cancellation performs no source request and leaves an active job intact', async () => {
    const workers=[];let fetches=0;
    const load=createMapRegionLoader({cache:new MapTileCache({fetchTile:async()=>{fetches++;return new ArrayBuffer(1);}}),
        workerFactory:workerFactory(workers,false),minimumIntervalMs:0});
    const first=load(request);await settle();
    const controller=new AbortController();const second=load(request,controller.signal);
    const rejected=assert.rejects(second,{name:'AbortError'});controller.abort();
    assert.equal(workers[0].terminated,false);workers[0].onmessage({data:success});
    await first;await rejected;
    assert.equal(workers.length,1);assert.equal(fetches,2);
});

test('active conversion cancellation terminates the worker and discards late results', async () => {
    const workers=[];const controller=new AbortController();
    const load=createMapRegionLoader({...loaderOptions(workers),workerFactory:workerFactory(workers,false)});
    const job=load(request,controller.signal);await settle();
    const rejected=assert.rejects(job,{name:'AbortError'});controller.abort();await rejected;
    assert.equal(workers[0].terminated,true);workers[0].onmessage({data:success});
});

test('conversion requests retain a snapshot when caller state changes', async () => {
    const workers=[];const data={...request,clip:request.clip.slice()};
    const load=createMapRegionLoader(loaderOptions(workers));const job=load(data);data.clip[0]=999;
    await job;assert.equal(workers[0].data.request.clip[0],0);
});

test('DEM payload and progress reach only the worker, leaving the game UI unlocked', async () => {
    const workers=[],updates=[];
    const load=createMapRegionLoader({...loaderOptions(workers),elevationSource:async(plan,signal,progress)=>{
        assert(plan.tiles.length>0);progress(1,1);return {metadata:{tiles:[]},bytes:new ArrayBuffer(4)};
    }});
    await load({...request,terrain:true},undefined,p=>updates.push(p));
    assert.equal(workers[0].transfers.length,2);
    assert(updates.some(p=>p.stage==='Loading measured terrain'));
});

test('rate-limited sources retain Retry-After and delay the next job without provider substitution', async () => {
    const workers=[],waits=[];let fail=true,clock=0;
    const error=new SourceHttpError(429,'120',0);
    const load=createMapRegionLoader({...loaderOptions(workers),now:()=>clock,
        wait:async milliseconds=>{waits.push(milliseconds);clock+=milliseconds;},
        cache:{get:async()=>{if(fail)throw error;return new ArrayBuffer(1);}}});
    await assert.rejects(load(request), e=>e===error && e.retryAfterMs===120000);
    fail=false;await load(request);assert.equal(waits.at(-1),120000);
    assert.equal(new SourceHttpError(503,'Thu, 01 Jan 1970 00:00:03 GMT',1000).retryAfterMs,2000);
});

test('HTTP source errors preserve status and oversized bodies are cancelled', async () => {
    const original=globalThis.fetch;let cancelled=false;
    try {
        globalThis.fetch=async()=>new Response('',{status:429,headers:{'Retry-After':'15'}});
        await assert.rejects(fetchBounded('https://example.test'),e=>e.status===429 && e.retryAfterMs===15000);
        globalThis.fetch=async()=>new Response(new ReadableStream({cancel(){cancelled=true;}}),
            {headers:{'Content-Length':'1025'}});
        await assert.rejects(fetchBounded('https://example.test',{},1024),/input limit/);
        assert.equal(cancelled,true);
    } finally {globalThis.fetch=original;}
});
