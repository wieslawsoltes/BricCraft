# Construction mechanics

This crate has no UI, renderer, browser, or operating-system dependencies. The geometry uses studs horizontally and plates vertically (`1 plate = 0.4 studs`). Angles use radians; time uses seconds.

## Assemblies and drive behavior

`assess_assembly` inspects the actual placed parts. It counts motors, power sources, wheels and wings; estimates ABS shell mass; measures wheel radius, wheelbase and track width; and checks the number of connected contact groups. A ready car needs a connected powered motor and four wheels. A train needs powered motors and train wheels. A plane needs powered motors and two wings. A boat may use a sail.

Call `Vehicle::configure(&assessment)` after constructing a vehicle. Cars use the motor torque/speed curve, automatic gearbox, final reduction, wheel radius and estimated mass to calculate acceleration. Their steering and suspension use the measured wheelbase and track. Plane, train and boat acceleration also responds to assembly mass and available motor count. The motor constants are tuned for stud-scaled play; they are not calibrated measurements of a particular physical motor.

Wheel contact with a floor does not join a vehicle to the ground. Horizontal wheel contact with an axle or chassis remains a structural connection. Placement collision uses conservative part envelopes: inserting an axle through an occupied beam hole is not yet a supported editor operation.

The assembly recognizer infers one external gear reduction only when gear pitch circles meet and their axes align. A disconnected or decorative gear does not change vehicle behavior. General transmissions use explicit `GearTrain` shaft connections; a caller supplies which gears mesh and which shaft is driven.

## Reusable engineering primitives

`GearTrain::solve` propagates signed angular velocity and available output torque through external gears, internal gears, belts or chains. It rejects contradictory closed loops, including an odd external-gear loop at rest. Efficiency decreases available power. Branches report available torque independently; a load-sharing multi-body solver is not implied.

`differential` maintains the ideal open differential identity `ω_left + ω_right = 2ω_carrier`. `ackermann_steering` computes inner and outer wheel angles. `rack_displacement` converts pinion rotation into linear travel. `Motor` has stall torque and free-speed behavior. `SpringDamper` computes compression force. `Joint` integrates a generalized hinge, axle or slider coordinate with limits and damping.

Vehicles use bounded 120 Hz substeps. The car follows sampled terrain and stops at steep vertical obstacles. The plane has speed-dependent lift, pitch and bank controls. The train follows a distance-parameterized closed path. The boat has damped buoyancy and grounding checks. These are playable rigid construction models, not an articulated rigid-body solver: individual rods, joints and gears are not yet independently animated or collision-solved while driving.

The generated coastal track and the train use the same rounded centerline in `briccraft-world`, with a seven-stud gauge and six-stud curve radius. Its tangent is continuous through every straight/curve boundary. The game keeps a moving assembly's rotated footprint within the finite world and parks it using an atomic, identity-preserving history transform. Physics uses the absolute heading; rendering uses the difference from the construction's initial orientation, so a rotated blueprint does not rotate twice when activated or parked.

## Validation

The tests verify reduction direction and power, locked loops, differential and steering identities, rail wraparound, frame-rate independence, brakes, takeoff, buoyancy, powered connected blueprints, disconnected power rejection, and changes in driving behavior when mass or gear reduction changes.
