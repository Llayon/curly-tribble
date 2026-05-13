# SOP: A River Routing (Pathfinding-based)

## Overview
Classic gradient descent (flow based on local neighbor height) fails on perfectly flat terrain like terraced plateaus. To solve this, we use **Dijkstra's Algorithm** (Uniform Cost Search) to find the shortest path from mountains to oceans/edges.

## Core Rules
1. **Source Selection**: Randomly pick a tile with `elevation > threshold`.
2. **Pathfinding Weights**:
    - **Downhill**: Cost = 1 (High priority).
    - **Flat**: Cost = 5 (Shortest path across plateau to the next cliff).
    - **Uphill**: Cost = 1000 (Hard blocker, water doesn't flow up).
3. **Termination Criteria**:
    - Edge of map.
    - Elevation < 0.2 (Ocean/Lake level).
    - Existing `TerrainType::Water` (River merging).
4. **Post-Processing**: Apply **Mud Banks** (1-tile radius of Mud around Water segments).

## Implementation Details (Bevy 0.18.1)
- **Module**: `src/map/river_gen.rs`.
- **Plugin**: `RiverGenPlugin` (Marker).
- **Trigger**: Called in `spawn_map_internal` after climate-based biome generation but before 3D mesh spawning.
- **Safety**: No `.unwrap()` or `.expect()` in routing loops. Use `if let` or `match`.

## Aesthetics
- **Mud Banks**: Surround water with `TerrainType::Mud` if neighbors are Grass, Sand, or Stone.
- **Warping**: Domain warping in height noise ensures rivers aren't straight lines on the terrain.
