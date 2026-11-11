use super::geometry;
use crate::{error::invalid, limits::limit, ImportLimits, ImportReport, ImportResult};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Deserialize)]
struct Document {
    elements: Vec<Element>,
    #[serde(default)]
    osm3s: Metadata,
    #[serde(default)]
    remark: Option<String>,
}
#[derive(Default, Deserialize)]
struct Metadata {
    timestamp_osm_base: Option<String>,
}
#[derive(Clone, Copy, Deserialize)]
struct Point {
    lat: f64,
    lon: f64,
}
#[derive(Deserialize)]
struct Element {
    #[serde(rename = "type")]
    kind: String,
    id: i64,
    #[serde(default)]
    lat: Option<f64>,
    #[serde(default)]
    lon: Option<f64>,
    #[serde(default)]
    nodes: Vec<i64>,
    #[serde(default)]
    geometry: Vec<Option<Point>>,
    #[serde(default)]
    tags: BTreeMap<String, String>,
    #[serde(default)]
    members: Vec<Member>,
}
#[derive(Deserialize)]
struct Member {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "ref")]
    reference: i64,
    #[serde(default)]
    role: String,
    #[serde(default)]
    geometry: Vec<Option<Point>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Water,
    Road,
    Building,
}
pub(super) struct Feature {
    pub id: i64,
    pub osm_type: String,
    pub kind: Kind,
    pub tags: BTreeMap<String, String>,
    pub outer: Vec<Vec<[f64; 2]>>,
    pub inner: Vec<Vec<[f64; 2]>>,
    pub line: Vec<[f64; 2]>,
}
fn kind(tags: &BTreeMap<String, String>) -> Option<Kind> {
    if ["building", "building:part"]
        .iter()
        .any(|key| tags.get(*key).is_some_and(|v| v != "no"))
    {
        Some(Kind::Building)
    } else if tags.contains_key("highway") {
        Some(Kind::Road)
    } else if tags.get("natural").is_some_and(|v| v == "water")
        || tags.contains_key("waterway")
        || tags.get("landuse").is_some_and(|v| v == "reservoir")
    {
        Some(Kind::Water)
    } else {
        None
    }
}
fn geometry(points: &[Option<Point>]) -> Option<Vec<[f64; 2]>> {
    points.iter().map(|p| p.map(|p| [p.lat, p.lon])).collect()
}

pub(super) fn parse(
    source: &str,
    limits: &ImportLimits,
    report: &mut ImportReport,
) -> ImportResult<Vec<Feature>> {
    let document: Document = serde_json::from_str(source)?;
    if let Some(remark) = document.remark {
        return Err(invalid(format!(
            "map provider returned an incomplete result: {remark}"
        )));
    }
    limit("map elements", document.elements.len(), limits.max_features)?;
    report.input_features = document.elements.len();
    report.source_timestamp = document.osm3s.timestamp_osm_base;
    let nodes: HashMap<i64, [f64; 2]> = document
        .elements
        .iter()
        .filter_map(|e| Some((e.id, [e.lat?, e.lon?])))
        .collect();
    let mut ways = HashMap::new();
    let mut vertices = 0;
    for e in document.elements.iter().filter(|e| e.kind == "way") {
        let points = if e.geometry.is_empty() {
            e.nodes
                .iter()
                .map(|id| nodes.get(id).copied())
                .collect::<Option<Vec<_>>>()
        } else {
            geometry(&e.geometry)
        };
        if let Some(points) = points {
            vertices += points.len();
            limit("map vertices", vertices, limits.max_vertices)?;
            ways.insert(e.id, points);
        }
    }
    let mut result = Vec::new();
    let mut consumed = HashSet::new();
    let mut ring_work = 0;
    for e in document.elements.iter().filter(|e| e.kind == "relation") {
        let Some(kind) = kind(&e.tags) else {
            continue;
        };
        if kind == Kind::Road {
            continue;
        }
        let mut outers = Vec::new();
        let mut inners = Vec::new();
        let mut members = Vec::new();
        let mut missing_inner_geometry = e
            .members
            .iter()
            .any(|m| m.role == "inner" && m.kind != "way");
        for member in e.members.iter().filter(|m| m.kind == "way") {
            let points = if member.geometry.is_empty() {
                ways.get(&member.reference).cloned()
            } else {
                geometry(&member.geometry)
            };
            if let Some(points) = points {
                vertices += points.len();
                limit("map vertices", vertices, limits.max_vertices)?;
                if member.role == "inner" {
                    inners.push(points);
                } else if member.role.is_empty() || member.role == "outer" {
                    outers.push(points);
                }
                members.push(member.reference);
            } else {
                missing_inner_geometry |= member.role == "inner";
                report.warn("Some relation members lack complete node geometry; incomplete rings were omitted.");
            }
        }
        let (outer, missing_outer) = geometry::rings(outers, &mut ring_work, limits.max_work)?;
        let (inner, missing_inner) = geometry::rings(inners, &mut ring_work, limits.max_work)?;
        if missing_outer + missing_inner > 0 {
            report.warn("Some multipolygon rings are incomplete and were omitted without inventing closing segments.");
        }
        if !outer.is_empty() && missing_inner == 0 && !missing_inner_geometry {
            consumed.extend(members);
            result.push(Feature {
                id: e.id,
                osm_type: "relation".into(),
                kind,
                tags: e.tags.clone(),
                outer,
                inner,
                line: Vec::new(),
            });
        }
    }
    for e in document
        .elements
        .iter()
        .filter(|e| e.kind == "way" && !consumed.contains(&e.id))
    {
        let Some(kind) = kind(&e.tags) else {
            continue;
        };
        let Some(points) = ways.get(&e.id) else {
            report.warn("Some ways lack full node geometry and were skipped.");
            continue;
        };
        if points.len() < 2 {
            continue;
        }
        let closed = points.len() >= 4 && geometry::same(points[0], *points.last().unwrap());
        if kind == Kind::Building && !closed {
            report
                .warn("An open building footprint was skipped; its missing edge was not invented.");
            continue;
        }
        let area = closed && (kind != Kind::Road || e.tags.get("area").is_some_and(|v| v == "yes"));
        result.push(Feature {
            id: e.id,
            osm_type: "way".into(),
            kind,
            tags: e.tags.clone(),
            outer: if area {
                vec![points.clone()]
            } else {
                Vec::new()
            },
            inner: Vec::new(),
            line: if area { Vec::new() } else { points.clone() },
        });
    }
    result.sort_by_key(|f| {
        (
            match f.kind {
                Kind::Water => 0,
                Kind::Road => 1,
                Kind::Building => 2,
            },
            f.id,
        )
    });
    Ok(result)
}
