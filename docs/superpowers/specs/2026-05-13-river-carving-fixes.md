# Spec: River Carving and Geometry Fixes

- **Status**: Draft
- **Date**: 2026-05-13
- **Topic**: Physical River Channels & Mesh Stability
- **References**: Pathfinding Rivers design

## 1. Vision & Goals
Transform rivers from "flat paint" into physical channels that carve into the terrain. Resolve geometric glitches caused by cave generation and ensure water meshes respect local elevation.

## 2. Technical Architecture

### UI Integration (TerrainConfig)
Add the following fields:
- `river_depth`: `f32` (Default: 0.05, Limits: 0.0..0.2) - How deep to carve the riverbed.

### River Carving Algorithm
1. **Vertical Constraint**: During pathfinding backtracking, ensure each tile's elevation is `min(previous_tile_elev, current_tile_elev)`. Water must never flow uphill.
2. **Bed Carving**: For each tile in the river path:
   `tile.elevation -= config.river_depth`.
3. **Slope Smoothing (Mud Banks)**: 
   During the mud bank pass, for each `Mud` tile:
   `mud_elev = (avg_water_neighbor_elev + avg_land_neighbor_elev) / 2.0`.
   This creates a smooth ramp down to the water.

### Geometry Fixes
- **Caves**: Temporarily disable `apply_cave_stamp` in `src/map/mod.rs` to fix "black hole" artifacts.
- **Mesh Generation**: Update `SpawnGlobalTerrainCommand` in `src/economy/mesh_gen.rs`. Remove any logic that hardcodes water Y-position to 0.0 or sea-level. Water vertices must use the `elevation` stored in `MapData`.

## 3. Integration Flow
1. Biome Gen.
2. **Apply Rivers + Carving** (New logic).
3. **Apply Mud Banks + Smoothing** (New logic).
4. Logic Tile Spawn.
5. Global Mesh Creation (Updated to respect elevation).

## 4. Verification
- **Visual**: Rivers should sit in a "U" or "V" shaped ditch.
- **Visual**: Banks (Mud) should look like ramps, not steps.
- **Shadows**: Disabling caves should restore clean shadow maps across mountains.
