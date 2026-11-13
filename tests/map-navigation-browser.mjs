// Geographic navigation uses the actual native location controls and map layer.
import path from 'node:path';
import {assert} from '../FerroUI/scripts/browser/harness.mjs';

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
    const selected=[selection.latitude,selection.longitude];
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
        await page.screenshot(path.join(out,'mobile-map-navigation.png'));
        return {player,geographicPosition:true,heading:true,selectionPreserved:true,
            staleAreaStatusCleared:true,mobileLocate:true,worldPreserved:true,
            desktopRect:desktop.picker.rect,mobileRect:mobile.picker.rect,sourceRegions:initial.regions.length};
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
