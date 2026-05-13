# Spec: Procedural Pathfinding Rivers

- **Status**: Draft
- **Date**: 2026-05-12
- **Topic**: A* River Routing for Terraced Terrain
- **References**: Pathfinding Rivers (A River Routing)

## 1. Vision & Goals
Create natural-looking river systems that navigate the world's terraced plateaus. Since standard gradient descent fails on flat surfaces, we use A* pathfinding to "find" the shortest path to the next cliff or the ocean.

## 2. Technical Architecture

### UI Integration (TerrainConfig)
Add the following fields to `TerrainConfig`:
- `river_count`: `u32` (Default: 5, Max: 30) - Number of river attempts.
- `river_start_elevation`: `f32` (Default: 0.6) - Minimum height for a source.
- `generate_mud_banks`: `bool` (Default: true) - Toggle for surrounding mud tiles.

### River Routing Algorithm (Pathfinding)
- **Source Selection**: Random tile with `elevation > river_start_elevation`.
- **Target Criteria**: 
  - Edge of the map.
  - Tile with `elevation < 0.2` (Sea/Lake).
  - Existing `TerrainType::Water` tile (Merging).
- **A* Pathfinding Weights**:
  - **Downhill** (neighbor < current): `1.0` (Priority).
  - **Flat** (neighbor == current): `5.0` (Shortest path across plateau).
  - **Uphill** (neighbor > current): `1000.0` (Blocker).
- **Output**: Set `MapData` tiles to `TerrainType::Water`.

### Mud Bank Pass
Post-routing proccess:
1. Iterate over all grid tiles.
2. If tile is `Water`, check 8 neighbors.
3. If neighbor is `Grass`, `Sand`, or `Stone`, change to `Mud`.

### Integration Flow
1. Heights & Base Biomes Generated.
2. **apply_rivers** (Our new step).
3. Cave Stamp.
4. Logic Tile Spawn.
5. Global Mesh Creation.

## 3. Configuration & Tuning
- Optimization: Use a simple Dijkstra or BFS if purely looking for "downwards" without complex heuristics, but A* with a "distance to sea" heuristic might be faster for long plateaus.

## 4. Verification
- **Visual**: Rivers should flow from mountains, cut through plateaus at the shortest points, and merge into oceans or each other.
- **Logic**: No rivers should flow uphill.
- **Banks**: Mud should appear as a 1-tile border around all river segments.
