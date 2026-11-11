import assert from 'node:assert/strict';
import {GRAPHICS_KEY,graphicsPreferences,installGraphicsPreferences} from '../crates/briccraft-browser/wwwroot/graphics-preferences.js';

assert.deepEqual(graphicsPreferences(null),{version:1,mode:0,samples:2});
assert.deepEqual(graphicsPreferences({version:2,mode:1,samples:4}),{version:1,mode:0,samples:2});
assert.deepEqual(graphicsPreferences({version:1,mode:9,samples:3}),{version:1,mode:0,samples:2});
assert.deepEqual(graphicsPreferences({version:1,mode:1,samples:4}),{version:1,mode:1,samples:4});

let saved=JSON.stringify({version:1,mode:1,samples:4}),writes=0;
const calls=[],actual={mode:0,samples:2};
const host={localStorage:{
    getItem(key){assert.equal(key,GRAPHICS_KEY);return saved;},
    setItem(key,value){assert.equal(key,GRAPHICS_KEY);saved=value;writes++;},
}};
const runtime={briccraftCommand(name,value){
    calls.push([name,value]);
    if(name==='resolution') actual.mode=Number(value);
    if(name==='antialiasing') actual.samples=Number(value);
    // The real native dispatcher persists each change synchronously. Restoring
    // the first field must not replace the still-pending saved second field.
    host.briccraftSaveGraphics(actual.mode,actual.samples);
}};
const adapter=installGraphicsPreferences(runtime,host);
adapter.restore();
assert.deepEqual(calls,[['resolution','1'],['antialiasing','4']]);
assert.deepEqual(adapter.current(),{version:1,mode:1,samples:4});
assert.equal(writes,0);
host.briccraftSaveGraphics(2,1);
assert.deepEqual(JSON.parse(saved),{version:1,mode:2,samples:1});
assert.equal(writes,1);
adapter.dispose();
assert.equal(host.briccraftSaveGraphics,undefined);

const blocked={get localStorage(){throw new Error('Storage disabled');}};
const fallback=[];
const safe=installGraphicsPreferences({briccraftCommand:(...args)=>fallback.push(args)},blocked);
safe.restore();
assert.deepEqual(fallback,[['resolution','0'],['antialiasing','2']]);
assert.doesNotThrow(()=>blocked.briccraftSaveGraphics(1,4));
safe.dispose();

const damaged={localStorage:{getItem:()=>'{bad',setItem:()=>{}}};
const recovered=installGraphicsPreferences({briccraftCommand:()=>{}},damaged);
assert.deepEqual(recovered.current(),{version:1,mode:0,samples:2});
recovered.dispose();
console.log(JSON.stringify({test:'Graphics preferences',nativeDefault:true,exactRestore:true,storageOptional:true,malformedRecovery:true}));
