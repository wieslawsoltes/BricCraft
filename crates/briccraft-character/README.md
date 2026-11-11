# briccraft-character

A reusable kinematic walking controller over an axis-aligned voxel volume. It depends
on `glam`; it has no dependency on FerroUI, rendering, the game session, or the part catalog.

## Integration

Implement `CollisionGrid` with the volume's dimensions, physical cell size, and occupancy
query. The controller queries only valid cell coordinates. Pass the **centre of the feet**
to `CharacterController::advance`, a world-space X/Z movement direction, movement speed,
the held jump state, and the simulation step. The returned `CharacterMotion` contains the
resolved position, velocity, ground contact, and whether a step, ceiling hit, or recovery
occurred.

The caller owns position. Moving the local position and voxel window together during a
streaming rebase preserves velocity and jump state. The controller stores no global
position and no references to the world. `reset` clears momentum when deliberately
switching movement modes.

`CameraRig::tick_world` supplies BricCraft's occupancy adapter with cells measuring
`[1.0, 0.4, 1.0]`. Camera position remains the eye position; its serialized fields are
unchanged and the controller is runtime state. Fly and Orbit use their independent camera
movement. Panning in Walk cannot translate the body; zoom changes the field of view.

## Movement and contact

- An upright cylinder has a 0.32-stud radius, 2.5-unit height, and 2.2-unit eye height by
  default. The circular footprint can pass a corner that an equally wide square would hit.
- Horizontal sweeps analytically intersect the expanded faces and rounded corners of
  occupied cell columns. Up to four contact-plane projections slide along walls and stop
  at inside corners. Movement direction is length-limited, so diagonals have no speed bonus.
- Continuous vertical sweeps stop the head at ceilings and feet on floors, including
  single-plate floors during a fast fall. Gravity, terminal speed, and grounded jump
  impulse are configurable. Jump uses a press edge, preventing repeated jumps from holding
  the key through landing; presses in mid-air do not add an impulse.
- A step may rise at most 0.8 units, or two BricCraft plates. It must have a clear upward
  path, make more forward progress than ordinary sliding, and find support on its downward
  sweep. Limited ceiling clearance can permit a shorter step while blocking a taller one.
- Input acceleration and gravity integrate in steps no longer than 1/120 second. One call
  consumes at most 100 ms, with at most twelve integration steps. Sweeps protect translations
  between steps; this limit keeps a stalled host from doing unbounded catch-up work.

## Recovery and bounds

A valid position below a bridge or roof stays there; the controller never replaces collision
with a highest-column surface lookup. If a flight camera, loaded position, or new construction
leaves the body inside solids, recovery scans upward to the first clear cell-top. The search
is bounded by the volume's height and retains X/Z.

Y=0 is a bottom safety plane, including in empty cells and outside the active X/Z window.
Outside X/Z and above the volume are otherwise open, so streaming adds no invisible side
walls. In BricCraft this plane is below the water surface: water does not become a solid
voxel, and this controller does not implement swimming or buoyancy.

Collision is deliberately based on occupancy. A part's slope, wheel bore, window opening,
or other visual cutout inside occupied cells remains solid. This is a character controller,
not a rigid body or articulated-vehicle solver.

## Verification

Run `cargo test -p briccraft-character` for fixtures covering walls, diagonal wall sliding,
convex and concave corners, normalized movement, short steps, low ceilings, jumps, fast
wall/floor sweeps, bridges, embedded-terrain recovery, ledge falls, empty-space bounds, and
stream-origin rebasing. Camera integration and saved-schema checks live in
`briccraft-game::camera::tests`.
