// Exercise cancellation at the asynchronous storage boundary using real host code.
// No browser or network is needed: canceled work must never reach either service.
import assert from 'node:assert/strict';
import {setImmediate as settle} from 'node:timers/promises';
import {installImports} from '../crates/briccraft-browser/wwwroot/import-host.js';

function fixture() {
    const progress=[],saves=[],saveSources=[],messages=[];
    const state={worldId:'test-world',importBusy:false,worldBusy:false};
    const snapshot='{"world_info":{"id":"test-world"},"fresh":true}';
    const runtime={
        briccraftImportProgress(text,value,busy) { state.importBusy=busy;progress.push({text,value,busy}); },
        briccraftState() { return JSON.stringify(state); },
        briccraftSerializeWorld() { return snapshot; },
        briccraftMessage(text) { messages.push(text); },
    };
    installImports(runtime,{
        saveBeforeImport:source=>{saveSources.push(source);return new Promise(resolve=>saves.push(resolve));},
        beginImportedWorld:()=>assert.fail('Canceled work must not replace a world'),
    });
    return {progress,saves,saveSources,messages,state,snapshot};
}
function start(action,extra={}) {
    briccraftImportSource(action,JSON.stringify({snapshot:'{}',world_id:'test-world',
        sample:true,latitude:52.516275,longitude:13.377704,radius:160,meters_per_stud:4,
        endpoint:'https://overpass-api.de/api/interpreter',...extra}));
}

const originalFetch=globalThis.fetch;
const originalDocument=globalThis.document;
const originalWorker=globalThis.Worker;
let fetches=0;
globalThis.fetch=()=>{ fetches++;throw new Error('Unexpected source fetch after cancellation'); };
try {
    for(const action of ['model','map']) {
        const f=fixture();
        start(action); await settle();
        assert.equal(f.saves.length,1);
        assert.equal(f.progress.at(-1).busy,true);
        briccraftCancelImport(); await settle();
        assert.equal(f.progress.at(-1).busy,false);
        const messages=f.progress.length;
        f.saves[0](); await settle();
        assert.equal(f.progress.length,messages,'A canceled save continuation must remain silent');
        assert.equal(f.progress.at(-1).busy,false,'Canceled imports must remain unlocked');
        assert.equal(fetches,0,'Canceled imports must not request source data');
        console.log(`ok    cancel ${action} while its save is pending`);
    }
    const f=fixture();
    start('model'); await settle();
    briccraftCancelImport(); await settle();
    start('map'); await settle();
    assert.equal(f.saves.length,2);
    const messages=f.progress.length;
    f.saves[0](); await settle();
    assert.equal(f.progress.length,messages,'Old work must not emit progress into the replacement job');
    assert.equal(f.progress.at(-1).busy,true,'The replacement job retains its own busy state');
    assert.equal(fetches,0);
    briccraftCancelImport(); await settle();
    f.saves[1](); await settle();
    assert.equal(f.progress.at(-1).busy,false);
    assert.equal(fetches,0);
    console.log('ok    canceled work cannot reuse a replacement job controller');

    const queued=fixture();
    start('map');briccraftCancelImport();await settle();
    assert.equal(queued.saves.length,0,'Cancel before the queued start must prevent a new save');
    assert.equal(queued.state.importBusy,false);
    console.log('ok    cancellation invalidates a queued source request');

    let inputs=[],workers=[];
    globalThis.document={createElement(){
        const input={files:[],addEventListener(type,handler){this.change=handler;},click(){this.clicked=true;}};
        inputs.push(input);return input;
    }};
    globalThis.Worker=class {
        constructor(){this.terminated=false;workers.push(this);}
        terminate(){this.terminated=true;}
        postMessage(data){this.data=data;}
    };
    const file={name:'chosen.obj',size:4,arrayBuffer:async()=>new ArrayBuffer(4)};
    const choose=input=>{input.files=[file];input.change();};

    const delayed=fixture();
    start('model',{sample:false});
    assert.equal(inputs[0].clicked,true,'The file chooser must open synchronously in the user gesture');
    const oldChooser=inputs[0];
    globalThis.fetch=async()=>{fetches++;return new Response(new Uint8Array([1,2,3,4]));};
    start('model');await settle();
    delayed.saves[0]();await settle();await settle();
    assert.equal(workers.length,1);
    assert.equal(delayed.state.importBusy,true);
    choose(oldChooser);await settle();
    assert.equal(workers[0].terminated,false,'A delayed chooser must not stop the newer worker');
    assert.equal(delayed.saves.length,1,'A delayed chooser must not autosave its old snapshot');
    assert.equal(delayed.state.importBusy,true,'The newer import retains its lock');
    assert.match(delayed.messages.at(-1),/earlier import request/);
    briccraftCancelImport();await settle();
    console.log('ok    delayed file selection preserves a newer conversion and its snapshot');

    inputs=[];workers=[];
    const choices=fixture();
    start('model',{sample:false});start('model',{sample:false});
    choose(inputs[0]);await settle();
    assert.equal(choices.saves.length,0,'A newer chooser invalidates an earlier chooser even before conversion starts');
    choose(inputs[1]);
    assert.equal(choices.saves.length,1);
    assert.equal(choices.saveSources[0],choices.snapshot,'Accepted choices capture the current same-world state');
    briccraftCancelImport();choices.saves[0]();await settle();
    assert.equal(workers.length,0);
    console.log('ok    chooser request epochs preserve only the latest choice and refresh its snapshot');

    for(const changed of [{worldBusy:true},{importBusy:true},{worldId:'another-world'}]) {
        inputs=[];
        const guarded=fixture();start('model',{sample:false});
        Object.assign(guarded.state,changed);choose(inputs[0]);await settle();
        assert.equal(guarded.saves.length,0);
        assert.equal(guarded.progress.length,0,'Rejected selections must not clear another operation’s progress');
        assert.match(guarded.messages.at(-1),/world changed or another operation/);
    }
    console.log('ok    busy or changed worlds reject native file selections before any mutation');
    console.log(JSON.stringify({test:'Import host cancellation and chooser races',checks:9,sourceFetches:fetches}));
} finally {
    globalThis.fetch=originalFetch;
    if(originalDocument===undefined)delete globalThis.document;else globalThis.document=originalDocument;
    if(originalWorker===undefined)delete globalThis.Worker;else globalThis.Worker=originalWorker;
}
