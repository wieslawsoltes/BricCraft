// Real Chromium IndexedDB coverage for the browser persistence boundary.
// Run: node tests/storage.mjs   (Node 22+, Chrome/Chromium or CHROME=/path/to/chrome)
// This fixture copies the production services unchanged and needs no game/WASM build or npm packages.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {open} from '../FerroUI/scripts/browser/harness.mjs';

const root=path.resolve(fileURLToPath(new URL('..',import.meta.url)));
const directory=fs.mkdtempSync(path.join(os.tmpdir(),'briccraft-storage-'));
const sourceDirectory=path.join(root,'crates/briccraft-browser/wwwroot');
const payload='{"marker":"legacy","seed":18446744073709551608,"origin":{"x":1152921504606846992,"z":-1152921504606847008},"position":{"x":1152921504606847025,"y":18,"z":-1152921504606846961}}';
const legacy='{"version":1,"payload":'+payload+'}';
const legacyUpdated=1700000000123;

for(const name of ['world-db.js','storage-host.js']) {
    fs.copyFileSync(path.join(sourceDirectory,name),path.join(directory,name));
}
fs.writeFileSync(path.join(directory,'index.html'),'<!doctype html><html><head><meta charset="utf-8"><title>Storage regression</title></head><body><input id="world-file" type="file"><script type="module" src="fixture.js"></script></body></html>');
fs.writeFileSync(path.join(directory,'fixture.js'),
    'import * as db from "./world-db.js";\nimport {installStorage} from "./storage-host.js";\n'+
    'const initialPayload='+JSON.stringify(payload)+';\n'+
    'const legacy='+JSON.stringify(legacy)+';\n'+
    'const legacyUpdated='+legacyUpdated+';\n'+String.raw`
// The runtime stub validates only the service contract. Its payload remains opaque text, just as
// the real Rust session keeps i64/u64 values exact instead of routing them through JS Numbers.
const live={id:'initial',name:'Initial',preset:null,body:initialPayload};
const events={messages:[],lists:[],identities:[],imports:[],creates:[],ready:0};
function serialize() {
    return '{"version":2,"world_info":'+JSON.stringify({id:live.id,name:live.name,preset:live.preset})+',"payload":'+live.body+'}';
}
function installSource(source) {
    const parsed=JSON.parse(source);
    if(!parsed.payload) return false;
    const field='"payload":';
    live.body=source.slice(source.indexOf(field)+field.length,-1);
    if(parsed.world_info) {
        live.id=parsed.world_info.id;
        live.name=parsed.world_info.name;
        live.preset=parsed.world_info.preset;
    }
    return true;
}
function editMarker(marker) {
    live.body=live.body.replace(/"marker":"[^"]*"/,'"marker":'+JSON.stringify(marker));
}
const runtime={
    briccraftMessage(message) { events.messages.push(message); },
    briccraftWorldList(source) { events.lists.push(JSON.parse(source)); },
    briccraftState() { return JSON.stringify({worldId:live.id,worldName:live.name}); },
    briccraftWorldIdentity(id,name) {
        live.id=id; live.name=name; events.identities.push({id,name});
    },
    briccraftWorldReady() { events.ready++; },
    briccraftSerializeWorld:serialize,
    briccraftImportWorld(source) {
        events.imports.push(source);
        return installSource(source);
    },
    briccraftCreateWorld(preset,name,seed,id) {
        if(typeof seed!=='string' || !/^[0-9]+$/.test(seed)) return false;
        live.id=id; live.name=name; live.preset=preset;
        live.body=initialPayload.replace(/"seed":[0-9]+/,'"seed":'+seed);
        editMarker('created'); events.creates.push({id,name,preset,seed});
        return true;
    },
};
const storage=installStorage(runtime);

async function seedLegacy() {
    await new Promise((resolve,reject) => {
        const request=indexedDB.open('briccraft-worlds',1);
        request.onupgradeneeded=()=>request.result.createObjectStore('worlds');
        request.onerror=()=>reject(request.error);
        request.onsuccess=()=>{
            const database=request.result;
            const transaction=database.transaction('worlds','readwrite');
            transaction.objectStore('worlds').put({source:legacy,updated:legacyUpdated},'current');
            transaction.oncomplete=()=>{database.close();resolve();};
            transaction.onabort=()=>{database.close();reject(transaction.error);};
        };
    });
}

// Abort one actual IDB transaction before its first world write. No storage implementation is
// mocked: this represents a quota/permission failure and must leave earlier records unchanged.
function failNextWorldWrite(id) {
    const original=IDBObjectStore.prototype.put;
    let fired=false;
    IDBObjectStore.prototype.put=function(value,key) {
        if(!fired && this.name==='worlds' && key===id) {
            fired=true;
            this.transaction.abort();
            throw new DOMException('Injected world storage quota failure','QuotaExceededError');
        }
        return original.call(this,value,key);
    };
    return () => { IDBObjectStore.prototype.put=original; return fired; };
}

globalThis.fixture={db,storage,runtime,live,events,serialize,editMarker,seedLegacy,failNextWorldWrite,legacy,data:{}};
`);

