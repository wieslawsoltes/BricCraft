// Exercise the actual native map viewport. The caller installs bounded tile fixtures.
import path from 'node:path';
import {assert} from '../FerroUI/scripts/browser/harness.mjs';

const mapState='JSON.parse(briccraft.briccraftMapState())';
const pickerState=`${mapState}.picker`;
const distance=(a,b)=>Math.hypot(...a.map((value,index)=>value-b[index]));
const center=picker=>[picker.latitude,picker.longitude];

// Independent inverse-Mercator reference for checking an actual hit-tested pixel.
export function geographicAt(picker,local) {
    const world=256*2**picker.zoom;
    const latitude=picker.latitude*Math.PI/180;
    const x=(picker.longitude+180)/360*world+local[0]-picker.size[0]/2;
    const y=(1-Math.asinh(Math.tan(latitude))/Math.PI)/2*world+local[1]-picker.size[1]/2;
    return [Math.atan(Math.sinh(Math.PI*(1-2*y/world)))*180/Math.PI,
        ((x/world*360)%360+360)%360-180];
}

export async function checkMapPicker({page,state,command,wait,current,out}) {
    const picker=async()=>JSON.parse(await page.evaluate('briccraft.briccraftMapState()')).picker;
    const send=async(method,params)=>{
        const result=await page.send(method,params);
        assert(!result.error,`${method}: ${result.error?.message}`);
    };
    const touch=(type,points=[])=>send('Input.dispatchTouchEvent',{
        type,touchPoints:points.map(([id,x,y])=>({id,x,y,radiusX:5,radiusY:5,force:1})),
    });
    const paint=before=>wait(`${current}.rendering.frames > ${before} && !${current}.worldBusy`);
    const construction=()=>page.evaluate(`(() => {
        const saved=JSON.parse(briccraft.briccraftSerializeWorld());
        return JSON.stringify(saved.stream ? {
            edits:saved.stream.edits,nextUserId:saved.stream.next_user_id,
            seed:saved.stream.seed,preset:saved.stream.preset,
        } : saved.world);
    })()`);
    const berlin={latitude:52.516275,longitude:13.377704,zoom:16,terrain:false};
    const result={};
    const original=await page.evaluate('({width:innerWidth,height:innerHeight,dpr:devicePixelRatio})');
    const assertSameWorld=async(base,unchanged,label)=>{
        const observed=await state();
        assert(observed.worldId===base.worldId,`${label}: choosing places must not replace the current world`);
        assert(distance(observed.globalCamera,base.globalCamera)<0.0001,
            `${label}: map input must not move the 3D camera`);
        assert(await construction()===unchanged,`${label}: map input must not place/remove world bricks`);
    };
    try {
        if((await state()).interface.modal) await page.press('Escape');
        await page.resize(1280,800);
        await command('map-view',JSON.stringify(berlin));
        const openingFrame=(await state()).rendering.frames;
        await command('places');
        await wait(`${pickerState}.open && ${pickerState}.rect && ${pickerState}.tiles > 0`);
        await paint(openingFrame);
        const originalWorld=await state(), unchanged=await construction();
        let map=await picker();
        assert(map.rect[2]>400 && map.rect[3]>200,'Desktop map must provide a useful visible viewport');
        assert(Math.abs(map.rect[2]-map.size[0])<1 && Math.abs(map.rect[3]-map.size[1])<1,
            'Native map dimensions must match the picking and tile-request geometry');
        const [x,y,width,height]=map.rect;
        const local=[Math.floor(width*0.38),Math.floor(height*0.42)];
        const expected=geographicAt(map,local);
        let before=(await state()).rendering.frames;
        await page.click(x+local[0],y+local[1]);
        await paint(before);
        map=await picker();
        assert(distance(map.selected,expected)<0.00002,
            `A map click must select its geographic coordinates: ${JSON.stringify({actual:map.selected,expected})}`);
        const selection=map.selected;
        const previousCenter=center(map);
        before=(await state()).rendering.frames;
        await page.mouseDown(x+width*0.5,y+height*0.5);
        await page.mouseMove(x+width*0.5+35,y+height*0.5+15);
        await page.mouseMove(x+width*0.5+65,y+height*0.5+25);
        await page.mouseUp(x+width*0.5+65,y+height*0.5+25);
        await paint(before);
        map=await picker();
        assert(distance(center(map),previousCenter)>0.00001,'Dragging must pan the geographic map');
        assert(distance(map.selected,selection)<1e-10,'Dragging must preserve the selected build location');

        // High-resolution trackpads emit small wheel deltas. Accumulate them
        // before changing a whole tile zoom level; keep the pointed place anchored.
        const wheelLocal=[Math.floor(width*0.4),Math.floor(height*0.35)];
        const wheelPoint=[x+wheelLocal[0],y+wheelLocal[1]];
        const wheelLocation=geographicAt(map,wheelLocal), wheelZoom=map.zoom;
        for(let i=0;i<5;i++) await page.wheel(...wheelPoint,0,-2);
        assert((await picker()).zoom===wheelZoom,'Small trackpad deltas must not jump one zoom level per event');
        before=(await state()).rendering.frames;
        await page.wheel(...wheelPoint,0,-40);
        await wait(`${pickerState}.zoom === ${wheelZoom+1}`);
        await paint(before);
        map=await picker();
        assert(distance(geographicAt(map,wheelLocal),wheelLocation)<0.00002,
            'Map wheel zoom must keep the geographic location under the pointer');
        assert(distance(map.selected,selection)<1e-10,'Zoom must preserve the selected build location');

        // The bridge leaves map keys to the focused native control while modal.
        const keyCenter=center(map);
        before=(await state()).rendering.frames;
        await page.press('ArrowRight');
        await paint(before);
        map=await picker();
        assert(map.longitude>keyCenter[1],'The focused map must pan east with ArrowRight');
        const keyZoom=map.zoom;
        before=(await state()).rendering.frames;
        await page.press('PageDown');
        await wait(`${pickerState}.zoom === ${keyZoom-1}`);
        await paint(before);
        before=(await state()).rendering.frames;
        await page.press('Enter');
        await paint(before);
        map=await picker();
        assert(distance(map.selected,center(map))<1e-10,'Enter must select the map center');
        const safariZoom=map.zoom, safariLocation=geographicAt(map,wheelLocal);
        before=(await state()).rendering.frames;
        assert(await page.evaluate(`briccraft.briccraftGesture(${wheelPoint[0]},${wheelPoint[1]},${-Math.log(1.5)/0.0015})`),
            'The Safari gesture adapter must route a map hit to the native map viewport');
        await wait(`${pickerState}.zoom === ${safariZoom+1}`);
        await paint(before);
        map=await picker();
        assert(distance(geographicAt(map,wheelLocal),safariLocation)<0.00002,
            'Safari map zoom must preserve the geographic pointer anchor');
        await assertSameWorld(originalWorld,unchanged,'Desktop picker');
        await page.screenshot(path.join(out,'map-picker.png'));
        result.desktop={rect:map.rect,zoom:map.zoom,selected:map.selected,tiles:map.tiles,
            coordinatePicking:true,panPreservesSelection:true,zoomAnchor:true,keyboard:true,safariGesture:true,constructionUnchanged:true};

        before=(await state()).rendering.frames;
        await send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:5});
        await send('Emulation.setDeviceMetricsOverride',{
            width:390,height:844,deviceScaleFactor:1,mobile:true,
        });
        await wait(`${current}.interface.width === 390 && ${pickerState}.rect && ${pickerState}.rect[2] < 390`);
        await paint(before);
        map=await picker();
        const [mx,my,mw,mh]=map.rect;
        assert(mx>=0 && mx+mw<=390 && my>=0 && my+mh<=844,'The phone map must fit the visible screen');
        const mobilePoint=[Math.floor(mx+mw*0.48),Math.floor(my+mh*0.45)];
        const mobileExpected=geographicAt(map,[mobilePoint[0]-mx,mobilePoint[1]-my]);
        before=(await state()).rendering.frames;
        await touch('touchStart',[[1,...mobilePoint]]);
        await touch('touchEnd');
        await paint(before);
        map=await picker();
        assert(distance(map.selected,mobileExpected)<0.00002,'A phone tap must choose the actual geographic point');
        const mobileSelection=map.selected;
        const mobileZoom=map.zoom;
        const [tx,ty]=mobilePoint;
        before=(await state()).rendering.frames;
        await touch('touchStart',[[2,tx-30,ty],[3,tx+30,ty]]);
        await touch('touchMove',[[2,tx-55,ty],[3,tx+55,ty]]);
        await paint(before);
        const pinched=await picker();
        assert(pinched.zoom>mobileZoom,'Two-finger map pinch must zoom the native map');
        assert(distance(pinched.selected,mobileSelection)<1e-10,'Pinch must not choose a new place');
        await touch('touchMove',[[3,tx+55,ty]]);
        const lifted=await picker();
        assert(distance(center(lifted),center(pinched))<1e-10 && lifted.zoom===pinched.zoom,
            'Lifting one pinch contact must not jump the map');
        before=(await state()).rendering.frames;
        await touch('touchMove',[[3,tx+70,ty+10]]);
        await touch('touchEnd');
        await paint(before);
        map=await picker();
        assert(distance(center(map),center(lifted))>0.000001,'The remaining pinch contact must continue panning');
        assert(distance(map.selected,mobileSelection)<1e-10,'Contact release must not become a map selection');
        // Upward movement would scroll the surrounding phone panel if its
        // ancestor gesture recognizer were allowed to steal the map contact.
        const beforeUpward=map;
        before=(await state()).rendering.frames;
        await touch('touchStart',[[4,tx,ty]]);
        await touch('touchMove',[[4,tx,ty-45]]);
        await touch('touchMove',[[4,tx,ty-80]]);
        await touch('touchEnd');
        await paint(before);
        map=await picker();
        assert(distance(map.rect,beforeUpward.rect)<0.01,
            'Dragging upward on the map must not scroll the surrounding phone panel');
        assert(distance(center(map),center(beforeUpward))>0.000001,
            'Upward map dragging must pan its geographic center');
        assert(distance(map.selected,mobileSelection)<1e-10,'Upward map dragging must preserve the chosen location');
        await assertSameWorld(originalWorld,unchanged,'Phone picker');
        await page.screenshot(path.join(out,'mobile-map-picker.png'));
        result.mobile={rect:map.rect,zoom:map.zoom,selected:map.selected,tiles:map.tiles,
            touchPicking:true,pinch:true,contactLift:true,ancestorScrollIsolation:true,constructionUnchanged:true};
        return result;
    } finally {
        await touch('touchCancel').catch(()=>{});
        await send('Emulation.setTouchEmulationEnabled',{enabled:false}).catch(()=>{});
        await send('Emulation.setDeviceMetricsOverride',{
            width:1280,height:800,deviceScaleFactor:original.dpr,mobile:false,
        }).catch(()=>{});
        await command('map-view',JSON.stringify(berlin));
        if((await state()).interface.modal) await page.press('Escape');
        await wait(`!${current}.interface.modal && ${current}.interface.width === 1280`);
    }
}
