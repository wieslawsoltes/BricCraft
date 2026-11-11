use briccraft_design::{CaptureKind, DesignLibrary, DesignMetadata, MAX_DESIGNS, MAX_PARTS};
use briccraft_world::{Blueprint, BuildOp, Placement, World};

fn metadata(kind: CaptureKind) -> DesignMetadata {
    DesignMetadata {
        source_world_id: "fixture-world".into(),
        source_world_name: "My building world".into(),
        source_origin: [9_007_199_254_741_013, 18, -9_007_199_254_741_011],
        capture: kind,
    }
}

#[test]
fn all_builtin_constructions_preserve_colors_geometry_and_quarter_turns() {
    for blueprint in Blueprint::ALL {
        let mut library = DesignLibrary::default();
        let source = blueprint.placements([13, 2, 17], 0);
        let id = library
            .capture(
                blueprint.name(),
                &source,
                metadata(CaptureKind::ConstructionGroup),
            )
            .unwrap();
        let design = library.get(id).unwrap();
        assert_eq!(design.part_count(), source.len());
        for rotation in 0..4 {
            let expected = blueprint.placements([25, 0, 25], rotation);
            let actual = design.placements([25, 0, 25], rotation).unwrap();
            assert_eq!(actual, expected, "{} rotation {rotation}", blueprint.name());
            let mut world = World::empty(1);
            world
                .apply(&actual.into_iter().map(BuildOp::Place).collect::<Vec<_>>())
                .unwrap();
            assert_eq!(world.len(), source.len());
        }
    }
}

#[test]
fn connected_captures_require_face_contact_but_explicit_groups_preserve_separate_parts() {
    let parts = [
        Placement::new(20, 0, [40, 7, 40], 0),
        Placement::new(20, 3, [43, 7, 40], 0),
    ];
    let mut library = DesignLibrary::default();
    assert!(library
        .capture(
            "Disconnected",
            &parts,
            metadata(CaptureKind::ConnectedAssembly)
        )
        .is_err());
    assert!(library.is_empty());
    let id = library
        .capture(
            "Decorative group",
            &parts,
            metadata(CaptureKind::ConstructionGroup),
        )
        .unwrap();
    let design = library.get(id).unwrap();
    assert_eq!(design.component_count(), 2);
    assert_eq!(design.dimensions(), [4, 1, 1]);
    assert_eq!(design.relative_parts()[0].position, [0, 0, 0]);
    assert_eq!(design.relative_parts()[1].position, [3, 0, 0]);
}

#[test]
fn captures_reject_invalid_parts_overlap_extents_and_unbounded_counts_atomically() {
    let one = Placement::new(20, 0, [0, 0, 0], 0);
    let mut library = DesignLibrary::default();
    let invalid = [
        vec![Placement {
            part_id: u16::MAX,
            ..one
        }],
        vec![Placement {
            color: u8::MAX,
            ..one
        }],
        vec![Placement { rotation: 4, ..one }],
        vec![one, one],
        vec![
            one,
            Placement {
                position: [32, 0, 0],
                ..one
            },
        ],
        vec![one; MAX_PARTS + 1],
        vec![],
    ];
    for parts in invalid {
        assert!(library
            .capture("Invalid", &parts, metadata(CaptureKind::ConstructionGroup))
            .is_err());
        assert!(library.is_empty());
    }
    let id = library
        .capture(
            "First valid",
            &[one],
            metadata(CaptureKind::ConnectedAssembly),
        )
        .unwrap();
    assert_eq!(id, 1, "failed captures must not consume an identity");
    assert!(library
        .get(id)
        .unwrap()
        .placements([i32::MAX, 0, 0], 0)
        .is_err());
    assert!(library.get(id).unwrap().placements([0; 3], 4).is_err());
}

