// Exercise saved construction recipes through the production Wasm/UI command boundary.
// The original terrain save remains an exact string; this journey never parses/rebuilds it.
import path from 'node:path';
import {assert} from '../FerroUI/scripts/browser/harness.mjs';

export async function checkDesigns({page,state,command,wait,current,out}) {
    const initial=await state();
    assert(!initial.vehicle,'The isolated design journey starts with the original vehicle parked');
    const original=await page.evaluate('briccraft.briccraftSerializeWorld()');
    assert(typeof original==='string' && original.length>0,'The original world must be safely serializable');
    const viewport=await page.evaluate('({width:innerWidth,height:innerHeight,dpr:devicePixelRatio})');
    const result={};
    const recipeEconomy=value=>JSON.stringify({
        mode:value.gameplay.mode,started:value.gameplay.started,
        studs:value.gameplay.studs,objectives:value.gameplay.objectives,
    });
    const unchanged=(before,after,label)=>{
        assert(after.worldId===before.worldId && after.parts===before.parts,label+': recipes must not edit or replace world geometry');
        assert(recipeEconomy(after)===recipeEconomy(before),label+': recipes must not spend studs or grant objective progress/rewards');
    };
    const newFlat=async()=>{
        const previous=(await state()).worldId;
        await command('create-world','flat');
        await wait(current+".preset==='flat' && "+current+".worldId!=="+JSON.stringify(previous)+" && !"+current+".worldBusy");
        const created=await state();
        assert(created.designs.items.length===0,'A fresh world must start with its own empty design library');
        return created;
    };
    const resize=async(width,height)=>{
        const before=await state();
        if(Math.abs(before.interface.width-width)<2 && Math.abs(before.interface.height-height)<2) return;
        await page.resize(width,height);
        await wait('Math.abs('+current+'.interface.width-'+width+')<2 && Math.abs('+current+'.interface.height-'+height+')<2 && '+current+'.rendering.frames>'+before.rendering.frames);
    };
    const commandAndPaint=async(name,value='')=>{
        const frame=(await state()).rendering.frames;
        await command(name,value);
        await wait(current+'.rendering.frames>'+frame);
    };
    const assertModalInputBlocked=async()=>{
        const value=await state();
        assert(value.interface.designsOpen && value.interface.modal,'My designs must be an actual native modal panel');
        try {
            assert(!await page.evaluate('briccraft.briccraftKey("KeyW",true,false,false)'),
                'The open native design library must block world movement input');
        } finally {
            await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false)');
        }
    };
    const objective=(value,id)=>value.gameplay.objectives.find(goal=>goal.id===id);
    let failure;
    try {
        await command('designs','false');
        await resize(1280,800);
        const sourceWorld=await newFlat();
        assert(sourceWorld.gameplay.mode==='creative','The source construction must start in unlimited Creative mode');
        const sourceBaseline=sourceWorld.parts;
        await command('blueprint','rover');
        const built=await state();
        assert(built.parts-sourceBaseline===23,'The source rover must contain its actual 23 parts');
        assert(built.gameplay.studs===sourceWorld.gameplay.studs &&
            JSON.stringify(built.gameplay.objectives)===JSON.stringify(sourceWorld.gameplay.objectives),
            'Creative source construction must not consume the Expedition wallet or grant goals');
        assert(built.designs.capture.ready && built.designs.capture.parts===23,
            'Capture must identify the complete separate rover rather than terrain or a partial selection');

        await commandAndPaint('design-capture','Roadster prototype');
        let captured=await state();
        assert(captured.designs.items.length===1,'Capture must create exactly one reusable recipe');
        const sourceId=captured.designs.items[0].id;
        assert(captured.designs.items[0].parts===23 && captured.designs.items[0].studs===377,
            'The recipe must quote its actual 23 constituent parts at 377 studs');
        assert(captured.designs.items[0].source.source_world_id===sourceWorld.worldId,
            'Capture must retain the actual source-world identity');
        unchanged(built,captured,'Capture');
        await commandAndPaint('design-rename',JSON.stringify({id:sourceId,name:'Atlas rover'}));
        const renamed=await state();
        assert(renamed.designs.items[0].id===sourceId && renamed.designs.items[0].name==='Atlas rover',
            'Rename must preserve the recipe identity');
        unchanged(captured,renamed,'Rename');
        await assertModalInputBlocked();
        assert(!renamed.interface.compact,'The desktop design library must use the desktop native layout');
        await page.screenshot(path.join(out,'design-library.png'));

        const portable=await page.evaluate('briccraft.briccraftSerializeDesign('+sourceId+')');
        const document=JSON.parse(portable);
        assert(document.format==='briccraft.design' && document.version===1,'Export must use the versioned portable design format');
        assert(document.design.parts.length===23 && document.design.name==='Atlas rover',
            'Portable export must retain the complete named recipe');
        assert(document.design.metadata.source_world_id===sourceWorld.worldId,'Portable export must preserve source metadata');

        await resize(390,844);
        await wait(current+'.interface.compact && '+current+'.interface.designsOpen && '+current+'.interface.modal');
        const mobileBefore=await state();
        assert(mobileBefore.interface.panelWidth+24<=mobileBefore.interface.width,
            'The mobile design library must fit within its logical viewport');
        await commandAndPaint('design-capture','Pocket rover');
        const mobile=await state();
        assert(mobile.designs.items.length===2 && mobile.designs.items.some(item=>item.id!==sourceId && item.name==='Pocket rover'),
            'Capture must remain functional in the compact native design library');
        unchanged(mobileBefore,mobile,'Mobile capture');
        await assertModalInputBlocked();
        await page.screenshot(path.join(out,'mobile-designs.png'));
        const mobileId=mobile.designs.items.find(item=>item.id!==sourceId).id;
        await command('design-delete',String(mobileId));
        const deleted=await state();
        assert(deleted.designs.items.length===1 && deleted.designs.items[0].id===sourceId,
            'Deleting a recipe must preserve the other saved design');
        unchanged(mobile,deleted,'Recipe deletion');

        await command('designs','false');
        await resize(1280,800);
        const destination=await newFlat();
        assert(destination.worldId!==sourceWorld.worldId,'Portable reuse must exercise a genuinely different world');
        await command('expedition','on');
        const expedition=await state();
        assert(expedition.gameplay.mode==='expedition' && expedition.gameplay.studs===1200,
            'The destination Expedition must start with its actual 1200-stud budget');
        assert(await page.evaluate('briccraft.briccraftImportDesign('+JSON.stringify(portable)+')'),
            'The portable recipe must import through the production Wasm bridge');
        const imported=await state();
        assert(imported.designs.items.length===1,'Portable import must add one recipe');
        const importedId=imported.designs.items[0].id;
        assert(imported.designs.items[0].name==='Atlas rover' && imported.designs.items[0].parts===23,
            'Cross-world import must retain the recipe name and complete geometry');
        assert(imported.designs.items[0].source.source_world_id===sourceWorld.worldId,
            'Import must keep the source identity rather than substituting the destination world');
        unchanged(expedition,imported,'Portable import');
        assert(await page.evaluate('briccraft.briccraftImportDesign('+JSON.stringify(portable)+')'),
            'A second portable import must receive a distinct local identity');
        const duplicated=await state();
        assert(duplicated.designs.items.length===2 && new Set(duplicated.designs.items.map(item=>item.id)).size===2,
            'Imported source IDs must never overwrite existing destination recipes');
        assert(duplicated.designs.items.some(item=>item.name==='Atlas rover (2)'),
            'A name collision must receive the documented deterministic suffix');
        unchanged(imported,duplicated,'Duplicate portable import');

        // This deliberately corrupted document came from a fresh small-coordinate fixture.
        // The untouched original terrain save above is never parsed or reserialized.
        const malformed=JSON.parse(portable);
        malformed.design.parts[0].color=255;
        const libraryBeforeInvalid=JSON.stringify(duplicated.designs.items);
        const invalidCalls=[
            'briccraft.briccraftImportDesign('+JSON.stringify(JSON.stringify(malformed))+')',
            'briccraft.briccraftImportDesign('+JSON.stringify(portable.replace('briccraft.design','unsupported.design'))+')',
            'briccraft.briccraftImportDesign("not JSON")',
            'briccraft.briccraftImportDesign(" ".repeat(256*1024+1))',
        ];
        for(const expression of invalidCalls) {
            assert(!await page.evaluate(expression),'Invalid or oversized portable data must be rejected');
            const after=await state();
            assert(JSON.stringify(after.designs.items)===libraryBeforeInvalid,'A rejected import must leave the entire recipe library intact');
            unchanged(duplicated,after,'Rejected portable import');
        }

        await command('designs','false');
        await wait('!'+current+'.interface.modal');
        const quote=(await state()).designs.items.find(item=>item.id===importedId);
        assert(quote.studs===377 && quote.charged && quote.affordable,
            'The imported recipe must use the destination Expedition quote');
        await command('design-place',String(importedId));
        const placed=await state();
        assert(placed.parts===destination.parts+23,'Placing a portable design must create all 23 real parts');
        assert(placed.gameplay.studs===823,'Placement must debit exactly 377 studs from the 1200-stud starting wallet');
        assert(objective(placed,'build').progress===23 && !objective(placed,'build').completed &&
            objective(placed,'home').progress===0,
            'A copy counts actual placed parts without inventing a structure or recurring reward');
        for(let cycle=0;cycle<2;cycle++) {
            await command('undo');
            const undone=await state();
            assert(undone.parts===destination.parts && undone.gameplay.studs===1200,
                'Undo must remove the entire copy and refund exactly its purchase');
            await command('redo');
            const redone=await state();
            assert(redone.parts===placed.parts && redone.gameplay.studs===placed.gameplay.studs,
                'Redo must restore the whole recipe and charge exactly once');
            assert(JSON.stringify(redone.gameplay.objectives)===JSON.stringify(placed.gameplay.objectives),
                'Construction history must not replay recipe placement progress or rewards');
        }
        await command('designs','false');
        assert((await state()).gameplay.workshop.ready,'The copied rover must pass the real workshop assessment');
        await command('drive','auto');
        const activated=await state();
        assert(activated.vehicle && objective(activated,'workshop').completed,
            'An imported design copy must become a usable vehicle');
        assert(activated.gameplay.studs===1083,'The first successful activation must grant the existing workshop reward once');
        assert(await page.evaluate('briccraft.briccraftKey("KeyW",true,false,false)'),'The copied vehicle must accept actual throttle input');
        try {
            await wait(current+'.vehicle && Math.hypot('+current+'.vehicle[0]-'+activated.vehicle[0]+','+
                current+'.vehicle[2]-'+activated.vehicle[2]+')>0.5 && '+
                current+".gameplay.objectives.find(goal=>goal.id==='drive').progress>1");
        } finally {
            await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false)');
        }
        const driven=await state();
        const drivenDistance=Math.hypot(driven.vehicle[0]-activated.vehicle[0],driven.vehicle[2]-activated.vehicle[2]);
        await command('park');
        assert(!(await state()).vehicle,'A driven construction copied from a portable recipe must remain parkable');

        const beforeSave=await state();
        const save=await page.evaluate('briccraft.briccraftSerializeWorld()');
        assert(typeof save==='string' && save.length>0,'The funded design world must produce a valid complete save');
        const persistedLibrary=JSON.stringify(beforeSave.designs.items);
        const persistedGameplay=recipeEconomy(beforeSave);
        await command('design-rename',JSON.stringify({id:importedId,name:'Changed after save'}));
        assert((await state()).designs.items.find(item=>item.id===importedId).name==='Changed after save',
            'The save round-trip must restore a genuinely changed live library');
        assert(await page.evaluate('briccraft.briccraftImportWorld('+JSON.stringify(save)+')'),
            'The complete world save must load through the validated production bridge');
        await wait('!'+current+'.worldBusy && '+current+'.worldId==='+JSON.stringify(destination.worldId));
        const restored=await state();
        assert(JSON.stringify(restored.designs.items)===persistedLibrary,
            'Save/load must preserve recipe names, identities, quotes, colors and source metadata');
        assert(restored.parts===beforeSave.parts && recipeEconomy(restored)===persistedGameplay,
            'Save/load must preserve placed constructions, the exact wallet and objective evidence');
        const restoredPortable=await page.evaluate('briccraft.briccraftSerializeDesign('+importedId+')');
        assert(JSON.stringify(JSON.parse(restoredPortable).design.parts)===JSON.stringify(document.design.parts),
            'Part colors and relative orientations must survive cross-world placement and persistence');

        result.sourceWorldId=sourceWorld.worldId;result.destinationWorldId=destination.worldId;
        result.sourceDesignId=sourceId;result.importedDesignIds=restored.designs.items.map(item=>item.id);
        result.partsPerDesign=23;result.designCost=377;result.balanceAfterPlacement=placed.gameplay.studs;
        result.balanceAfterDriving=beforeSave.gameplay.studs;result.drivenDistance=drivenDistance;
        result.undoRedoCycles=2;result.malformedRejected=invalidCalls.length;
        result.desktop={width:renamed.interface.width,height:renamed.interface.height,designs:renamed.designs.items.length};
        result.mobile={width:mobile.interface.width,height:mobile.interface.height,designs:mobile.designs.items.length};
        result.recipeActionsPreserveResources=true;result.crossWorldPortability=true;result.saveRoundTrip=true;
        return result;
    } catch(error) {
        failure=error;
        console.error('Design journey failure state:',await state().catch(()=>null));
        throw error;
    } finally {
        try {
            try {
                await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false)');
                await command('designs','false');
                if((await state()).vehicle) await command('park');
                assert(await page.evaluate('briccraft.briccraftImportWorld('+JSON.stringify(original)+')'),
                    'The exact original terrain world must be restored after the isolated design journey');
                await wait('!'+current+'.worldBusy && '+current+'.worldId==='+JSON.stringify(initial.worldId));
            } finally {
                // Restore presentation even when a failing world replacement needs its own diagnostic.
                await page.resize(viewport.width,viewport.height);
                await wait('Math.abs('+current+'.interface.width-'+viewport.width+')<2 && Math.abs('+current+'.interface.height-'+viewport.height+')<2');
            }
        } catch(error) {
            if(failure) throw new AggregateError([failure,error],'The design journey and original-world restoration both failed');
            throw error;
        }
    }
}
