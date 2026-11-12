// Regression gate for the actual FerroUI/Skia site and its worker/storage boundaries.
// Public game commands drive deterministic scenarios; real pointer input verifies the viewport.
import fs from 'node:fs';
import path from 'node:path';
import {open,sleep,assert,differing} from '../FerroUI/scripts/browser/harness.mjs';
import {checkTerrainArea} from './terrain-browser.mjs';
import {checkMobile} from './mobile.mjs';
import {checkCanvasPresentation} from './graphics-browser.mjs';
import {checkGameplay} from './gameplay-browser.mjs';
import {checkImageQuality} from './quality-browser.mjs';
import {checkDesigns} from './designs-browser.mjs';
import {checkMappedWorlds} from './maps-browser.mjs';
const site=process.argv[2],out=process.argv[3] || 'target/browser-checks';
// SwiftShader is a software GPU. Keep the long input/import journey in the
// player's explicit Performance mode; checkImageQuality separately verifies
// native Auto/Quality surfaces and 4x AA, with actual presented screenshots.
const page=await open(site,{width:1280,height:800,network:true,
    initScript:"localStorage.setItem('briccraft.graphics.v1',JSON.stringify({version:1,mode:2,samples:1}));"});
const state=async()=>JSON.parse(await page.evaluate('briccraft.briccraftState()'));
const command=async(name,value='')=>page.evaluate(`briccraft.briccraftCommand(${JSON.stringify(name)},${JSON.stringify(value)})`);
const wait=async expression=>page.waitFor(`globalThis.briccraft && (${expression})`,90000);
const current='JSON.parse(briccraft.briccraftState())';
const measurements={};
const readiness=()=>page.evaluate(`(() => {
    const loading=document.getElementById('loading');
    return {
        timestamp:performance.now(),visibility:document.visibilityState,
        loading:loading ? {classes:loading.className,text:loading.textContent,display:getComputedStyle(loading).display} : null,
        state:globalThis.briccraft ? JSON.parse(briccraft.briccraftState()) : null,
    };
})()`);
async function presentedSample(milliseconds=2000) {
    const observe=()=>page.evaluate(`(() => {
        const value=JSON.parse(briccraft.briccraftState());
        return {timestamp:performance.now(),rendering:value.rendering,renderer:value.renderer,worldRendering:value.worldRendering};
    })()`);
    const before=await observe();
    await sleep(milliseconds);
    const after=await observe();
    const seconds=(after.timestamp-before.timestamp)/1000;
    const frames=after.rendering.frames-before.rendering.frames;
    return {seconds,frames,fps:frames/Math.max(seconds,0.001),
        effectFrames:after.renderer.effectFrames-before.renderer.effectFrames,
        atlasUploads:after.renderer.uploads-before.renderer.uploads,
        start:before.rendering,end:after.rendering,
        worldStart:before.worldRendering,worldEnd:after.worldRendering};
}
const stage=name=>console.log(new Date().toISOString(),name);
try {
    stage('Waiting for the first native Skia frame');
    await wait(`${current}.ready && !${current}.worldBusy && !document.getElementById('loading')`);
    await sleep(700);
    const start=await state();
    assert(start.rendering.kind==='webgl' && start.rendering.glMajorVersion===3,
        `The GPU regression must exercise Skia WebGL2, got ${JSON.stringify(start.rendering)}`);
    measurements.backend=start.rendering;
    measurements.initial={hudFps:start.fps,rendering:start.rendering,renderer:start.renderer,worldRendering:start.worldRendering};
    const worldLayer=start.worldRendering;
    assert(worldLayer.effectWidth>0 && worldLayer.effectHeight>0,
        'The world must have a nonempty render layer');
    assert(worldLayer.effectWidth*worldLayer.effectHeight<=worldLayer.pixelBudget,
        'Physical shader bounds must respect the adaptive pixel budget');
    assert(worldLayer.effectiveQuality<=worldLayer.requestedQuality && start.renderer.shaderQuality===worldLayer.effectiveQuality,
        'The actual shader must respect the requested and adaptive quality limits');
    measurements.presented=await presentedSample();
    measurements.canvasSizing=await checkCanvasPresentation(page,presentedSample);
    console.log('Presented frames over elapsed wall time:',JSON.stringify(measurements.presented));
    assert(start.parts>5000,'The initial world must contain actual geometry');
    const before=await page.screenshot(undefined,{x:300,y:110,width:640,height:440});
    await page.mouseDown(700,360);
    await page.mouseMove(790,400); await page.mouseMove(850,420); await page.mouseUp(850,420);
    await sleep(500);
    const after=await page.screenshot(undefined,{x:300,y:110,width:640,height:440});
    const pixels=differing(before,after);
    assert(pixels.different>pixels.samples*0.04,'Camera changes must repaint world pixels, including effect uniforms');
    const cameraState=await state();
    assert(Math.hypot(...cameraState.camera.map((value,index)=>value-start.camera[index]))>0.01,
        'Pointer input must change the actual world camera');
    assert(cameraState.renderer.uploads===start.renderer.uploads,'Camera-only frames must reuse atlas images');
    measurements.cameraPixelsChanged=pixels.different/pixels.samples;
    await page.screenshot(path.join(out,'island.png'));

    stage('Creating, editing, and driving in a streamed world');
    await command('create-world','flat');
    await wait(`${current}.preset === 'flat' && !${current}.worldBusy`);
    const baseline=(await state()).parts;
    await command('blueprint','rover');
    const built=(await state()).parts;
    assert(built>baseline,'A blueprint must add parts');
    await command('undo'); assert((await state()).parts===baseline,'Streamed undo must remove the entire blueprint');
    await command('redo'); assert((await state()).parts===built,'Streamed redo must restore the blueprint');
    await command('drive','auto');
    const vehicle=(await state()).vehicle;
    assert(vehicle,'A connected rover must activate');
    assert(await page.evaluate('briccraft.briccraftKey("KeyW",true,false,false)'), 'Drive input must be accepted');
    try {
        // The first body shader may compile on a software GPU. Wait for actual
        // motion, rather than releasing the key before any frame has advanced.
        await wait(`${current}.vehicle && Math.hypot(${current}.vehicle[0]-${vehicle[0]},${current}.vehicle[2]-${vehicle[2]}) > 0.25`);
    } finally { await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false)'); }
    const driven=(await state()).vehicle;
    assert(Math.hypot(driven[0]-vehicle[0],driven[2]-vehicle[2])>0.1,'Vehicle controls must change its real pose');
    await page.screenshot(path.join(out,'rover.png'));
    await command('settings');
    const menuVehicle=await state();
    await wait(`${current}.renderer.effectReuses > ${menuVehicle.renderer.effectReuses}+3`);
    const menuRetained=await state();
    assert(JSON.stringify(menuRetained.vehicle)===JSON.stringify(menuVehicle.vehicle),
        'A menu must suspend vehicle inertia as well as held input');
    assert(JSON.stringify(menuRetained.camera)===JSON.stringify(menuVehicle.camera),
        'A menu must suspend follow-camera motion');
    measurements.menuPause={vehicleUnchanged:true,cameraUnchanged:true,
        effectReuses:menuRetained.renderer.effectReuses-menuVehicle.renderer.effectReuses};
    await command('settings','false');
    await command('park');
    assert(!(await state()).vehicle,'A flat-world rover must park');

    stage('Verifying rebasing and the saved-world library');
    // Importing a legal saved camera far from its window exercises the same streamer as travel.
    await page.evaluate(`(() => { const data=JSON.parse(briccraft.briccraftSerializeWorld()); data.camera.mode='Fly'; data.camera.position=[240.25,20,240.5]; data.camera.target=[240.25,20,245.5]; if(!briccraft.briccraftImportWorld(JSON.stringify(data))) throw new Error('Camera test save rejected'); })()`);
    await wait(`${current}.origin.x !== '0' && ${current}.origin.z !== '0' && ${current}.sceneGeneration > 0`);
    const streamed=await state();
    assert(streamed.cachedChunks<=64,'Chunk residency must remain bounded');
    assert(Math.abs(streamed.globalCamera[0]-240.25)<0.01,'Floating origin must preserve global position');
    await command('save');
    await wait(`${current}.worldCount >= 2`);
    await page.press('g'); await sleep(250);
    assert((await state()).worldsOpen,'G must open the native world library');
    await page.screenshot(path.join(out,'world-library.png'));
    await page.press('Escape');
    stage('Importing the actual Brandenburg OSM extract through the worker');
    const previousWorldId=(await state()).worldId;
    await command('landmark');
    const importedWorld=`${current}.worldId !== ${JSON.stringify(previousWorldId)} && ${current}.worldName === 'Brandenburg Gate' && ${current}.attributions.length > 0`;
    // A small local import can complete between probes; accept its verified
    // new world rather than requiring observation of the transient busy flag.
    await wait(`${current}.importBusy || (${importedWorld})`);
    await wait(`!${current}.importBusy && (${importedWorld})`);
    await wait(`${current}.worldCount >= 3`);
    const map=await state();
    const exported=await page.evaluate('briccraft.briccraftSerializeWorld()');
    const report=JSON.parse(JSON.parse(exported).world_info.import_report);
    assert(report.height_evidence.some(h=>h.osm_id===518071791 && !h.inferred),'Real map height evidence must survive the full worker/session/save pipeline');
    assert(report.buildings>20 && map.preset==='flat','Map brickification must publish onto a flat streamed world');
    await page.screenshot(path.join(out,'brandenburg.png'));
    measurements.map={parts:map.parts,importedParts:report.parts,buildings:report.buildings,roads:report.roads,fps:map.fps,renderer:map.renderer,worldRendering:map.worldRendering};

    // A tiny real GLB verifies Web Worker module loading and transformed colored meshes offline.
    stage('Converting a real GLB in the shipped worker');
    const glb=Array.from(fs.readFileSync('crates/briccraft-import/tests/fixtures/BoxVertexColors.glb'));
    const mesh=await page.evaluate(`new Promise((resolve,reject) => { const worker=new Worker('./import-worker.js',{type:'module'}); const bytes=new Uint8Array(${JSON.stringify(glb)}).buffer; worker.onerror=e=>reject(new Error(e.message)); worker.onmessage=({data})=>{if(data.type==='error'){worker.terminate();reject(new Error(data.message));}if(data.type==='complete'){worker.terminate();resolve(data);}};worker.postMessage({kind:'glb',name:'Colored GLB fixture',size:24,bytes},[bytes]);})`);
    const meshPlan=JSON.parse(mesh.source);
    assert(meshPlan.placements.length>0 && new Set(meshPlan.placements.map(p=>p.color)).size>1,'The shipped worker must preserve mesh colors');
    measurements.mesh={parts:meshPlan.report.parts,triangles:meshPlan.report.source_triangles};

    stage('Generating a complete real area with measured elevation');
    measurements.terrain=await checkTerrainArea({page,state,command,wait,current,out,presentedSample});

    stage('Playing a funded expedition with persistent objectives');
    measurements.gameplay=await checkGameplay({page,state,command,wait,current,out});

    stage('Verifying native-resolution AA and retained paused-world rendering');
    measurements.imageQuality=await checkImageQuality({page,state,command,wait,current,out});

    stage('Capturing, transferring, building, and driving a saved design');
    measurements.designs=await checkDesigns({page,state,command,wait,current,out});

    stage('Checking the compact native world library');
    await page.resize(390,844); await sleep(350); await page.press('g'); await sleep(200);
    await page.screenshot(path.join(out,'compact-world-library.png'));
    const sizes=await page.evaluate(`(() => {const c=document.querySelector('canvas');return {width:innerWidth,height:innerHeight,dpr:devicePixelRatio,canvas:[c.width,c.height]};})()`);
    measurements.compact=sizes;
    assert(Math.abs(sizes.canvas[0]-sizes.width*sizes.dpr)<=2,'Canvas pixel size must match the coherent browser scale');
    stage('Checking native mobile touch navigation and modal isolation');
    measurements.mobile=await checkMobile(page,out);
    stage('Choosing places and walking a dynamically generated mapped world');
    measurements.maps=await checkMappedWorlds({page,state,command,wait,current,out});
    assert(page.errors.length===0,`Browser errors: ${page.errors.join('\n')}`);
    console.log(JSON.stringify(measurements,null,2));
    fs.mkdirSync(out,{recursive:true}); fs.writeFileSync(path.join(out,'browser-metrics.json'),JSON.stringify(measurements,null,2));
} catch(error) {
    console.error('Browser regression:',error);
    // A screenshot may wait for further expensive native draws. Capture the
    // failed readiness state first instead of labeling a later frame as the timeout.
    const diagnostic=await readiness().catch(error=>({probeError:String(error)}));
    console.error('State before failure screenshot:',diagnostic);
    console.error(page.errors.join('\n'));
    process.exitCode=1;
    fs.mkdirSync(out,{recursive:true});
    fs.writeFileSync(path.join(out,'browser-failure.json'),JSON.stringify({error:String(error),diagnostic,measurements},null,2));
    await page.screenshot(path.join(out,'failure.png')).catch(()=>{});
} finally { await page.close(); }
