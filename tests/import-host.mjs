// Exercise cancellation at the asynchronous storage boundary using real host code.
// No browser or network is needed: canceled work must never reach either service.
import assert from 'node:assert/strict';
import {setImmediate as settle} from 'node:timers/promises';
import {installImports} from '../crates/briccraft-browser/wwwroot/import-host.js';

function fixture() {
    const progress=[],saves=[];
    const runtime={
        briccraftImportProgress(text,value,busy) { progress.push({text,value,busy}); },
        briccraftState() { return JSON.stringify({worldId:'test-world'}); },
    };
    installImports(runtime,{
        saveBeforeImport:()=>new Promise(resolve=>saves.push(resolve)),
        beginImportedWorld:()=>assert.fail('Canceled work must not replace a world'),
    });
    return {progress,saves};
}
function start(action,extra={}) {
    briccraftImportSource(action,JSON.stringify({snapshot:'{}',world_id:'test-world',
        sample:true,latitude:52.516275,longitude:13.377704,radius:160,meters_per_stud:4,
        endpoint:'https://overpass-api.de/api/interpreter',...extra}));
}

const originalFetch=globalThis.fetch;
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
    console.log(JSON.stringify({test:'Import host cancellation',checks:3,sourceFetches:fetches}));
} finally { globalThis.fetch=originalFetch; }
