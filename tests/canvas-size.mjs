import assert from 'node:assert/strict';
import {installCanvasSizing} from '../crates/briccraft-browser/wwwroot/canvas-size.js';

class Canvas { constructor() { this.width=1280;this.height=800;this.writes=0; } }
const canvas=new Canvas(), foreign=new Canvas();
const mount={ownerDocument:{defaultView:{HTMLCanvasElement:Canvas}},contains:value=>value===canvas};
const calls=[];
class Target {
    static setSize(target,width,height) {
        calls.push([this,target,width,height]);
        target.canvas.width=width;target.canvas.height=height;target.canvas.writes+=2;
        return 'delegated';
    }
}
const original=Target.setSize,target={renderTargetType:'webgl',canvas};
const adapter=installCanvasSizing(mount,Target);
for(let frame=0;frame<120;frame++) Target.setSize(target,1280,800);
assert.equal(canvas.writes,0);
assert.equal(adapter.statistics().skipped,120);
assert.equal(Target.setSize(target,390,844),'delegated');
assert.deepEqual([canvas.width,canvas.height],[390,844]);
assert.equal(adapter.statistics().resizes,1);
assert.equal(calls[0][0],Target);
for(const other of [{renderTargetType:'webgl',canvas:foreign},{renderTargetType:'software',canvas},
    {renderTargetType:'webgl',canvas:{width:390,height:844,writes:0}}]) {
    assert.equal(Target.setSize(other,390,844),'delegated');
}
adapter.setEnabled(false);
assert.equal(Target.setSize(target,390,844),'delegated');
adapter.setEnabled(true);
const before=canvas.writes;Target.setSize(target,390,844);assert.equal(canvas.writes,before);
adapter.dispose();assert.equal(Target.setSize,original);
const second=installCanvasSizing(mount,Target),replacement=()=>{};
Target.setSize=replacement;second.dispose();assert.equal(Target.setSize,replacement);
console.log(JSON.stringify({test:'Owned canvas sizing',repeatedFrames:120,realResize:true,foreignTargets:true,diagnosticToggle:true,disposal:true}));
