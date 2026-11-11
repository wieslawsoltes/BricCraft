// Exercise the real file adapter at its DOM and asynchronous file boundaries.
// The domain parser stays behind the runtime API; the host must preserve bytes.
import assert from 'node:assert/strict';
import {installDesignFiles} from '../crates/briccraft-browser/wwwroot/design-host.js';

const LIMIT=256*1024;
// A JS parse/stringify round trip would silently round both source coordinates.
// Unicode and CRLF also distinguish the original UTF-8 payload from a rewrite.
const source=String.raw`{
  "format":"briccraft.design",
  "version":1,
  "design":{
    "id":7,"name":"Harbor/Bridge: \"Öresund\"\\?*<>|",
    "parts":[{"part_id":0,"color":4,"position":[0,0,0],"rotation":0}],
    "dimensions":[1,3,1],"components":1,
    "metadata":{"source_world_id":"source-world","source_world_name":"Malmö 🧱",
      "source_origin":[9007199254740993,18,-9007199254740993],"capture":"connected_assembly"}
  }
}
`.replaceAll('\n','\r\n');

function fixture() {
    const state={worldId:'destination-world'},imports=[],messages=[];
    const elements=[],children=new Set(),downloads=[],urls=new Map(),revoked=[],timers=[];
    const runtime={
        briccraftState:()=>JSON.stringify(state),
        briccraftImportDesign:text=>{imports.push(text);return true;},
        briccraftMessage:text=>messages.push(text),
    };
    globalThis.document={
        body:{append(element){children.add(element);}},
        createElement(tag){
            assert.ok(tag==='input' || tag==='a',`Unexpected host element: ${tag}`);
            const listeners=new Map();
            const element={tag,files:[],clicks:0,
                addEventListener(type,handler,options={}){
                    const entries=listeners.get(type) || [];
                    entries.push({handler,once:options.once});listeners.set(type,entries);
                },
                dispatch(type){
                    const entries=listeners.get(type) || [];
                    listeners.set(type,entries.filter(entry=>!entry.once));
                    return Promise.all(entries.map(({handler})=>handler({type,target:element})));
                },
                click(){
                    assert.ok(children.has(element),'The native picker/download must be attached before activation');
                    element.clicks++;
                    if(tag==='a')downloads.push({href:element.href,name:element.download});
                },
                remove(){children.delete(element);},
            };
            elements.push(element);return element;
        },
    };
    globalThis.URL={
        createObjectURL(blob){
            assert.ok(blob instanceof Blob,'Export must provide a real Blob');
            const url=`blob:design-test/${urls.size}`;urls.set(url,blob);return url;
        },
        revokeObjectURL(url){revoked.push(url);},
    };
    globalThis.setTimeout=(callback,delay)=>{timers.push({callback,delay});return timers.length;};
    installDesignFiles(runtime);
    return {state,imports,messages,elements,children,downloads,urls,revoked,timers,
        open(){
            globalThis.briccraftDesignFile('import');
            const input=elements.at(-1);
            assert.equal(input.tag,'input');
            assert.equal(input.clicks,1,'Picker activation must occur synchronously in the initiating gesture');
            return input;
        },
    };
}

const replaced=['document','URL','setTimeout','briccraftDesignFile'];
const originals=new Map(replaced.map(name=>[name,Object.getOwnPropertyDescriptor(globalThis,name)]));
try {
    const exported=fixture();
    globalThis.briccraftDesignFile('export',source);
    assert.equal(exported.downloads.length,1);
    const download=exported.downloads[0],blob=exported.urls.get(download.href);
    assert.equal(blob.type,'application/json');
    assert.deepEqual(Buffer.from(await blob.arrayBuffer()),Buffer.from(source,'utf8'),
        'Export must retain original bytes, including i64 coordinates beyond Number precision');
    assert.ok(download.name.endsWith('.bricdesign'));
    assert.ok(download.name.length<=64+'.bricdesign'.length);
    assert.doesNotMatch(download.name,/[<>:"/\\|?*\u0000-\u001f]/,'Download filename must be portable');
    assert.equal(exported.children.size,0,'The temporary download link must be removed');
    assert.equal(exported.messages.length,0);
    assert.deepEqual(exported.revoked,[],'The Blob URL must survive the initial download activation');
    assert.equal(exported.timers.length,1);
    assert.ok(exported.timers[0].delay>0 && exported.timers[0].delay<=60_000);
    exported.timers[0].callback();
    assert.deepEqual(exported.revoked,[download.href]);
    console.log('ok    export retains exact UTF-8/i64 bytes, uses a safe filename and releases its temporary resources');

    const oversized=fixture(),tooLarge=oversized.open();
    let reads=0;
    tooLarge.files=[{size:LIMIT+1,text:async()=>{reads++;return source;}}];
    await tooLarge.dispatch('change');
    assert.equal(reads,0,'The byte cap must be enforced before reading a file');
    assert.equal(oversized.imports.length,0);
    assert.match(oversized.messages.at(-1),/256 KiB/);
    assert.equal(oversized.children.size,0);
    console.log('ok    oversized files are rejected before I/O or a domain call');

    const delayed=fixture(),pendingPicker=delayed.open();
    let finishRead,readStarted=false;
    const pendingText=new Promise(resolve=>{finishRead=resolve;});
    pendingPicker.files=[{size:Buffer.byteLength(source),text(){readStarted=true;return pendingText;}}];
    const pendingChange=pendingPicker.dispatch('change');
    assert.equal(readStarted,true);
    assert.equal(delayed.imports.length,0);
    delayed.state.worldId='different-world';
    finishRead(source);
    await pendingChange;
    assert.equal(delayed.imports.length,0,'A file read started in another world must not mutate the current library');
    assert.match(delayed.messages.at(-1),/world changed/i);
    assert.equal(delayed.children.size,0);
    console.log('ok    world changes during an awaited file read reject the stale import');

    const canceled=fixture(),canceledPicker=canceled.open();
    await canceledPicker.dispatch('cancel');
    assert.equal(canceled.children.size,0,'Canceling a native picker must remove its hidden input');
    assert.equal(canceled.imports.length,0);
    assert.equal(canceled.messages.length,0);
    console.log('ok    cancel removes the picker without a domain mutation or error');

    const valid=fixture(),validPicker=valid.open();
    // Valid JSON may contain trailing whitespace. Exercise the inclusive byte
    // boundary, with non-ASCII text making character count different from bytes.
    const atLimit=source+' '.repeat(LIMIT-Buffer.byteLength(source,'utf8'));
    assert.equal(Buffer.byteLength(atLimit,'utf8'),LIMIT);
    validPicker.files=[{size:LIMIT,text:async()=>atLimit}];
    await validPicker.dispatch('change');
    assert.deepEqual(valid.imports,[atLimit],'Accepted files must reach the domain parser unchanged');
    assert.equal(valid.messages.length,0);
    assert.equal(valid.children.size,0);
    console.log('ok    a valid file at the byte limit reaches the runtime unchanged');
    console.log(JSON.stringify({test:'Design file host services',checks:5,
        exactI64Export:true,preReadByteCap:true,worldRaceGuard:true,cancelCleanup:true,unchangedImport:true}));
} finally {
    for(const [name,descriptor] of originals) {
        if(descriptor)Object.defineProperty(globalThis,name,descriptor);
        else delete globalThis[name];
    }
}