let page;
let passed=0;
async function check(name,run) {
    await run();
    passed++;
    console.log('ok    '+name);
}

try {
    page=await open(directory,{width:480,height:320});
    await page.waitFor('globalThis.fixture',15000);

    await check('legacy current migrates transactionally and retains its original backup',async()=>{
        const result=await page.evaluate(`(async()=>{
            const t=fixture;
            await t.seedLegacy();
            await t.storage.ready();
            const database=await t.db.database();
            const version=database.version;database.close();
            return {version,backup:await t.db.readRecord('current'),migrated:await t.db.readRecord('legacy-island'),
                worlds:await t.db.allRecords(),active:await t.db.activeId(),live:{...t.live},loaded:t.events.imports.at(-1)};
        })()`);
        assert.equal(result.version,2);
        assert.equal(result.backup.source,legacy);
        assert.equal(result.migrated.source,legacy);
        assert.equal(result.migrated.updated,legacyUpdated);
        assert.equal(result.migrated.name,'My original island');
        assert.deepEqual(result.worlds.map(world=>world.id),['legacy-island']);
        assert.equal(result.active,'legacy-island');
        assert.equal(result.live.id,'legacy-island');
        assert.equal(result.loaded,legacy);
    });

    await check('two named worlds save independently with exact u64 and i64 source tokens',async()=>{
        const result=await page.evaluate(`(async()=>{
            const t=fixture;
            await briccraftWorldHost('create',JSON.stringify({preset:'flat',name:'Alpine laboratory',seed:'18446744073709551608',snapshot:t.serialize()}));
            t.editMarker('alpha construction');
            t.data.alpha={id:t.live.id,source:t.serialize()};
            await briccraftSave(t.data.alpha.source);
            await briccraftWorldHost('create',JSON.stringify({preset:'coast',name:'Coastal laboratory',seed:'18446744073709551606',snapshot:t.serialize()}));
            t.editMarker('beta construction');
            t.data.beta={id:t.live.id,source:t.serialize()};
            await briccraftSave(t.data.beta.source);
            return {alpha:await t.db.readRecord(t.data.alpha.id),beta:await t.db.readRecord(t.data.beta.id),
                expected:t.data,worlds:await t.db.allRecords(),active:await t.db.activeId(),creates:t.events.creates,
                requiresExactText:BigInt(JSON.parse(t.data.alpha.source).payload.seed)!==18446744073709551608n};
        })()`);
        assert.notEqual(result.alpha.id,result.beta.id);
        assert.equal(result.alpha.name,'Alpine laboratory');
        assert.equal(result.beta.name,'Coastal laboratory');
        assert.equal(result.alpha.source,result.expected.alpha.source);
        assert.equal(result.beta.source,result.expected.beta.source);
        assert.equal(result.active,result.beta.id);
        assert.equal(result.worlds.length,3);
        assert.equal(result.creates[0].seed,'18446744073709551608');
        assert.equal(result.requiresExactText,true,'Fixture must exceed JavaScript integer precision');
        assert.match(result.alpha.source,/"seed":18446744073709551608/);
        assert.match(result.alpha.source,/"x":1152921504606847025/);
        assert.match(result.alpha.source,/"z":-1152921504606846961/);
    });

    await check('a delayed snapshot cannot overwrite or activate the current different world',async()=>{
        const result=await page.evaluate(`(async()=>{
            const t=fixture;
            const identities=t.events.identities.length;
            await briccraftSave(t.data.alpha.source);
            let rejected=false;
            try { await t.storage.saveBeforeImport(t.data.alpha.source); } catch { rejected=true; }
            return {alpha:await t.db.readRecord(t.data.alpha.id),beta:await t.db.readRecord(t.data.beta.id),
                expected:t.data,active:await t.db.activeId(),live:t.live.id,newIdentities:t.events.identities.length-identities,rejected};
        })()`);
        assert.equal(result.alpha.source,result.expected.alpha.source);
        assert.equal(result.beta.source,result.expected.beta.source);
        assert.equal(result.active,result.beta.id);
        assert.equal(result.live,result.beta.id);
        assert.equal(result.newIdentities,0);
        assert.equal(result.rejected,true,'Import must reject a snapshot belonging to another live world');
    });

    await check('queued import completion saves retain their source identity after a world switch',async()=>{
        const result=await page.evaluate(`(async()=>{
            const t=fixture;
            const switching=briccraftWorldHost('open',JSON.stringify({id:t.data.alpha.id,snapshot:t.serialize()}));
            // Captured while Beta is still live, but serialized after the queued switch to Alpha.
            const finishing=t.storage.finishImport();
            await Promise.all([switching,finishing]);
            return {alpha:await t.db.readRecord(t.data.alpha.id),beta:await t.db.readRecord(t.data.beta.id),
                expected:t.data,active:await t.db.activeId(),live:t.live.id};
        })()`);
        assert.equal(result.alpha.source,result.expected.alpha.source);
        assert.equal(result.beta.source,result.expected.beta.source);
        assert.equal(result.active,result.alpha.id);
        assert.equal(result.live,result.alpha.id);
    });

    await check('copy creates an independent identity without rewriting the exact source payload',async()=>{
        const result=await page.evaluate(`(async()=>{
            const t=fixture;
            const original=t.serialize();
            await briccraftWorldHost('copy',JSON.stringify({name:'Alpine copy',snapshot:original}));
            t.data.copyId=t.live.id;
            const copyBefore=await t.db.readRecord(t.data.copyId);
            t.editMarker('copy-only construction');
            t.data.copySource=t.serialize();
            await briccraftSave(t.data.copySource);
            await briccraftWorldHost('open',JSON.stringify({id:t.data.alpha.id,snapshot:t.serialize()}));
            return {copyBefore,original,copy:await t.db.readRecord(t.data.copyId),alpha:await t.db.readRecord(t.data.alpha.id),
                expected:t.data,live:t.live.id,marker:JSON.parse(t.serialize()).payload.marker};
        })()`);
        assert.notEqual(result.copy.id,result.alpha.id);
        assert.equal(result.copyBefore.name,'Alpine copy');
        assert.equal(result.copyBefore.source,result.original,'Save As metadata must not round payload integers');
        assert.equal(result.copy.source,result.expected.copySource);
        assert.equal(result.alpha.source,result.expected.alpha.source);
        assert.equal(result.live,result.alpha.id);
        assert.equal(result.marker,'alpha construction');
    });

    await check('archive and both restore paths retain source bytes and independent worlds',async()=>{
        const result=await page.evaluate(`(async()=>{
            const t=fixture;
            await briccraftWorldHost('archive',JSON.stringify({id:t.data.copyId,archived:true}));
            const archived=await t.db.readRecord(t.data.copyId);
            const archivedManifest=(await t.db.allRecords()).find(world=>world.id===t.data.copyId);
            await briccraftWorldHost('archive',JSON.stringify({id:t.data.copyId,archived:false}));
            const restored=await t.db.readRecord(t.data.copyId);
            await briccraftWorldHost('archive',JSON.stringify({id:t.data.copyId,archived:true}));
            await briccraftWorldHost('open',JSON.stringify({id:t.data.copyId,snapshot:t.serialize()}));
            return {archived,archivedManifest,restored,opened:await t.db.readRecord(t.data.copyId),
                expected:t.data,live:t.live.id,active:await t.db.activeId(),marker:JSON.parse(t.serialize()).payload.marker};
        })()`);
        assert.equal(result.archived.archived,true);
        assert.equal(result.archivedManifest.archived,true);
        assert.equal(result.restored.archived,false);
        assert.equal(result.opened.archived,false);
        for(const record of [result.archived,result.restored,result.opened]) {
            assert.equal(record.source,result.expected.copySource);
        }
        assert.equal(result.live,result.expected.copyId);
        assert.equal(result.active,result.expected.copyId);
        assert.equal(result.marker,'copy-only construction');
    });

    await check('failed autosave prevents switching and leaves the queue usable for retry',async()=>{
        const result=await page.evaluate(`(async()=>{
            const t=fixture;
            const original=await t.db.readRecord(t.live.id);
            const imports=t.events.imports.length,ready=t.events.ready;
            t.editMarker('unsaved draft after quota failure');
            const draft=t.serialize();
            const restore=t.failNextWorldWrite(t.live.id);
            let fired;
            try { await briccraftWorldHost('open',JSON.stringify({id:t.data.beta.id,snapshot:draft})); }
            finally { fired=restore(); }
            const failed={fired,source:(await t.db.readRecord(original.id)).source,live:t.live.id,active:await t.db.activeId(),
                imports:t.events.imports.length-imports,ready:t.events.ready-ready,messages:[...t.events.messages]};
            await briccraftWorldHost('open',JSON.stringify({id:t.data.beta.id,snapshot:draft}));
            return {failed,original,draft,savedAfterRetry:await t.db.readRecord(original.id),beta:await t.db.readRecord(t.data.beta.id),
                live:t.live.id,active:await t.db.activeId(),expected:t.data};
        })()`);
        assert.equal(result.failed.fired,true);
        assert.equal(result.failed.source,result.original.source,'Aborted autosave must preserve the prior record');
        assert.equal(result.failed.live,result.original.id,'Failed autosave must not replace the live session');
        assert.equal(result.failed.active,result.original.id);
        assert.equal(result.failed.imports,0,'The target world must not be loaded after a failed autosave');
        assert.equal(result.failed.ready,1,'Failure must release the world-switch interaction lock');
        assert(result.failed.messages.some(message=>message.includes('Injected world storage quota failure')));
        assert.equal(result.savedAfterRetry.source,result.draft);
        assert.equal(result.beta.source,result.expected.beta.source);
        assert.equal(result.live,result.beta.id);
        assert.equal(result.active,result.beta.id);
    });

    assert.deepEqual(page.errors,[],`Unexpected browser errors: ${page.errors.join('\n')}`);
    console.log(JSON.stringify({test:'IndexedDB storage services',checks:passed,backend:'Chromium IndexedDB',exactIntegerSource:true}));
} catch(error) {
    console.error('Storage regression:',error);
    if(page) {
        console.error('Runtime events:',await page.evaluate('fixture.events').catch(()=>null));
        console.error(page.errors.join('\n'));
    }
    process.exitCode=1;
} finally {
    if(page) await page.close();
    fs.rmSync(directory,{recursive:true,force:true,maxRetries:3,retryDelay:100});
}
