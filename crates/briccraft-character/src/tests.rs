use crate::grid::Grid;
use crate::*;
use glam::{IVec3, Vec2, Vec3};
use std::collections::HashSet;

#[derive(Default)]
struct Fixture {
    solid: HashSet<IVec3>,
}

impl Fixture {
    fn fill(&mut self, min: [i32; 3], max: [i32; 3]) {
        for y in min[1]..max[1] {
            for z in min[2]..max[2] {
                for x in min[0]..max[0] {
                    self.solid.insert(IVec3::new(x, y, z));
                }
            }
        }
    }
}

impl CollisionGrid for Fixture {
    fn dimensions(&self) -> IVec3 {
        IVec3::new(32, 80, 32)
    }
    fn cell_size(&self) -> Vec3 {
        Vec3::new(1.0, 0.4, 1.0)
    }
    fn is_solid(&self, cell: IVec3) -> bool {
        assert!(cell.min_element() >= 0 && cell.cmplt(self.dimensions()).all());
        self.solid.contains(&cell)
    }
}

fn drive(
    controller: &mut CharacterController,
    grid: &Fixture,
    mut p: Vec3,
    input: CharacterInput,
    frames: usize,
) -> CharacterMotion {
    let mut motion = CharacterMotion::default();
    for _ in 0..frames {
        motion = controller.advance(p, input, 1.0 / 60.0, grid);
        assert!(
            Grid::new(grid)
                .unwrap()
                .clear(motion.position, controller.config()),
            "overlap at {:?}",
            motion.position
        );
        p = motion.position;
    }
    motion
}

fn moving(direction: Vec2) -> CharacterInput {
    CharacterInput {
        direction,
        speed: 7.0,
        jump: false,
    }
}

#[test]
fn wall_stops_the_body_at_its_radius() {
    let mut grid = Fixture::default();
    grid.fill([5, 0, 0], [6, 12, 20]);
    let mut controller = CharacterController::default();
    let motion = drive(
        &mut controller,
        &grid,
        Vec3::new(2.0, 0.0, 5.0),
        moving(Vec2::X),
        90,
    );
    assert!((motion.position.x - (5.0 - controller.config().radius)).abs() < 0.001);
    assert!(motion.grounded);
    assert!(motion.velocity.x.abs() < 0.001);
}

#[test]
fn diagonal_contact_slides_along_a_wall_without_entering_it() {
    let mut grid = Fixture::default();
    grid.fill([5, 0, 0], [6, 12, 25]);
    let mut controller = CharacterController::default();
    let motion = drive(
        &mut controller,
        &grid,
        Vec3::new(2.0, 0.0, 3.0),
        moving(Vec2::ONE),
        120,
    );
    assert!(motion.position.x < 4.681);
    assert!(motion.position.y < 0.001);
    assert!(
        motion.position.z > 11.0,
        "wall blocked tangential travel: {:?}",
        motion.position
    );
}

#[test]
fn diagonal_input_does_not_gain_speed() {
    let grid = Fixture::default();
    let p = Vec3::new(4.0, 0.0, 4.0);
    let a = drive(
        &mut CharacterController::default(),
        &grid,
        p,
        moving(Vec2::X),
        90,
    );
    let b = drive(
        &mut CharacterController::default(),
        &grid,
        p,
        moving(Vec2::ONE),
        90,
    );
    assert!(((a.position - p).length() - (b.position - p).length()).abs() < 0.0001);
}

#[test]
fn two_plate_step_is_climbed_but_three_plate_wall_is_not() {
    for (plates, crosses) in [(1, true), (2, true), (3, false)] {
        let mut grid = Fixture::default();
        grid.fill([5, 0, 0], [12, plates, 20]);
        let mut controller = CharacterController::default();
        let motion = drive(
            &mut controller,
            &grid,
            Vec3::new(3.0, 0.0, 5.0),
            moving(Vec2::X),
            60,
        );
        assert_eq!(
            motion.position.x > 6.0,
            crosses,
            "{plates} plates: {:?}",
            motion.position
        );
        if crosses {
            assert!((motion.position.y - plates as f32 * 0.4).abs() < 0.001);
        }
    }
}

#[test]
fn step_requires_headroom_and_never_lifts_through_a_ceiling() {
    let mut grid = Fixture::default();
    grid.fill([5, 0, 0], [12, 2, 20]);
    grid.fill([2, 7, 0], [12, 8, 20]); // Ceiling at 2.8, only 0.3 above the body.
    let mut controller = CharacterController::default();
    let motion = drive(
        &mut controller,
        &grid,
        Vec3::new(3.0, 0.0, 5.0),
        moving(Vec2::X),
        60,
    );
    assert!(motion.position.x < 4.681);
    assert!(motion.position.y < 0.001);
}

