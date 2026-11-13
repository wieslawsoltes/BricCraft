// Geographic navigation uses the actual native location controls and map layer.
import path from 'node:path';
import {assert} from '../FerroUI/scripts/browser/harness.mjs';
import {geographicAt} from './map-picker-browser.mjs';

const mapState='JSON.parse(briccraft.briccraftMapState())';
const pickerState=`${mapState}.picker`;
const distance=(a,b)=>Math.hypot(...a.map((value,index)=>value-b[index]));

export async function checkMapNavigation({page,state,command,wait,current,out}) {
    const mapped=async()=>JSON.parse(await page.evaluate('briccraft.briccraftMapState()'));
    const send=async(method,params)=>{
        const result=await page.send(method,params);
        assert(!result.error,`${method}: ${result.error?.message}`);
    };
    const original=await page.evaluate('({width:innerWidth,height:innerHeight,dpr:devicePixelRatio})');
    const selection={latitude:52.5172,longitude:13.379,zoom:15,radius:500,meters_per_stud:8,terrain:true};
    let selected=[selection.latitude,selection.longitude];
    try {
        await command('paused','true');
        const worldBefore=await state();
        const initial=await mapped();
        await command('map-view',JSON.stringify(selection));
        // A previous model or area result must not label a new selected place.
        await page.evaluate('briccraft.briccraftImportProgress("Previous model imported",1,false)');
        assert((await mapped()).picker.area_status==='Previous model imported','The stale-result regression needs an earlier import result');
        let before=(await state()).rendering.frames;
        await command('map-locate');
        await wait(`${pickerState}.open && ${pickerState}.rect && ${pickerState}.player && ${pickerState}.tiles > 0 && ${current}.rendering.frames > ${before}`);
        const desktop=await mapped();
        const player=desktop.picker.player, anchor=desktop.anchor;
        const radius=6378137;
        const expected=[
            anchor.latitude-(worldBefore.globalCamera[2]-anchor.origin[2])*anchor.meters_per_stud/radius*180/Math.PI,
            anchor.longitude+(worldBefore.globalCamera[0]-anchor.origin[0])*anchor.meters_per_stud/(radius*Math.cos(anchor.latitude*Math.PI/180))*180/Math.PI,
        ];
        assert(distance([player.latitude,player.longitude],expected)<1e-7,
            'Navigation must unproject the current walked position, including streamed origin changes');
        assert(Math.abs(player.bearing-90)<0.01,'The eastbound walker must show geographic east, not game-yaw north');
        assert(player.distance>100,'The location card must report mapped distance from the starting pin');
        assert(distance([desktop.picker.latitude,desktop.picker.longitude],[player.latitude,player.longitude])<1e-9,
            'Locate must center the current geographic body position');
        assert(distance(desktop.picker.selected,selected)<1e-9
            && desktop.picker.radius===500 && desktop.picker.meters_per_stud===8 && desktop.picker.terrain,
            'Locating the player must preserve the independent build selection and generation options');
        assert(desktop.picker.area_status==='','Opening through Locate must clear an unrelated import result');
        assert(distance((await state()).globalCamera,worldBefore.globalCamera)<0.0001,
            'Locating on the map must not move the 3D walker');

        // Retain the same wide layout while changing its dimensions. Replacing
        // an on-mount NodeRef during reconciliation used to leave the viewport
        // handlers pointing at an empty reference, while ordinary buttons worked.
        before=(await state()).rendering.frames;
        await page.resize(original.width+160,original.height+40);
        await wait(`${current}.interface.width === ${original.width+160} && ${current}.rendering.frames > ${before}`);
        let resized=await mapped();
        let [x,y,width,height]=resized.picker.rect;
        const local=[Math.floor(width*0.38),Math.floor(height*0.42)];
        const expectedSelection=geographicAt(resized.picker,local);
        before=(await state()).rendering.frames;
        await page.click(x+local[0],y+local[1]);
        await wait(`${current}.rendering.frames > ${before}`);
        resized=await mapped();
        assert(distance(resized.picker.selected,expectedSelection)<0.00002,
            'A retained desktop map must still select the clicked location after a same-branch resize');
        selected=resized.picker.selected;
        const previousCenter=[resized.picker.latitude,resized.picker.longitude];
        before=(await state()).rendering.frames;
        await page.mouseDown(x+width*0.5,y+height*0.5);
        await page.mouseMove(x+width*0.5+35,y+height*0.5+15);
        await page.mouseMove(x+width*0.5+65,y+height*0.5+25);
        await page.mouseUp(x+width*0.5+65,y+height*0.5+25);
        await wait(`${current}.rendering.frames > ${before}`);
        resized=await mapped();
        assert(distance([resized.picker.latitude,resized.picker.longitude],previousCenter)>0.00001,
            'A retained desktop map must still pan after a same-branch resize');
        assert(distance(resized.picker.selected,selected)<1e-9,'Resized map dragging must preserve the build pin');
        before=(await state()).rendering.frames;
        await command('map-locate');
        await wait(`${current}.rendering.frames > ${before}`);
        const relocated=await mapped();
        assert(distance([relocated.picker.latitude,relocated.picker.longitude],[player.latitude,player.longitude])<1e-9
            && distance(relocated.picker.selected,selected)<1e-9,
            'Locate after a real resized-map gesture must preserve the new selection');
        await page.screenshot(path.join(out,'map-navigation.png'));

        await command('places','false');
        await send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:5});
        before=(await state()).rendering.frames;
        await send('Emulation.setDeviceMetricsOverride',{width:390,height:844,deviceScaleFactor:1,mobile:true});
        await wait(`${current}.interface.width === 390 && !${current}.interface.modal && ${current}.rendering.frames > ${before}`);
        before=(await state()).rendering.frames;
        await page.evaluate('briccraft.briccraftMessage("Construction ready. Your map remains available.")');
        await wait(`${current}.rendering.frames > ${before}`);
        // The ordinary activity message must remain visible beside the compass.
        // The screenshot records the actual native text and 48px touch target.
        await page.screenshot(path.join(out,'mobile-map-navigation-hud.png'));
        before=(await state()).rendering.frames;
        await send('Input.dispatchTouchEvent',{
            type:'touchStart',touchPoints:[{id:1,x:334,y:150,radiusX:5,radiusY:5,force:1}],
        });
        await send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
        await wait(`${pickerState}.open && ${pickerState}.rect && ${current}.rendering.frames > ${before}`);
        const mobile=await mapped();
        assert(mobile.picker.rect[0]>=0 && mobile.picker.rect[0]+mobile.picker.rect[2]<=390,
            'The player map must fit the phone viewport');
        assert(distance([mobile.picker.latitude,mobile.picker.longitude],[player.latitude,player.longitude])<1e-9,
            'Tapping the native compass must open Places centered on the player');
        assert(distance(mobile.picker.selected,selected)<1e-9,'Phone navigation must preserve the build pin');
        assert((await state()).worldId===worldBefore.worldId && distance((await state()).globalCamera,worldBefore.globalCamera)<0.0001,
            'Phone navigation must preserve the walking world and body position');

        // Phone browser chrome can change available height without changing the
        // compact layout branch. Verify actual touch picking on that retained map.
        before=(await state()).rendering.frames;
        await send('Emulation.setDeviceMetricsOverride',{width:390,height:780,deviceScaleFactor:1,mobile:true});
        await wait(`${current}.interface.height === 780 && ${current}.rendering.frames > ${before}`);
        const compact=await mapped();
        [x,y,width,height]=compact.picker.rect;
        const touchLocal=[Math.floor(width*0.7),Math.floor(height*0.5)];
        const touchSelection=geographicAt(compact.picker,touchLocal);
        before=(await state()).rendering.frames;
        await send('Input.dispatchTouchEvent',{
            type:'touchStart',touchPoints:[{id:2,x:x+touchLocal[0],y:y+touchLocal[1],radiusX:5,radiusY:5,force:1}],
        });
        await send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
        await wait(`${current}.rendering.frames > ${before}`);
        const resizedMobile=await mapped();
        assert(distance(resizedMobile.picker.selected,touchSelection)<0.00002,
            'A retained phone map must still select the touched location after a height-only resize');
        assert(distance([resizedMobile.picker.player.latitude,resizedMobile.picker.player.longitude],[player.latitude,player.longitude])<1e-9
            && (await state()).worldId===worldBefore.worldId,
            'Touch selection on the resized map must preserve the player and walking world');
        await page.screenshot(path.join(out,'mobile-map-navigation.png'));
        return {player,geographicPosition:true,heading:true,selectionPreserved:true,
            staleAreaStatusCleared:true,mobileLocate:true,worldPreserved:true,
            wideResizePicking:true,wideResizePanning:true,compactResizePicking:true,
            desktopRect:relocated.picker.rect,mobileRect:resizedMobile.picker.rect,sourceRegions:initial.regions.length};
    } finally {
        await send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]}).catch(()=>{});
        await send('Emulation.setTouchEmulationEnabled',{enabled:false}).catch(()=>{});
        await send('Emulation.setDeviceMetricsOverride',{
            width:original.width,height:original.height,deviceScaleFactor:original.dpr,mobile:false,
        }).catch(()=>{});
        await command('places','false');
        await command('paused','false');
        await wait(`!${current}.interface.modal && ${current}.interface.width === ${original.width}`);
    }
}
