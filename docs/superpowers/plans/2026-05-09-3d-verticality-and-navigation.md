# 3D Elevation & 2.5D Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement visual 3D terrain height (verticality) while maintaining performant grid-based navigation that respects slopes.

**Architecture:**
1. **Vertical Mapping**: Translate `MapData.elevation` (0.0..1.0) into actual `y` coordinates in the world.
2. **Slope-Aware Costs**: Update `NavigationMap` during generation. If a neighbor is significantly higher, mark it as blocked or expensive.
3. **Adaptive Movement**: Update pathfinding and movement systems to query `MapData` for the correct `y` at every step.

**Tech Stack:** Bevy 0.18.1, `NavigationMap`, `MapData`.

---

### Task 1: Vertical Visuals & Data Sync

**Files:**
- Modify: `src/map/mod.rs` (Apply vertical offset during spawn)
- Modify: `src/map/navigation/types.rs` (Update `grid_to_world` to be context-aware)

- [ ] **Step 1: Scale elevation in `spawn_map`**
    - Define `const MAX_HEIGHT: f32 = 4.0;`
    - Apply `y = tile_data.elevation * MAX_HEIGHT` to `Transform.translation`.
    - Update `Roof` spawning to be `y + 1.0`.

- [ ] **Step 2: Update `grid_to_world` to use `MapData`**
    - Change signature: `fn grid_to_world(cell: IVec2, map: &MapData) -> Vec3`.
    - It should look up the elevation for the cell and return the correct `y + AGENT_HEIGHT`.

- [ ] **Step 3: Fix call sites for `grid_to_world`**
    - Update `algo.rs` and `mod.rs` to pass `MapData`.

### Task 2: Slope-Aware Navigation

**Files:**
- Modify: `src/map/mod.rs` (Update navigation costs based on neighbors)

- [ ] **Step 1: Implement slope logic in `spawn_map`**
    - After initial elevation is set, run a second pass.
    - Compare `elevation` of cell (x, z) with neighbors (x+1, z, etc.).
    - `let slope = (neighbor.elevation - current.elevation).abs();`
    - If `slope > 0.3`, set `NavigationMap` cost to `COST_BLOCKER` or `150` (very expensive).

### Task 2.5: Map Scaling & Polish

**Files:**
- Modify: `src/map/mod.rs` (Increase map size for better height visualization)

- [ ] **Step 1: Increase map size to 40x40**
    - Change `width/height` to 40.
    - Adjust Voronoi and Fbm frequencies for better aesthetics on larger map.

### Task 3: Adaptive Follow-Path

**Files:**
- Modify: `src/map/navigation/systems.rs` (Make pawns follow height)

- [ ] **Step 1: Update `follow_path` to query height**
    - Instead of just interpolating, at each frame query `MapData` for the current tile's elevation.
    - Set `transform.translation.y = current_tile_elevation * MAX_HEIGHT + AGENT_HEIGHT`.
    - This ensures smooth movement up and down hills.

---

## Self-Review
1. **Spec coverage:** Visual 3D heights, slope-aware navigation, and adaptive movement are all included.
2. **Placeholder scan:** No TBDs. Clear scaling constants provided.
3. **Type consistency:** Passing `MapData` to navigation functions ensures they stay in sync.
