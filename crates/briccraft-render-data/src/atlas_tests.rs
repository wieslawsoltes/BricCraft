use super::*;
use crate::data_texture::encode_id;
use briccraft_world::{DEFAULT_SEED, Placement};

fn packed_cell(atlas: &SceneAtlas, index: usize) -> u32 {
    let p = &atlas.cells.pixels[index * 4..index * 4 + 3];
    u32::from(p[0]) | u32::from(p[1]) << 8 | u32::from(p[2]) << 16
}

#[test]
fn packed_ids_preserve_every_byte_at_channel_boundaries() {
    for value in [0, 1, 255, 256, 65535, 65536, 0x00ff_ffff] {
        let [r, g, b] = encode_id(value);
        assert_eq!(u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16, value);
    }
}

#[test]
fn data_texture_padding_is_opaque_to_preserve_encoded_channels() {
    let atlas = PixelAtlas::new(1025).unwrap();
    assert_eq!(atlas.height, 2);
    assert!(atlas.pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
    assert!(matches!(
        PixelAtlas::new(MAX_ATLAS_TEXELS + 1),
        Err(AtlasError::TextureTooLarge)
    ));
}

#[test]
fn active_assembly_moves_to_local_volume_without_remaining_in_static_grid() {
    let mut world = World::empty(1);
    let active = world.place(Placement::new(3, 0, [4, 3, 4], 0)).unwrap();
    let fixed = world.place(Placement::new(3, 1, [20, 6, 20], 0)).unwrap();
    let atlas = SceneAtlas::build(&world, &[active], None).unwrap();
    assert_eq!(atlas.vehicle_min, [4, 3, 4]);
    assert_eq!(atlas.vehicle_size, [2, 3, 4]);
    assert_eq!(packed_cell(&atlas, world.index([4, 3, 4]).unwrap()), 0);
    assert_eq!(
        packed_cell(&atlas, atlas.vehicle_offset),
        atlas.packed_ids[&active]
    );
    assert_eq!(
        packed_cell(&atlas, world.index([20, 6, 20]).unwrap()),
        atlas.packed_ids[&fixed]
    );
    let packed = atlas.packed_ids[&active];
    assert_eq!(
        &atlas.parts.pixels[packed as usize * 16..packed as usize * 16 + 3],
        &[0, 0, 0]
    );
}

#[test]
fn coarse_tiles_include_stud_overhang_above_a_tile_boundary() {
    let mut world = World::empty(1);
    world.place(Placement::new(1, 0, [7, 3, 7], 0)).unwrap();
    let atlas = SceneAtlas::build(&world, &[], None).unwrap();
    let coarse_width = 96_usize.div_ceil(4);
    let coarse_depth = 96_usize.div_ceil(4);
    let below = (7 / 4) * coarse_width + 7 / 4;
    let above = coarse_depth * coarse_width + below;
    assert_eq!(packed_cell(&atlas, atlas.coarse_offset + below), 1);
    assert_eq!(packed_cell(&atlas, atlas.coarse_offset + above), 1);
}

#[test]
fn detached_vehicle_ids_do_not_replace_static_ids_after_a_window_swap() {
    let mut world = World::empty(1);
    let fixed = world.place(Placement::new(3, 0, [20, 6, 20], 0)).unwrap();
    let body = [Instance {
        id: fixed,
        part_id: 3,
        color: 1,
        position: [-100, 3, 200],
        rotation: 0,
    }];
    let atlas = SceneAtlas::build(&world, &[], Some(&body)).unwrap();
    assert_eq!(atlas.stats.instances, 2);
    assert_eq!(atlas.stats.static_instances, 1);
    assert_eq!(atlas.stats.vehicle_instances, 1);
    assert_eq!(atlas.packed_ids[&fixed], 1);
    assert_eq!(packed_cell(&atlas, world.index([20, 6, 20]).unwrap()), 1);
    assert_eq!(packed_cell(&atlas, atlas.vehicle_offset), 2);
    assert_eq!(atlas.vehicle_min, [-100, 3, 200]);
    assert_eq!(atlas.parts.pixels[2 * 16 + 14], 1);
}

#[test]
fn vehicle_rebase_preserves_both_textures_when_its_local_geometry_is_unchanged() {
    let world = World::empty(1);
    let mut body = [Instance {
        id: 800,
        part_id: 3,
        color: 1,
        position: [40, 3, 40],
        rotation: 1,
    }];
    let before = SceneAtlas::build(&world, &[], Some(&body)).unwrap();
    body[0].position[0] -= 160;
    body[0].position[2] += 96;
    let after = SceneAtlas::build(&world, &[], Some(&body)).unwrap();
    assert!(
        before.cells == after.cells,
        "origin rebasing must not upload cell geometry"
    );
    assert!(
        before.parts == after.parts,
        "origin rebasing must not upload descriptors"
    );
    assert_ne!(before.vehicle_min, after.vehicle_min);
}

#[test]
fn recoloring_preserves_the_cell_texture_and_changes_only_descriptors() {
    let mut world = World::empty(1);
    let id = world.place(Placement::new(3, 0, [20, 6, 20], 0)).unwrap();
    let before = SceneAtlas::build(&world, &[], None).unwrap();
    let mut changed = world.remove(id).unwrap();
    changed.color = 1;
    world.restore(&[changed]).unwrap();
    let after = SceneAtlas::build(&world, &[], None).unwrap();
    assert!(before.cells == after.cells);
    assert!(before.parts != after.parts);
}

#[test]
fn cuboid_packing_matches_the_actual_world_occupancy_including_rotated_parts() {
    let world = World::generate(DEFAULT_SEED);
    let atlas = SceneAtlas::build(&world, &[], None).unwrap();
    for (index, id) in world.occupancy().iter().copied().enumerate() {
        let expected = id
            .checked_sub(1)
            .map(|id| atlas.packed_ids[&id])
            .unwrap_or(0);
        assert_eq!(
            packed_cell(&atlas, index),
            expected,
            "placement cell {index}"
        );
    }
    assert_eq!(
        atlas.stats.occupied_cells,
        world.occupancy().iter().filter(|id| **id != 0).count()
    );
}

#[test]
fn hundred_thousand_parts_fit_the_fixed_window_and_rgb24_indices() {
    let mut world = World::empty(1);
    let parts: Vec<_> = (0..100_000_u32)
        .map(|id| Instance {
            id,
            part_id: 20,
            color: (id as usize % PALETTE.len()) as u8,
            position: [
                (id % 96) as i32,
                (id / (96 * 96)) as i32,
                (id / 96 % 96) as i32,
            ],
            rotation: 0,
        })
        .collect();
    world.restore(&parts).unwrap();
    let atlas = SceneAtlas::build(&world, &[], None).unwrap();
    assert_eq!(atlas.stats.instances, 100_000);
    assert_eq!(atlas.stats.occupied_cells, 100_000);
    assert_eq!(atlas.stats.active_cells, 96 * 72 * 96);
    assert!(atlas.stats.texture_bytes < 8 * 1024 * 1024);
    for index in [0, 254, 255, 65534, 65535, 99999] {
        assert_eq!(packed_cell(&atlas, index), index as u32 + 1);
    }
    assert_eq!(packed_cell(&atlas, 100_000), 0);
}

#[test]
fn detached_body_limits_bound_traversal_and_cpu_work() {
    let world = World::empty(1);
    let piece = Instance {
        id: 0,
        part_id: 20,
        color: 0,
        position: [0; 3],
        rotation: 0,
    };
    let too_many = vec![piece.clone(); MAX_VEHICLE_PARTS + 1];
    assert!(matches!(
        SceneAtlas::build(&world, &[], Some(&too_many)),
        Err(AtlasError::TooManyVehicleParts)
    ));
    let mut distant = piece.clone();
    distant.position[0] = 96;
    assert!(matches!(
        SceneAtlas::build(&world, &[], Some(&[piece, distant])),
        Err(AtlasError::VehicleTooLarge)
    ));
}
