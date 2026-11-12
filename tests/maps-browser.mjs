import path from 'node:path';
import { assert } from '../FerroUI/scripts/browser/harness.mjs';
import { installMapFixtures } from './map-fixtures.mjs';
import { checkMapPicker } from './map-picker-browser.mjs';

export async function checkMappedWorlds(context) {
    const {page,state,command,wait,current,out}=context;
    const mapCurrent='JSON.parse(briccraft.briccraftMapState())';
    const mapped=async()=>JSON.parse(await page.evaluate('briccraft.briccraftMapState()'));
    const serialized=()=>page.evaluate('briccraft.briccraftSerializeWorld()');
    const provenance=await installMapFixtures(page);
    const result={sourceFixture:provenance.source};
    try {
        result.picker=await checkMapPicker(context);
        // The explicit build action uses the same selected point and complete
        // area conversion as the existing importer; it preserves the old save.
        const previous=(await state()).worldId;
        await command('map-view',JSON.stringify({latitude:52.516275,longitude:13.377704,zoom:16,
            radius:160,meters_per_stud:4,terrain:false}));
        await command('map-build');
        await wait(`${current}.worldId !== ${JSON.stringify(previous)} && !${current}.worldBusy && !${current}.importBusy`);
        const built=JSON.parse(await serialized());
        const report=JSON.parse(built.world_info.import_report);
        assert(report.buildings>20 && report.roads>0,'The selected map area must generate real recorded OSM geometry');
        assert(Math.abs(report.projection.latitude-52.516275)<1e-8 && Math.abs(report.projection.longitude-13.377704)<1e-8,
            'Building the selection must use the chosen geographic anchor');
        result.buildArea={buildings:report.buildings,roads:report.roads,parts:report.parts,projection:report.projection};

        // Start the production vector-fetch -> dedicated worker -> Rust source
        // publication path. Recorded provider bytes replace network I/O only.
        await command('map-view',JSON.stringify({latitude:52.516275,longitude:13.377704,zoom:16,
            meters_per_stud:2,terrain:false}));
        const beforeRoam=(await state()).worldId;
        await command('map-roam');
        await wait(`${mapCurrent}.anchor && ${current}.worldId !== ${JSON.stringify(beforeRoam)} && !${current}.worldBusy`);
        const initial=await mapped();
        assert((await state()).mode==='WALK','Free roam must start with the collision-aware walking controller');
        assert(initial.regions.length===1 && initial.anchor.region_size===128,'The starting view must publish one complete bounded region');
        assert(initial.source_parts>0 && (await state()).parts>0,'The free-roam world must contain converted source geometry');
        const firstSave=JSON.parse(await serialized());
        const firstReport=JSON.parse(firstSave.world_info.import_report);
        assert(firstReport.buildings>0 && firstReport.inferred_heights>0,
            'Shortbread footprints must become real buildings, with missing source heights explicitly inferred');
        await command('blueprint','rover');
        const beforeWalk=JSON.parse(await serialized());
        assert(beforeWalk.stream.edits.length>0,'A live map must support persistent player construction');
        const construction=JSON.stringify(beforeWalk.stream.edits);
        const nextUserId=beforeWalk.stream.next_user_id;
        // Put the saved walker on the east-west road in the recorded fixture,
        // near a region edge; subsequent movement is actual held key input.
        beforeWalk.camera.mode='Walk';
        beforeWalk.camera.position=[88-Number(beforeWalk.origin.x),4,64-Number(beforeWalk.origin.z)];
        beforeWalk.camera.target=[100-Number(beforeWalk.origin.x),4,64-Number(beforeWalk.origin.z)];
        beforeWalk.camera.yaw=Math.PI/2; beforeWalk.camera.pitch=-0.06;
        assert(await page.evaluate(`briccraft.briccraftImportWorld(${JSON.stringify(JSON.stringify(beforeWalk))})`),
            'A mapped save with a valid nearby walking pose must reopen');
        await wait(`${mapCurrent}.active && !${current}.interface.modal`);
        const walker=await state();
        assert(await page.evaluate('briccraft.briccraftKey("KeyW",true,false,false)'), 'Mapped walking must accept forward input');
        await page.evaluate('briccraft.briccraftKey("ShiftLeft",true,false,false)');
        try {
            await wait(`${current}.globalCamera[0] > 132 && ${mapCurrent}.regions.length > 1`);
        } finally {
            await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false); briccraft.briccraftKey("ShiftLeft",false,false,false)');
        }
        const travelled=await state(), afterWalk=await mapped();
        assert(travelled.worldId===walker.worldId,'Crossing geographic regions must preserve the world identity');
        assert(travelled.globalCamera[0]-walker.globalCamera[0]>40,'The walker must travel across a generated source boundary');
        assert(travelled.globalCamera[1]>2,'Pending regions must not drop the walker below solid terrain');
        assert(travelled.cachedChunks<=64,'Map movement must retain bounded rendering residency');
        const walkedSave=JSON.parse(await serialized());
        assert(JSON.stringify(walkedSave.stream.edits)===construction && walkedSave.stream.next_user_id===nextUserId,
            'Source arrival and movement must preserve exact player edits and user identities');
        assert(walkedSave.gameplay.studs===beforeWalk.gameplay.studs,'Streaming source data must not spend construction resources');

        await command('map-pause'); await wait(`!${mapCurrent}.active`);
        const frozenRegions=(await mapped()).regions.length;
        assert(await page.evaluate(`briccraft.briccraftImportWorld(${JSON.stringify(JSON.stringify(walkedSave))})`),
            'Generated source regions and construction edits must survive a save round-trip');
        await wait(`${mapCurrent}.active`);
        const restored=JSON.parse(await serialized());
        assert(JSON.stringify(restored.stream.edits)===construction,'Reopening must retain live-map constructions');
        assert(JSON.stringify((await mapped()).anchor)===JSON.stringify(initial.anchor),'Reopening must retain the exact shared geographic and elevation reference');
        await command('map-pause'); await wait(`!${mapCurrent}.active`);
        await command('paused','true');
        const frame=(await state()).rendering.frames;
        await command('resolution','quality'); await command('antialiasing','4');
        await wait(`${current}.rendering.frames > ${frame} && ${current}.worldRendering.samples === 4`);
        await page.screenshot(path.join(out,'map-roam.png'));
        const requests=await page.evaluate('globalThis.__mapRequests');
        const vectors=requests.filter(r=>r.kind==='vector');
        assert(vectors.length>0 && vectors.length<=provenance.files.length,'Repeated regions must reuse already fetched vector tiles');
        result.freeRoam={initialRegions:initial.regions.length,regions:afterWalk.regions.length,
            sourceParts:afterWalk.source_parts,walkedStuds:travelled.globalCamera[0]-walker.globalCamera[0],
            start:walker.globalCamera,end:travelled.globalCamera,worldIdentityPreserved:true,
            constructionPreserved:true,saveRoundTrip:true,sourceRequests:vectors.length,
            cachedChunks:travelled.cachedChunks,pausedRegions:frozenRegions,anchor:afterWalk.anchor,
            firstRegion:{buildings:firstReport.buildings,roads:firstReport.roads,parts:firstReport.parts}};
        return result;
    } finally {
        await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false); briccraft.briccraftKey("ShiftLeft",false,false,false)');
        await command('map-pause');
        await command('paused','false');
        await command('resolution','performance'); await command('antialiasing','1');
        await page.evaluate('globalThis.__restoreMapFetch?.()');
    }
}
