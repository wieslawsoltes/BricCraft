// Exercise the same paid construction, vehicle and persistence commands as the UI.
import path from 'node:path';
import {assert} from '../FerroUI/scripts/browser/harness.mjs';

export async function checkGameplay({page,state,command,wait,current,out}) {
    const original=await page.evaluate('briccraft.briccraftSerializeWorld()');
    const rewards=gameplay=>gameplay.objectives.filter(goal=>goal.completed).reduce((sum,goal)=>sum+goal.reward,0);
    const objective=(gameplay,id)=>gameplay.objectives.find(goal=>goal.id===id);
    const result={};
    try {
        await command('create-world','flat');
        await wait(`${current}.preset==='flat' && !${current}.worldBusy`);
        assert((await state()).gameplay.mode==='creative','New worlds retain unlimited Creative mode');
        await command('expedition','on');
        const started=await state(),baseline=started.parts;
        assert(started.gameplay.mode==='expedition' && started.gameplay.started,'Expedition must be an actual game mode');
        assert(started.gameplay.studs===1200,'A new expedition receives its documented starting supply once');
        await command('camera');await command('camera');
        assert((await state()).mode==='WALK','Walk controls must select the collision-aware character');
        assert(await page.evaluate('briccraft.briccraftKey("KeyW",true,false,false)'), 'Walk input must be accepted');
        try {
            await wait(`${current}.gameplay.objectives.find(goal=>goal.id==='walk').progress > 2`);
        } finally { await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false)'); }
        result.walked=objective((await state()).gameplay,'walk').progress;
        await command('home');
        const rover=started.gameplay.blueprints.find(quote=>quote.name==='rover');
        assert(rover?.affordable && rover.charged && rover.studs>0,'The starter supply must afford a real rover');
        await command('blueprint','rover');
        const built=await state();
        assert(built.parts>baseline,'The funded blueprint must place real geometry');
        assert(built.gameplay.studs===started.gameplay.studs-rover.studs+rewards(built.gameplay)-rewards(started.gameplay),
            'Construction must charge its authoritative quote and award only completed objectives');
        for(let cycle=0;cycle<2;cycle++) {
            await command('undo');
            const undone=await state();
            assert(undone.parts===baseline && undone.gameplay.studs===built.gameplay.studs+rover.studs,
                'Undo must refund exactly the construction value');
            await command('redo');
            const redone=await state();
            assert(redone.parts===built.parts && redone.gameplay.studs===built.gameplay.studs,
                'Redo must charge once without re-awarding objectives');
        }
        assert((await state()).gameplay.workshop.ready,'The real workshop assessment must recognize the funded rover');
        await command('drive','auto');
        const active=await state();
        assert(active.vehicle && objective(active.gameplay,'workshop').completed,
            'Successful mechanical activation must complete the vehicle objective');
        assert(await page.evaluate('briccraft.briccraftKey("KeyW",true,false,false)'), 'Vehicle input must be accepted');
        try {
            await wait(`${current}.gameplay.objectives.find(goal=>goal.id==='drive').progress > 1`);
        } finally { await page.evaluate('briccraft.briccraftKey("KeyW",false,false,false)'); }
        await command('park');
        assert(!(await state()).vehicle,'A vehicle must remain parkable after expedition travel');
        const progressed=await state();
        assert(objective(progressed.gameplay,'drive').progress>0,'Drive progress must come from actual simulated motion');
        const source=await page.evaluate('briccraft.briccraftSerializeWorld()');
        assert(await page.evaluate(`briccraft.briccraftImportWorld(${JSON.stringify(source)})`),
            'Expedition construction and progress must survive validated persistence');
        await wait(`!${current}.worldBusy`);
        const restored=await state();
        assert(restored.gameplay.studs===progressed.gameplay.studs &&
            JSON.stringify(restored.gameplay.objectives)===JSON.stringify(progressed.gameplay.objectives),
            'Loading must preserve the exact wallet and objective evidence');
        await command('expedition','off');await command('expedition','on');
        assert((await state()).gameplay.studs===restored.gameplay.studs,
            'Mode switching must never grant another starting supply');
        await command('gameplay');
        await wait(`${current}.interface.gameplayOpen && ${current}.interface.modal`);
        await page.screenshot(path.join(out,'expedition.png'));
        await page.resize(390,844);
        await wait(`${current}.interface.compact && ${current}.interface.width<400`);
        await page.screenshot(path.join(out,'mobile-expedition.png'));
        result.startingStuds=started.gameplay.studs;result.roverCost=rover.studs;
        result.partsBuilt=built.parts-baseline;result.gameplay=(await state()).gameplay;
        result.undoRedoCycles=2;result.saveRoundTrip=true;result.starterGrantedOnce=true;
        return result;
    } finally {
        await command('gameplay','false');
        await page.resize(1280,800);
        if((await state()).vehicle) await command('park');
        const restored=await page.evaluate(`briccraft.briccraftImportWorld(${JSON.stringify(original)})`);
        assert(restored,'The terrain world must be restored after the isolated expedition journey');
        await wait(`!${current}.worldBusy`);
    }
}