#[test]
fn names_ids_and_capacity_are_stable_across_rename_delete_and_reload() {
    let part = [Placement::new(20, 0, [0; 3], 0)];
    let mut library = DesignLibrary::default();
    let id = library
        .capture(
            "  First build  ",
            &part,
            metadata(CaptureKind::ConnectedAssembly),
        )
        .unwrap();
    assert_eq!(library.get(id).unwrap().name, "First build");
    assert!(library
        .capture(
            "FIRST BUILD",
            &part,
            metadata(CaptureKind::ConnectedAssembly)
        )
        .is_err());
    for name in ["", "inside\nnewline"] {
        assert!(library.rename(id, name).is_err());
    }
    assert!(library.rename(id, &"x".repeat(65)).is_err());
    let original = library.get(id).unwrap().relative_parts().to_vec();
    let source = library.get(id).unwrap().metadata().clone();
    library.rename(id, "Renamed").unwrap();
    assert_eq!(library.get(id).unwrap().relative_parts(), original);
    assert_eq!(library.get(id).unwrap().metadata(), &source);
    let serialized = serde_json::to_string(&library).unwrap();
    let mut loaded: DesignLibrary = serde_json::from_str(&serialized).unwrap();
    loaded.validate().unwrap();
    assert_eq!(
        loaded, library,
        "exact source coordinates beyond 2^53 must round-trip"
    );
    loaded.delete(id).unwrap();
    let next = loaded
        .capture("Next", &part, metadata(CaptureKind::ConnectedAssembly))
        .unwrap();
    assert!(next > id, "deleting a design must not recycle its identity");
    for index in 1..MAX_DESIGNS {
        loaded
            .capture(
                &format!("Build {index}"),
                &part,
                metadata(CaptureKind::ConnectedAssembly),
            )
            .unwrap();
    }
    assert_eq!(loaded.len(), MAX_DESIGNS);
    assert!(loaded
        .capture("Too many", &part, metadata(CaptureKind::ConnectedAssembly))
        .is_err());
}

#[test]
fn saved_geometry_and_metadata_are_checked_before_a_library_is_used() {
    let mut library = DesignLibrary::default();
    library
        .capture(
            "Original",
            &[Placement::new(20, 0, [0; 3], 0)],
            metadata(CaptureKind::ConnectedAssembly),
        )
        .unwrap();
    let source = serde_json::to_value(library).unwrap();
    for (field, value) in [
        ("version", serde_json::json!(2)),
        ("next_id", serde_json::json!(0)),
    ] {
        let mut corrupt = source.clone();
        corrupt[field] = value;
        let parsed: DesignLibrary = serde_json::from_value(corrupt).unwrap();
        assert!(parsed.validate().is_err());
    }
    let mut corrupt = source.clone();
    corrupt["designs"][0]["dimensions"] = serde_json::json!([32, 72, 32]);
    assert!(serde_json::from_value::<DesignLibrary>(corrupt)
        .unwrap()
        .validate()
        .is_err());
    let mut corrupt = source.clone();
    corrupt["designs"][0]["parts"][0]["position"] = serde_json::json!([1, 0, 0]);
    assert!(serde_json::from_value::<DesignLibrary>(corrupt)
        .unwrap()
        .validate()
        .is_err());
    let mut corrupt = source;
    corrupt["designs"][0]["metadata"]["source_world_name"] = "x".repeat(1_025).into();
    assert!(serde_json::from_value::<DesignLibrary>(corrupt)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn portable_documents_keep_source_evidence_but_assign_destination_identity_and_names() {
    let part = [Placement::new(20, 4, [0; 3], 1)];
    let mut source = DesignLibrary::default();
    let source_id = source
        .capture("Reusable", &part, metadata(CaptureKind::ConnectedAssembly))
        .unwrap();
    let source_design = source.get(source_id).unwrap().clone();
    let mut document = serde_json::to_value(source.export(source_id).unwrap()).unwrap();
    document["design"]["id"] = u32::MAX.into();
    let parsed = serde_json::from_value(document).unwrap();
    let mut destination = DesignLibrary::default();
    let existing_id = destination
        .capture("Reusable", &part, metadata(CaptureKind::ConnectedAssembly))
        .unwrap();
    let imported_id = destination.import(parsed).unwrap();
    assert_ne!(imported_id, existing_id);
    assert_ne!(imported_id, u32::MAX);
    let imported = destination.get(imported_id).unwrap();
    assert_eq!(imported.name, "Reusable (2)");
    assert_eq!(imported.relative_parts(), source_design.relative_parts());
    assert_eq!(imported.metadata(), source_design.metadata());
    destination.validate().unwrap();
    for version in [0, 2, 255] {
        let mut invalid = serde_json::to_value(source.export(source_id).unwrap()).unwrap();
        invalid["version"] = version.into();
        let count = destination.len();
        assert!(destination
            .import(serde_json::from_value(invalid).unwrap())
            .is_err());
        assert_eq!(destination.len(), count);
    }
}
