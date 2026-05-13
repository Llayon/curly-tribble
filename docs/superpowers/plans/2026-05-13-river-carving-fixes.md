# River Carving and Geometry Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform rivers into physical channels by carving them into the terrain and ensuring the water mesh follows the local elevation. Fix geometric artifacts by disabling caves.

**Architecture:**
- **UI**: Add `river_depth` to `TerrainConfig`.
- **Logic**: Implement "Downhill" constraints and height subtraction during river routing.
- **Smoothing**: Implement linear interpolation for `Mud` bank heights.
- **Rendering**: Refactor `create_global_map_meshes` to generate a dynamic water mesh from the tile grid instead of a flat plane.

**Tech Stack:** Rust, Bevy 0.18.

---

### Task 1: UI Update & Cave Cleanup

**Files:**
- Modify: `src/map/terrain_gen.rs`
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Add river_depth to TerrainConfig**

```rust
// src/map/terrain_gen.rs
pub struct TerrainConfig {
    // ...
    #[inspector(min = 0.0, max = 0.2)]
    pub river_depth: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            // ...
            river_depth: 0.05,
        }
    }
}
```

- [ ] **Step 2: Disable cave generation in mod.rs**

Temporarily comment out `apply_cave_stamp` call inside `spawn_map_internal`.

- [ ] **Step 3: Commit**

```bash
git add src/map/terrain_gen.rs src/map/mod.rs
git commit -m "chore: add river_depth config and temporarily disable caves"
```

---

### Task 2: Implement River Carving Logic

**Files:**
- Modify: `src/map/river_gen.rs`

- [ ] **Step 1: Update apply_rivers with carving and downhill constraints**

Ensure that during backtracking, each tile's elevation is lowered and forced to be non-increasing.

```rust
// In src/map/river_gen.rs -> apply_rivers
// ... backtracking loop ...
if let Some(mut curr) = target_pos {
    let mut prev_elev = 1.0; // Start high
    while let Some(&prev) = came_from.get(&curr) {
        if let Some(tile) = map_data.get_tile_mut(curr.x, curr.y) {
            tile.terrain = TerrainType::Water;
            // Carve: Lower elevation and ensure it never goes up
            tile.elevation = (tile.elevation - config.river_depth).min(prev_elev).max(0.0);
            prev_elev = tile.elevation;
        }
        curr = prev;
    }
}
```

- [ ] **Step 2: Update apply_mud_banks with height smoothing**

For each mud tile, set its height to the average of its water and land neighbors.

- [ ] **Step 3: Commit**

```bash
git add src/map/river_gen.rs
git commit -m "feat: implement river carving and bank smoothing"
```

---

### Task 3: Refactor Global Water Mesh

**Files:**
- Modify: `src/economy/mesh_gen.rs`

- [ ] **Step 1: Rewrite water mesh generation to follow grid**

Instead of a `Plane3d`, build the water mesh using the same grid logic as the terrain, but only including faces where `tile.terrain == Water`. Use local `elevation`.

- [ ] **Step 2: Commit**

```bash
git add src/economy/mesh_gen.rs
git commit -m "refactor: dynamic water mesh that respects local elevation"
```

---

### Task 4: Verification

- [ ] **Step 1: Run and Verify**

1. Launch game.
2. Regenerate world.
3. Check rivers in mountains and plateaus.
4. Verify mud banks create a smooth ramp.

- [ ] **Step 2: Final Commit**

```bash
git add .
git commit -m "chore: finalize river carving and geometry fixes"
```
