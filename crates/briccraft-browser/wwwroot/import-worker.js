// All source parsing, triangulation, voxelization and brick packing run off the UI thread.
import init, { processImport, defaultOptions, mapOptions, elevationGrid } from './import-engine/briccraft_import_worker.js';
let engine;
self.onmessage = async ({data}) => {
    try {
        engine ||= init(); await engine;
        const options=JSON.parse(data.kind==='osm'
            ? mapOptions(data.latitude,data.longitude,data.radius,data.meters_per_stud)
            : defaultOptions(data.kind));
        options.origin=data.kind==='osm' ? [48,18,48] : [0,18,0];
        options.max_height_plates=data.max_height_plates || 54;
        if(data.kind==='osm' && data.elevation) {
            self.postMessage({type:'progress',progress:{stage:'Sampling real terrain',completed:0,total:1}});
            const elevation=JSON.parse(elevationGrid(JSON.stringify(data.elevation.metadata),new Uint8Array(data.elevation.bytes)));
            options.terrain={elevation,fit_height:data.fit_height!==false,
                vertical_exaggeration:data.vertical_exaggeration ?? 1,auto_coarsen:true,max_columns:262144};
        }
        if(data.kind!=='osm') options.fit_size_studs=data.size || 48;
        if(data.units_per_stud) options.units_per_stud=data.units_per_stud;
        // Budgets apply before and throughout conversion, not just to the final JSON.
        options.limits.max_parts=120000;
        // The bundled complete-area fixture includes all referenced OSM geometry.
        if(data.terrain_sample) options.limits.max_features=24000;
        let last=0;
        const source=processImport(data.kind,new Uint8Array(data.bytes),JSON.stringify(options),json => {
            const now=performance.now();
            if(now-last>80) { self.postMessage({type:'progress',progress:JSON.parse(json)}); last=now; }
            return true;
        });
        const plan=JSON.parse(source);
        if(data.kind!=='osm') {
            const b=plan.report.bounds;
            const dx=Math.round(48-(b.min[0]+b.max[0])/2),dz=Math.round(48-(b.min[2]+b.max[2])/2);
            for(const part of plan.placements) { part.position[0]+=dx; part.position[2]+=dz; }
            b.min[0]+=dx; b.max[0]+=dx; b.min[2]+=dz; b.max[2]+=dz;
        }
        plan.report.source_name=data.name;
        if(data.attribution) plan.attribution.push(data.attribution);
        self.postMessage({type:'complete',source:JSON.stringify(plan),name:data.name,report:plan.report});
    } catch(error) { self.postMessage({type:'error',message:String(error?.message || error)}); }
};
