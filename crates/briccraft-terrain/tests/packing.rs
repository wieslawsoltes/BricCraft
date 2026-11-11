use briccraft_terrain::*;
use std::collections::HashMap;

fn field() -> HeightField {
    let bounds = GeoBounds::around(0.0, 0.0, 5.0).unwrap();
    let elevation = ElevationGrid {
        width: 2,
        height: 2,
        bounds,
        grid_space: GridSpace::Geographic,
        values: vec![Some(0.0), Some(3.0), Some(1.0), Some(4.0)],
        vertical_datum: VerticalDatum::Unknown,
        source: ElevationSource {
            name: "fixture".into(),
            url: String::new(),
            attribution: "synthetic survey".into(),
            license: "CC0".into(),
            license_url: String::new(),
            resolution_m: None,
            timestamp: None,
        },
    };
    let options = TerrainOptions {
        elevation,
        vertical_exaggeration: 1.0,
        fit_height: false,
        auto_coarsen: true,
        max_columns: 1000,
    };
    let projection = GeoReference::new(bounds, 1.0, [-7, 18, 11]).unwrap();
    HeightField::sample(&projection, &options, 0.0, 54, &mut |_, _| true).unwrap()
}

#[test]
fn direct_column_packing_exactly_covers_every_solid_cell_and_keeps_surface_materials() {
    let mut field = field();
    field.cells[4].color = 14;
    field.cells[37].color = 7;
    let parts = pack_heightfield(&field, TerrainPackLimits::default(), &mut |_, _| true).unwrap();
    let mut occupied = HashMap::new();
    for p in &parts {
        let [sx, sy, sz] = p.size();
        assert_eq!(
            p.position[0].div_euclid(16),
            (p.position[0] + sx as i32 - 1).div_euclid(16)
        );
        assert_eq!(
            p.position[2].div_euclid(16),
            (p.position[2] + sz as i32 - 1).div_euclid(16)
        );
        for x in 0..sx as i32 {
            for y in 0..sy as i32 {
                for z in 0..sz as i32 {
                    let key = [p.position[0] + x, p.position[1] + y, p.position[2] + z];
                    assert!(
                        occupied.insert(key, p.color).is_none(),
                        "overlap at {key:?}"
                    );
                }
            }
        }
    }
    let b = field.bounds();
    for x in b[0]..b[2] {
        for z in b[1]..b[3] {
            let c = field.cell(x, z).unwrap();
            for y in field.base_y..c.top {
                assert_eq!(
                    occupied.remove(&[x, y, z]),
                    Some(if y == c.top - 1 { c.color } else { 28 })
                );
            }
        }
    }
    assert!(occupied.is_empty());
    assert!(parts.len() < field.filled_cells());
}

#[test]
fn packing_cancels_or_fails_budgets_without_returning_a_partial_plan() {
    let field = field();
    assert_eq!(
        pack_heightfield(&field, TerrainPackLimits::default(), &mut |_, _| false).unwrap_err(),
        TerrainError::Cancelled
    );
    assert!(matches!(
        pack_heightfield(
            &field,
            TerrainPackLimits {
                max_parts: 1,
                max_work: 1_000_000
            },
            &mut |_, _| true
        ),
        Err(TerrainError::BudgetExceeded {
            resource: "terrain parts",
            ..
        })
    ));
    assert!(matches!(
        pack_heightfield(
            &field,
            TerrainPackLimits {
                max_parts: 100_000,
                max_work: 1
            },
            &mut |_, _| true
        ),
        Err(TerrainError::BudgetExceeded {
            resource: "terrain packing work",
            ..
        })
    ));
}
