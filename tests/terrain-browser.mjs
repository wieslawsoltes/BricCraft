// End-to-end source bytes -> worker -> staged stream commit -> save round-trip.
import path from 'node:path';
import {assert} from '../FerroUI/scripts/browser/harness.mjs';

export async function checkTerrainArea({page,state,command,wait,current,out,presentedSample}) {
    const previousWorldId=(await state()).worldId;
    await command('terrain-sample');
    const complete=`${current}.worldId !== ${JSON.stringify(previousWorldId)} && ${current}.worldName === 'San Francisco · Telegraph Hill' && !${current}.importBusy && !${current}.worldBusy`;
    await wait(complete);
    const result=await state();
    const serialized=await page.evaluate('briccraft.briccraftSerializeWorld()');
    const saved=JSON.parse(serialized);
    const report=JSON.parse(saved.world_info.import_report);
    assert(report.parts===12820 && report.buildings===1351 && report.roads===1069,
        'The real area must preserve every converted source feature through the worker');
    assert(report.terrain.columns===12996 && report.terrain.effective_meters_per_stud===8,
        'Terrain must cover the complete selected area at the recorded scale');
    assert(report.terrain.elevation_max_m-report.terrain.elevation_min_m>100 && report.terrain.foundation_columns>0,
        'The real sample must retain measured relief and leveled building foundations');
    assert(result.attributions.length>=2,'Map and elevation credits must reach the saved world');
    assert(result.cachedChunks<=64,'The imported area must obey the residency budget');
    assert(await page.evaluate(`briccraft.briccraftImportWorld(${JSON.stringify(serialized)})`),
        'The full terrain construction and its provenance must survive a save round-trip');
    await wait(`!${current}.worldBusy`);
    const restored=JSON.parse(await page.evaluate('briccraft.briccraftSerializeWorld()'));
    assert(restored.world_info.import_report===saved.world_info.import_report,
        'Save loading must preserve the exact terrain report');
    await page.screenshot(path.join(out,'telegraph-hill.png'));
    return {parts:report.parts,visibleParts:(await state()).parts,buildings:report.buildings,roads:report.roads,
        landcover:report.landcover,terrain:report.terrain,presentation:await presentedSample()};
}
