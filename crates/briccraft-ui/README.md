# Native game interface

The interface is composed entirely with declarative FerroUI builders, signals and
native controls. World rendering and browser services remain outside this crate.

## Player-facing layout

The desktop HUD uses floating navigation groups, a compact world identity and a
small resource/objective chip. A vehicle action appears only for a ready or active
construction. Full mechanical details remain in the Workshop. The complete latest
activity message and world information open from the HUD feedback control.

Phone layouts retain separate movement and construction controls, safe-area and
keyboard handling, and 44–48 pixel minimum action targets. The vertical controls
become Jump while walking and are hidden for vehicles without airborne controls.
Landscape keeps the existing world gesture area and compact hotbar.

Settings expose independent resolution policy, antialiasing and surface-detail
choices. Actual render status appears there. Performance counters are an opt-in
overlay, so normal gameplay does not expose renderer implementation details.

## Reusable construction library

Construction plans contain Starter plans and My designs. Player designs support
capture, naming, placement, export and import. Deletion requires inline confirmation
and explains that previously placed constructions remain. Prices and affordability
come from the session, as do capture readiness and all validation messages.

The UI projects SavedDesignCard and DesignLibraryView without depending on the
design or game crates. UiAction routes every operation to the same host dispatcher
as keyboard and public commands.

## Presentation contracts

- Resolution mode: 0 Auto, 1 Quality, 2 Performance.
- Antialiasing: requested sample count 1, 2 or 4.
- Blueprint section: 0 Starter plans, 1 My designs.
- Vehicle kind: car, plane, train, boat or an empty string.
- World-details and performance-overlay state are local view preferences.
- Every modal is included in UiState's close and input-isolation helpers.

All interface illustrations are native geometry. Icon and construction drawings
scale with the device; there are no raster UI assets, DOM widgets, or painted
replacement controls.
