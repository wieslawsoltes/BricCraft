import assert from 'node:assert/strict';
import { createMapTileLoader, tileUrl, DEFAULT_MAP_TILES } from '../crates/briccraft-browser/wwwroot/map-tiles.js';
import { createMapRoamHost } from '../crates/briccraft-browser/wwwroot/map-roam-host.js';

const tick = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; };
const png = () => {
    const bytes = new Uint8Array(24); bytes.set([137,80,78,71,13,10,26,10]);
    const view = new DataView(bytes.buffer); view.setUint32(16,256); view.setUint32(20,256); return bytes;
};
let cases = 0;
const test = async (name, fn) => { await fn(); cases++; console.log(`ok ${name}`); };

await test('raster requests are bounded, cache-aware, and reused', async () => {
    const calls = [], tiles = [];
    const loader = createMapTileLoader(async (url, options) => {
        calls.push({url,options}); return new Response(png());
    });
    await loader.load([[2,1,1],[2,2,1]], (key, bytes) => tiles.push({key, bytes}));
    await loader.load([[2,1,1]], () => {});
    assert.equal(calls.length, 2); assert.equal(tiles.length, 2);
    assert(calls.every(c => c.options.cache === 'default' && c.options.credentials === 'omit'));
    await assert.rejects(loader.load(Array.from({length:25},()=>[2,1,1]),()=>{}), /budget/);
    assert.throws(()=>tileUrl(DEFAULT_MAP_TILES,[2,4,0]), /coordinates/);
    assert.throws(()=>tileUrl('http://example.test/{z}/{x}/{y}.png',[2,1,1]), /HTTPS/);
    assert.throws(()=>tileUrl('https://example.test/fixed.png',[2,1,1]), /contain/);
});

await test('the default raster transport observes the active browser fetch adapter', async () => {
    const original = globalThis.fetch;
    const loader = createMapTileLoader();
    const requests = [];
    try {
        globalThis.fetch = async (url, options) => {
            requests.push({url, options}); return new Response(png());
        };
        let published = 0;
        await loader.load([[2, 1, 1]], () => published++);
        assert.equal(requests.length, 1);
        assert.equal(published, 1);
        assert.equal(requests[0].url, 'https://tile.openstreetmap.org/2/1/1.png');
    } finally { globalThis.fetch = original; }
});

await test('stale raster callbacks and invalid images never reach the native view', async () => {
    const pending = deferred(); let published = 0;
    const loader = createMapTileLoader(()=>pending.promise);
    const first = loader.load([[2,1,1]],()=>published++);
    loader.cancel(); pending.resolve(new Response(png())); await first;
    assert.equal(published,0);
    const invalid = createMapTileLoader(async()=>new Response('<html>not a map</html>'));
    await assert.rejects(invalid.load([[2,1,1]],()=>published++), /PNG/);
    assert.equal(published,0);
});

function environment(loadRegion, overrides = {}) {
    let state = { epoch:1,world_id:'previous',anchor:null,regions:[],demands:[],camera:[64,4,64],
        world_busy:false,import_busy:false,active:false,status:'' };
    const calls = { saved:[],begin:0,append:0,unlocks:[],progress:[] };
    let snapshot = 'latest construction';
    const runtime = {
        briccraftMapState:()=>JSON.stringify(state),
        briccraftMapProgress(epoch,id,status,active) {
            if (state.epoch===epoch && state.world_id===id) {
                Object.assign(state,{status,active}); calls.progress.push({status,active});
            }
        },
        briccraftMapLock(epoch,id) {
            if (state.epoch!==epoch || state.world_id!==id || state.world_busy || state.import_busy) return false;
            state.world_busy=true; return true;
        },
        briccraftMapUnlock(epoch) { calls.unlocks.push(epoch); state.world_busy=false; },
        briccraftSerializeWorld:()=>snapshot,
        briccraftBeginMapWorld(epoch,previous,id,name,anchor,source) {
            assert.equal(state.world_busy,true); assert.equal(previous,state.world_id);
            assert.equal(epoch,state.epoch); assert.equal(source,'converted');
            calls.begin++; state={...state,world_id:id,anchor:JSON.parse(anchor),regions:[[0,0,128,128]],demands:[]};
            return true;
        },
        briccraftAppendMapRegion(epoch,id,bounds,source) {
            assert.equal(epoch,state.epoch); assert.equal(id,state.world_id); assert.equal(source,'converted');
            calls.append++; state.regions.push(JSON.parse(bounds)); state.demands=[]; return true;
        },
    };
    const storage = {
        async saveBeforeImport(value) { calls.saved.push(value); },
        async finishImport() {},
        ...overrides,
    };
    const host=createMapRoamHost(runtime,storage,{loadRegion,newId:()=> 'mapped-world'});
    const data={epoch:1,world_id:'previous',latitude:52.516275,longitude:13.377704,meters_per_stud:2,terrain:false,name:'Berlin'};
    return {host,runtime,calls,data,get state(){return state;},set state(value){state=value;},setSnapshot:value=>snapshot=value};
}
const converted = {source:'converted',report:{}};