#[test]
fn one_plate_step_uses_partial_headroom_below_full_step_height() {
    let mut grid = Fixture::default();
    grid.fill([5, 0, 0], [12, 1, 20]);
    grid.fill([2, 8, 0], [12, 9, 20]); // 0.7 clearance: enough for 0.4 but not 0.8.
    let mut controller = CharacterController::default();
    let motion = drive(
        &mut controller,
        &grid,
        Vec3::new(3.0, 0.0, 5.0),
        moving(Vec2::X),
        60,
    );
    assert!(motion.position.x > 6.0);
    assert!((motion.position.y - 0.4).abs() < 0.001);
}

#[test]
fn jump_stops_at_the_ceiling_and_a_held_key_does_not_repeat() {
    let mut grid = Fixture::default();
    grid.fill([0, 8, 0], [20, 9, 20]);
    let mut controller = CharacterController::default();
    let mut p = drive(
        &mut controller,
        &grid,
        Vec3::new(5.0, 0.0, 5.0),
        CharacterInput::default(),
        1,
    )
    .position;
    let input = CharacterInput {
        jump: true,
        ..CharacterInput::default()
    };
    let mut hit = false;
    let mut peak = 0.0_f32;
    for _ in 0..100 {
        let motion = controller.advance(p, input, 1.0 / 60.0, &grid);
        peak = peak.max(motion.position.y);
        assert!(motion.position.y + controller.config().height <= 3.20001);
        hit |= motion.hit_ceiling;
        p = motion.position;
    }
    assert!(hit && peak > 0.5);
    assert!(p.y < 0.001 && controller.grounded());
    controller.advance(p, CharacterInput::default(), 1.0 / 60.0, &grid);
    let jumped = controller.advance(p, input, 1.0 / 60.0, &grid);
    assert!(jumped.velocity.y > 6.0);
}

#[test]
fn airborne_jump_presses_do_not_add_upward_velocity() {
    let grid = Fixture::default();
    let mut controller = CharacterController::default();
    let first = controller.advance(
        Vec3::new(5.0, 8.0, 5.0),
        CharacterInput::default(),
        0.05,
        &grid,
    );
    let second = controller.advance(
        first.position,
        CharacterInput {
            jump: true,
            ..CharacterInput::default()
        },
        0.05,
        &grid,
    );
    assert!(second.velocity.y < first.velocity.y && second.position.y < first.position.y);
}

#[test]
fn high_speed_sweeps_cannot_tunnel_through_a_thin_wall_or_floor() {
    let mut grid = Fixture::default();
    grid.fill([7, 0, 0], [8, 60, 30]);
    grid.fill([0, 2, 0], [30, 3, 30]); // One plate at y=0.8..1.2.
    let config = CharacterConfig {
        max_speed: 500.0,
        ground_acceleration: 1000.0,
        air_acceleration: 1000.0,
        gravity: 1000.0,
        terminal_speed: 500.0,
        ..CharacterConfig::default()
    };
    let mut controller = CharacterController::new(config).unwrap();
    let mut p = Vec3::new(2.0, 20.0, 5.0);
    for _ in 0..5 {
        let motion = controller.advance(
            p,
            CharacterInput {
                direction: Vec2::X,
                speed: 500.0,
                jump: false,
            },
            0.1,
            &grid,
        );
        p = motion.position;
        assert!(p.x <= 7.0 - config.radius + 0.001, "wall tunneled: {p:?}");
        assert!(p.y >= 1.2 - 0.001, "floor tunneled: {p:?}");
        assert!(Grid::new(&grid).unwrap().clear(p, &config));
    }
    assert!((p.y - 1.2).abs() < 0.001);
}

#[test]
fn recovery_lifts_out_of_solid_terrain_and_does_not_snap_through_a_bridge() {
    let mut grid = Fixture::default();
    grid.fill([0, 0, 0], [20, 18, 20]);
    let mut controller = CharacterController::default();
    let recovered = controller.advance(
        Vec3::new(5.0, 3.0, 5.0),
        CharacterInput::default(),
        1.0 / 60.0,
        &grid,
    );
    assert!(recovered.recovered && recovered.grounded);
    assert!((recovered.position.y - 7.2).abs() < 0.001);
    grid.fill([0, 27, 0], [20, 28, 20]); // Walkable 3.6-unit space under a bridge.
    let under = controller.advance(
        recovered.position,
        CharacterInput::default(),
        1.0 / 60.0,
        &grid,
    );
    assert!(!under.recovered);
    assert!((under.position.y - 7.2).abs() < 0.001);
}

