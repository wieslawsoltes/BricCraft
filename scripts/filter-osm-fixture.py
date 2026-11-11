#!/usr/bin/env python3
"""Retain every supported OSM root and its exact reference closure, without clipping.

Inputs are original OSM API/Overpass JSON files. Supplements provide complete
way/relation responses when a map response references geometry outside its box.
This tool does no fetching, coordinate rounding, polygon repair or tag rewriting.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path


NATURAL = {"wood", "scrub", "grassland", "heath", "sand", "beach", "bare_rock", "wetland", "glacier"}
LEISURE = {"park", "garden", "golf_course", "pitch", "recreation_ground"}
GEOMETRY_FIELDS = ("type", "id", "lat", "lon", "nodes", "members", "geometry", "tags")


def supported(element):
    tags = element.get("tags", {})
    return element["type"] in {"way", "relation"} and (
        any(key in tags and tags[key] != "no" for key in ("building", "building:part"))
        or "highway" in tags
        or tags.get("natural") == "water"
        or "waterway" in tags
        or "landuse" in tags
        or tags.get("natural") in NATURAL
        or tags.get("leisure") in LEISURE
    )


def key(element):
    return element["type"], element["id"]


def derive(original, supplements, maximum):
    index = {key(element): element for element in original["elements"]}
    if len(index) != len(original["elements"]):
        raise ValueError("Original source contains duplicate identities")
    roots = {key(element) for element in original["elements"] if supported(element)}
    for supplement in supplements:
        for element in supplement["elements"]:
            identity = key(element)
            previous = index.get(identity)
            if previous is not None:
                if any(previous.get(field) != element.get(field) for field in GEOMETRY_FIELDS):
                    raise ValueError(f"Conflicting source geometry/tags for {identity}")
            else:
                index[identity] = element
    retained, missing, pending = set(), set(), list(roots)
    while pending:
        identity = pending.pop()
        if identity in retained:
            continue
        element = index.get(identity)
        if element is None:
            missing.add(identity)
            continue
        retained.add(identity)
        if element["type"] == "way":
            pending.extend(("node", node) for node in element.get("nodes", []))
        elif element["type"] == "relation":
            pending.extend((member["type"], member["ref"]) for member in element.get("members", []))
    if missing:
        raise ValueError(f"Missing required source identities: {sorted(missing)}")
    if len(retained) > maximum:
        raise ValueError(f"Complete supported-feature closure has {len(retained)} elements, above budget {maximum}; no output was truncated")
    assert roots <= retained
    # Preserve original objects byte-for-byte semantically; deterministic ordering
    # only groups nodes before ways before relations for a readable source graph.
    rank = {"node": 0, "way": 1, "relation": 2}
    elements = [index[identity] for identity in sorted(retained, key=lambda k: (rank[k[0]], k[1]))]
    output = {k: v for k, v in original.items() if k != "elements"}
    output["elements"] = elements
    return output, roots


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("supplements", nargs="*", type=Path)
    parser.add_argument("--max-elements", type=int, default=24000)
    parser.add_argument("--provenance", type=Path)
    args = parser.parse_args()
    files = [args.input, *args.supplements]
    raw = [path.read_bytes() for path in files]
    documents = [json.loads(data) for data in raw]
    output, roots = derive(documents[0], documents[1:], args.max_elements)
    result = (json.dumps(output, ensure_ascii=False, separators=(",", ":")) + "\n").encode()
    counts = dict(collections.Counter(element["type"] for element in output["elements"]))
    provenance = {
        "method": "All supported original root ways/relations and their complete referenced way/node/relation closure; no coordinate, tag, ring or extent modifications",
        "source_files": [{"name": path.name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()} for path, data in zip(files, raw)],
        "original_elements": len(documents[0]["elements"]),
        "supported_root_ids": {kind: sorted(identity for element_type, identity in roots if element_type == kind) for kind in ("way", "relation")},
        "retained_elements": len(output["elements"]),
        "retained_counts": counts,
        "max_features": args.max_elements,
        "output_bytes": len(result),
        "output_sha256": hashlib.sha256(result).hexdigest(),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(result)
    if args.provenance:
        args.provenance.write_text(json.dumps(provenance, indent=2) + "\n")
    print(json.dumps({k: v for k, v in provenance.items() if k != "supported_root_ids"}, indent=2))


if __name__ == "__main__":
    main()
