//! A small, renderer-independent Web Worker entry point. The host owns fetch,
//! file selection, caching, progress presentation and worker termination.
#![forbid(unsafe_code)]

mod elevation;
pub use elevation::elevation_grid_json;

use briccraft_import::{mesh, osm, GeoBounds, ImportPlan, ImportProgress, MeshOptions, OsmOptions};

fn mesh_options() -> MeshOptions {
    MeshOptions {
        origin: [0, 18, 0],
        max_height_plates: Some(54),
        ..MeshOptions::default()
    }
}

fn osm_options() -> OsmOptions {
    OsmOptions {
        origin: [0, 18, 0],
        max_height_plates: Some(54),
        ..OsmOptions::default()
    }
}

/// Defaults fit the application's resident height without distorting models.
pub fn default_options_json(kind: &str) -> Result<String, String> {
    match kind {
        "osm" => serde_json::to_string(&osm_options()),
        "obj" | "glb" | "gltf" => serde_json::to_string(&mesh_options()),
        _ => return Err(format!("Unsupported import kind: {kind}")),
    }
    .map_err(|e| e.to_string())
}

pub fn map_options_json(
    latitude: f64,
    longitude: f64,
    radius_m: f64,
    meters_per_stud: f64,
) -> Result<String, String> {
    let options = OsmOptions {
        bounds: GeoBounds::around(latitude, longitude, radius_m).map_err(|e| e.to_string())?,
        meters_per_stud,
        ..osm_options()
    };
    briccraft_import::GeoReference::new(options.bounds, meters_per_stud, options.origin)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&options).map_err(|e| e.to_string())
}

/// Native-callable dispatcher also makes the browser boundary testable without JS.
pub fn convert(
    kind: &str,
    bytes: &[u8],
    options_json: &str,
    progress: &mut dyn FnMut(ImportProgress) -> bool,
) -> Result<ImportPlan, String> {
    let defaults;
    let json = if options_json.trim().is_empty() {
        defaults = default_options_json(kind)?;
        defaults.as_str()
    } else {
        options_json
    };
    let text =
        || std::str::from_utf8(bytes).map_err(|_| "Input must be valid UTF-8 text".to_owned());
    let result = match kind {
        "osm" => {
            let options: OsmOptions = serde_json::from_str(json).map_err(|e| e.to_string())?;
            osm::import_overpass_json_with_progress(text()?, &options, progress)
        }
        "obj" | "glb" | "gltf" => {
            let options: MeshOptions = serde_json::from_str(json).map_err(|e| e.to_string())?;
            match kind {
                "obj" => mesh::import_obj_with_progress(text()?, None, &options, progress),
                "glb" => mesh::import_glb_with_progress(bytes, &options, progress),
                "gltf" => mesh::import_gltf_json_with_progress(text()?, &options, progress),
                _ => unreachable!(),
            }
        }
        _ => return Err(format!("Unsupported import kind: {kind}")),
    };
    result.map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
mod bindings {
    use super::*;
    use wasm_bindgen::prelude::*;

    /// Decode raw DEM channels and sample a complete georeferenced area off the UI thread.
    #[wasm_bindgen(js_name = elevationGrid)]
    pub fn elevation_grid(metadata: &str, png_bytes: &[u8]) -> Result<String, JsValue> {
        elevation_grid_json(metadata, png_bytes).map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(js_name = processImport)]
    pub fn process_import(
        kind: &str,
        bytes: &[u8],
        options_json: &str,
        progress: js_sys::Function,
    ) -> Result<String, JsValue> {
        let mut callback_error = None;
        let result = convert(kind, bytes, options_json, &mut |update| {
            let json =
                serde_json::to_string(&update).expect("progress contains only finite integers");
            match progress.call1(&JsValue::NULL, &JsValue::from_str(&json)) {
                Ok(value) => value.as_bool().unwrap_or(true),
                Err(error) => {
                    callback_error = Some(error);
                    false
                }
            }
        });
        if let Some(error) = callback_error {
            return Err(error);
        }
        let plan = result.map_err(|error| JsValue::from_str(&error))?;
        serde_json::to_string(&plan).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = defaultOptions)]
    pub fn default_options(kind: &str) -> Result<String, JsValue> {
        default_options_json(kind).map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = mapOptions)]
    pub fn map_options(
        latitude: f64,
        longitude: f64,
        radius_m: f64,
        meters_per_stud: f64,
    ) -> Result<String, JsValue> {
        map_options_json(latitude, longitude, radius_m, meters_per_stud)
            .map_err(|e| JsValue::from_str(&e))
    }

    #[wasm_bindgen(js_name = mapQuery)]
    pub fn map_query(options_json: &str) -> Result<String, JsValue> {
        let options: OsmOptions =
            serde_json::from_str(options_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        osm::overpass_query(options.bounds).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_defaults_and_dispatch_remain_renderer_independent() {
        let options: MeshOptions =
            serde_json::from_str(&default_options_json("obj").unwrap()).unwrap();
        assert_eq!(options.origin, [0, 18, 0]);
        assert_eq!(options.max_height_plates, Some(54));
        let plan = convert(
            "obj",
            b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
            "",
            &mut |_| true,
        )
        .unwrap();
        assert_eq!(plan.report.source_triangles, 1);
        assert!(plan.report.bounds.max[1] <= 72);
        assert!(convert("obj", b"\xff", "", &mut |_| true).is_err());
        assert!(default_options_json("fbx").is_err());
    }

    #[test]
    fn map_options_validate_coordinates_and_projection() {
        let options: OsmOptions =
            serde_json::from_str(&map_options_json(52.516275, 13.377704, 160.0, 4.0).unwrap())
                .unwrap();
        assert!(options.bounds.south < 52.516275 && options.bounds.north > 52.516275);
        assert_eq!(options.max_height_plates, Some(54));
        assert!(map_options_json(91.0, 0.0, 160.0, 4.0).is_err());
    }
}