#[test]
fn rebasing_the_external_position_preserves_velocity() {
    let grid = Fixture::default();
    let mut controller = CharacterController::default();
    let before = drive(
        &mut controller,
        &grid,
        Vec3::new(14.0, 0.0, 5.0),
        moving(Vec2::X),
        40,
    );
    let after = controller.advance(
        before.position - Vec3::X * 16.0,
        moving(Vec2::X),
        1.0 / 60.0,
        &grid,
    );
    assert!(!after.recovered);
    assert!(after.velocity.x >= before.velocity.x);
    assert!((after.position.x + 16.0 - before.position.x - 7.0 / 60.0).abs() < 0.001);
}

#[test]
fn walking_off_a_ledge_falls_instead_of_instantly_snapping_to_ground() {
    let mut grid = Fixture::default();
    grid.fill([0, 0, 0], [5, 10, 20]);
    let mut controller = CharacterController::default();
    let mut p = Vec3::new(4.0, 4.0, 5.0);
    let mut falling = false;
    for _ in 0..90 {
        let motion = controller.advance(p, moving(Vec2::X), 1.0 / 60.0, &grid);
        if motion.position.x > 5.4 && motion.position.y > 0.5 && motion.position.y < 3.9 {
            falling = true;
            assert!(!motion.grounded && motion.velocity.y < 0.0);
        }
        p = motion.position;
    }
    assert!(falling && p.y < 0.001);
}

#[test]
fn circular_body_passes_clear_of_a_corner_that_a_square_would_clip() {
    let mut grid = Fixture::default();
    grid.fill([5, 0, 5], [6, 12, 6]);
    let mut controller = CharacterController::default();
    let p = Vec3::new(4.75, 0.0, 4.75); // Distance sqrt(0.25²+0.25²) > radius 0.32.
    let motion = controller.advance(p, CharacterInput::default(), 1.0 / 60.0, &grid);
    assert!(!motion.recovered);
    assert!((motion.position.x - p.x).abs() < 0.0001);
}

#[test]
fn a_diagonal_sweep_stops_at_the_round_corner_without_penetration() {
    let mut grid = Fixture::default();
    grid.fill([5, 0, 5], [6, 12, 6]);
    let mut controller = CharacterController::default();
    let motion = drive(
        &mut controller,
        &grid,
        Vec3::new(3.0, 0.0, 3.0),
        moving(Vec2::ONE),
        90,
    );
    let offset = 5.0 - controller.config().radius / 2.0_f32.sqrt();
    assert!((motion.position.x - offset).abs() < 0.001);
    assert!((motion.position.z - offset).abs() < 0.001);
    assert!(motion.velocity.length() < 0.001);
}

#[test]
fn two_wall_corner_blocks_both_slide_directions() {
    let mut grid = Fixture::default();
    grid.fill([5, 0, 0], [6, 12, 20]);
    grid.fill([0, 0, 5], [20, 12, 6]);
    let mut controller = CharacterController::default();
    let motion = drive(
        &mut controller,
        &grid,
        Vec3::new(3.0, 0.0, 3.0),
        moving(Vec2::ONE),
        90,
    );
    assert!((motion.position.x - 4.68).abs() < 0.001);
    assert!((motion.position.z - 4.68).abs() < 0.001);
}

#[test]
fn empty_space_and_positions_outside_a_window_have_a_bounded_bottom() {
    let grid = Fixture::default();
    let mut controller = CharacterController::default();
    let motion = drive(
        &mut controller,
        &grid,
        Vec3::new(-400.0, 16.0, 900.0),
        CharacterInput::default(),
        180,
    );
    assert!(motion.grounded && motion.position.y >= 0.0 && motion.position.y < 0.001);
    assert_eq!(motion.position.x, -400.0);
    assert_eq!(motion.position.z, 900.0);
    let recovered = controller.advance(
        Vec3::new(-400.0, -50.0, 900.0),
        CharacterInput::default(),
        1.0 / 60.0,
        &grid,
    );
    assert!(recovered.recovered && recovered.grounded);
    assert!(recovered.position.y >= 0.0);
}
