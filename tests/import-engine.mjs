// Exercise the shipped wasm-bindgen ABI, not a second JavaScript implementation.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const site=path.resolve(process.argv[2]);
const module=await import(pathToFileURL(path.join(site,'import-engine/briccraft_import_worker.js')));
module.initSync({module:fs.readFileSync(path.join(site,'import-engine/briccraft_import_worker_bg.wasm'))});
const options=JSON.parse(module.mapOptions(52.516275,13.377704,160,4));
options.origin=[48,18,48];
const updates=[];
const plan=JSON.parse(module.processImport('osm',fs.readFileSync('crates/briccraft-import/tests/fixtures/brandenburg-osm.json'),JSON.stringify(options),json => {updates.push(JSON.parse(json));return true;}));
assert(plan.report.buildings>20 && plan.report.roads>10);
assert(plan.report.height_evidence.some(h=>h.osm_id===518071791 && h.height_m===26 && !h.inferred));
assert(plan.report.bounds.max[1]<=72);
assert(plan.attribution.some(a=>a.license==='ODbL-1.0'));
assert(updates.length>2);
const model=JSON.parse(module.processImport('glb',fs.readFileSync('crates/briccraft-import/tests/fixtures/BoxVertexColors.glb'),module.defaultOptions('glb'),()=>true));
assert(model.placements.length>0 && model.report.source_triangles===12);
assert(new Set(model.placements.map(p=>p.color)).size>1);
assert.throws(()=>module.processImport('obj',new TextEncoder().encode('v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3'),module.defaultOptions('obj'),()=>false));
// The shipped decoder must preserve raw elevation bytes and complete-area geometry.
const terrainManifest=JSON.parse(fs.readFileSync(path.join(site,'assets/terrain/telegraph-hill.json')));
const elevation=JSON.parse(module.elevationGrid(JSON.stringify(terrainManifest.metadata),
    fs.readFileSync(path.join(site,'assets/terrain/telegraph-hill.dem'))));
assert.equal(elevation.values.length,118*118);
assert(elevation.values.every(value=>Number.isFinite(value)));
assert.equal(elevation.vertical_datum,'Unknown');
const hillsOptions=JSON.parse(module.mapOptions(terrainManifest.latitude,terrainManifest.longitude,
    terrainManifest.radius,terrainManifest.meters_per_stud));
hillsOptions.origin=[48,18,48]; hillsOptions.max_height_plates=54;
hillsOptions.limits.max_features=24000; hillsOptions.limits.max_parts=120000;
hillsOptions.terrain={elevation,fit_height:true,vertical_exaggeration:1,auto_coarsen:true,max_columns:262144};
const hills=JSON.parse(module.processImport('osm',fs.readFileSync(path.join(site,'assets/maps/telegraph-hill.json')),
    JSON.stringify(hillsOptions),()=>true));
assert.equal(hills.report.parts,12820);
assert.equal(hills.report.terrain.columns,12996);
assert.equal(hills.report.terrain.effective_meters_per_stud,8);
assert(hills.report.terrain.elevation_max_m-hills.report.terrain.elevation_min_m>100);
assert(hills.report.bounds.max[1]<=72);
assert(hills.attribution.some(source=>source.text.includes('U.S. Geological Survey')));
console.log(JSON.stringify({test:'shipped import WASM',map:plan.report.parts,buildings:plan.report.buildings,
    roads:plan.report.roads,mesh:model.report.parts,progressUpdates:updates.length,
    hills:{parts:hills.report.parts,columns:hills.report.terrain.columns,buildings:hills.report.buildings}}));