await test('starting a map saves the current construction only at the publication boundary', async () => {
    const pending=deferred(); const env=environment(()=>pending.promise);
    const job=env.host.start(env.data);
    assert.equal(env.state.world_busy,false); assert.equal(env.calls.begin,0);
    env.setSnapshot('edited while source was loading'); pending.resolve(converted); await job;
    assert.deepEqual(env.calls.saved,['edited while source was loading']);
    assert.equal(env.state.world_id,'mapped-world'); assert.equal(env.state.world_busy,false);
    assert.deepEqual(env.state.anchor.origin,[64,0,64]); assert.equal(env.state.anchor.terrain,false);
    assert.equal(env.calls.begin,1); assert.deepEqual(env.calls.unlocks,[1]);
});

await test('cancelling a source prevents an uncooperative worker from replacing the world', async () => {
    const pending=deferred(); const env=environment(()=>pending.promise);
    const job=env.host.start(env.data); env.host.stop(); pending.resolve(converted); await job;
    assert.equal(env.calls.begin,0); assert.deepEqual(env.calls.saved,[]);
    assert.equal(env.state.world_id,'previous'); assert.equal(env.state.active,false);
});

await test('a same-ID world reload rejects an earlier starting-area result', async () => {
    const pending=deferred(); const env=environment(()=>pending.promise);
    const job=env.host.start(env.data); env.state.epoch=2; env.host.tick();
    pending.resolve(converted); await job;
    assert.equal(env.calls.begin,0); assert.equal(env.state.world_id,'previous');
});

await test('failed autosave keeps the current world and releases its own lock', async () => {
    const env=environment(async()=>converted,{saveBeforeImport:async()=>{throw new Error('quota');}});
    await env.host.start(env.data); assert.equal(env.calls.begin,0);
    assert.equal(env.state.world_id,'previous'); assert.equal(env.state.world_busy,false);
    assert.match(env.state.status,/quota/);
});

await test('cancellation during the queued save cannot publish after unlock', async () => {
    const saved=deferred(); const env=environment(async()=>converted,{saveBeforeImport:()=>saved.promise});
    const job=env.host.start(env.data); await tick(); assert.equal(env.state.world_busy,true);
    env.host.stop(); saved.resolve(); await job;
    assert.equal(env.calls.begin,0); assert.equal(env.state.world_busy,false);
});

await test('failed sources wait for explicit retry instead of repeatedly contacting a provider', async () => {
    let loads=0; const env=environment(async()=>{if(++loads===1)throw new Error('HTTP 429');return converted;});
    await env.host.start(env.data); env.host.tick(); env.host.tick();
    assert.equal(loads,1); assert.match(env.state.status,/429/);
    env.host.retry(); await tick(); await tick();
    assert.equal(loads,2); assert.equal(env.calls.begin,1);
});

await test('nearby regions preserve the geographic datum and keep source generation separate from save actions', async () => {
    const requests=[]; const env=environment(async request=>{requests.push(request);return converted;});
    await env.host.start(env.data);
    env.state.anchor.terrain=true; env.state.anchor.reference_elevation_m=37.5; env.state.anchor.meters_per_plate=2.25;
    env.state.demands=[[128,0,256,128]];
    env.host.tick(); await tick();
    assert.equal(env.calls.append,1); assert.equal(env.calls.begin,1);
    assert.equal(env.calls.saved.length,1); assert.equal(env.state.world_id,'mapped-world');
    assert.deepEqual(requests[1].vertical_scale,{
        reference_elevation_m:37.5,requested_meters_per_plate:0.8,effective_meters_per_plate:2.25,
    });
});

await test('paused streaming drops a delayed region and resumes the same saved world', async () => {
    const pending=deferred(); let loads=0;
    const env=environment(()=>++loads===1?Promise.resolve(converted):pending.promise);
    await env.host.start(env.data); env.state.demands=[[128,0,256,128]]; env.host.tick();
    env.host.stop(); pending.resolve(converted); await tick();
    assert.equal(env.calls.append,0); env.host.tick(); assert.equal(loads,2);
    env.host.retry(); await tick(); assert.equal(env.calls.append,1); assert.equal(env.calls.begin,1);
});

await test('a late save error cannot reactivate cancelled streaming', async () => {
    const saved=deferred(); const env=environment(async()=>converted,{finishImport:()=>saved.promise});
    const job=env.host.start(env.data); await tick(); assert.equal(env.state.world_id,'mapped-world');
    env.host.stop(); saved.reject(new Error('quota')); await job;
    assert.equal(env.state.active,false); assert.match(env.state.status,/paused/);
});

console.log(`${cases} map-host checks passed`);
