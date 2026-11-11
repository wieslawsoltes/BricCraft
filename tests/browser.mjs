// Regression gate for the actual FerroUI/Skia site and its worker/storage boundaries.
// Public game commands drive deterministic scenarios; real pointer input verifies the viewport.
import fs from 'node:fs';
import path from 'node:path';
import {open,sleep,assert,differing} from '../FerroUI/scripts/browser/harness.mjs';
const site=process.argv[2],out=process.argv[3] || 'target/browser-checks';
const page=await open(site,{width:1280,height:800,network:true});
const state=async()=>JSON.parse(await page.evaluate('briccraft.briccraftState()'));
const command=async(name,value='')=>page.evaluate(`briccraft.briccraftCommand(${JSON.stringify(name)},${JSON.stringify(value)})`);
const wait=async expression=>page.waitFor(`globalThis.briccraft && (${expression})`,90000);
const current='JSON.parse(briccraft.briccraftState())';
const measurements={};
try {
    await wait(`${current}.ready && !${current}.worldBusy && !document.getElementById('loading')`);
    await sleep(700);
    const start=await state();
    assert(start.parts>5000,'The initial world must contain actual geometry');
    const before=await page.screenshot(undefined,{x:300,y:110,width:640,height:440});
    await page.mouseDown(700,360);
    await page.mouseMove(790,400); await page.mouseMove(850,420); await page.mouseUp(850,420);
    await sleep(500);
    const after=await page.screenshot(undefined,{x:300,y:110,width:640,height:440});
    const pixels=differing(before,after);
    assert(pixels.different>pixels.samples*0.04,'Camera changes must repaint world pixels, including effect uniforms');
    assert((await state()).renderer.uploads===start.renderer.uploads,'Camera-only frames must reuse atlas images');
    measurements.cameraPixelsChanged=pixels.different/pixels.samples;
    await page.screenshot(path.join(out,'island.png'));

    await command('create-world','flat');
    await wait(`${current}.preset === 'Flat' && !${current}.worldBusy`);
    const baseline=(await state()).parts;
    await command('blueprint','rover');
    const built=(await state()).parts;
    assert(built>baseline,'A blueprint must add parts');
    await command('undo'); assert((await state()).parts===baseline,'Streamed undo must remove the entire blueprint');
    await command('redo'); assert((await state()).parts===built,'Streamed redo must restore the blueprint');
    await command('drive','auto');
    const vehicle=(await state()).vehicle;
    assert(vehicle,'A connected rover must activate');
    await page.evaluate('briccraft.briccraftKey("KeyW",true,false,false)');
    await sleep(1800);
    await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false)');
    const driven=(await state()).vehicle;
    assert(Math.hypot(driven[0]-vehicle[0],driven[2]-vehicle[2])>0.1,'Vehicle controls must change its real pose');
    await command('park');
    assert(!(await state()).vehicle,'A flat-world rover must park');

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
    await command('landmark');
    await wait(`${current}.importBusy`);
    await wait(`!${current}.importBusy && ${current}.worldName === 'Brandenburg Gate' && ${current}.attributions.length > 0`);
    await wait(`${current}.worldCount >= 3`);
    const map=await state();
    const exported=await page.evaluate('briccraft.briccraftSerializeWorld()');
    const report=JSON.parse(JSON.parse(exported).world_info.import_report);
    assert(report.height_evidence.some(h=>h.osm_id===518071791 && !h.inferred),'Real map height evidence must survive the full worker/session/save pipeline');
    assert(report.buildings>20 && map.preset==='Flat','Map brickification must publish onto a flat streamed world');
    await page.screenshot(path.join(out,'brandenburg.png'));
    measurements.map={parts:map.parts,importedParts:report.parts,buildings:report.buildings,roads:report.roads,fps:map.fps,renderer:map.renderer};

    // A tiny real GLB verifies Web Worker module loading and transformed colored meshes offline.
    const glb=Array.from(fs.readFileSync('crates/briccraft-import/tests/fixtures/BoxVertexColors.glb'));
    const mesh=await page.evaluate(`new Promise((resolve,reject) => { const worker=new Worker('./import-worker.js',{type:'module'}); const bytes=new Uint8Array(${JSON.stringify(glb)}).buffer; worker.onerror=e=>reject(new Error(e.message)); worker.onmessage=({data})=>{if(data.type==='error'){worker.terminate();reject(new Error(data.message));}if(data.type==='complete'){worker.terminate();resolve(data);}};worker.postMessage({kind:'glb',name:'Colored GLB fixture',size:24,bytes},[bytes]);})`);
    const meshPlan=JSON.parse(mesh.source);
    assert(meshPlan.placements.length>0 && new Set(meshPlan.placements.map(p=>p.color)).size>1,'The shipped worker must preserve mesh colors');
    measurements.mesh={parts:meshPlan.report.parts,triangles:meshPlan.report.source_triangles};

    await page.resize(390,844); await sleep(350); await page.press('g'); await sleep(200);
    await page.screenshot(path.join(out,'compact-world-library.png'));
    const sizes=await page.evaluate(`(() => {const c=document.querySelector('canvas');return {width:innerWidth,height:innerHeight,dpr:devicePixelRatio,canvas:[c.width,c.height]};})()`);
    measurements.compact=sizes;
    assert(Math.abs(sizes.canvas[0]-sizes.width*sizes.dpr)<=2,'Canvas pixel size must match the coherent browser scale');
    assert(page.errors.length===0,`Browser errors: ${page.errors.join('\n')}`);
    console.log(JSON.stringify(measurements,null,2));
    fs.mkdirSync(out,{recursive:true}); fs.writeFileSync(path.join(out,'browser-metrics.json'),JSON.stringify(measurements,null,2));
} catch(error) {
    await page.screenshot(path.join(out,'failure.png')).catch(()=>{});
    console.error('Browser regression:',error);
    console.error('Current state:',await state().catch(()=>null));
    console.error(page.errors.join('\n'));
    process.exitCode=1;
} finally { await page.close(); }
