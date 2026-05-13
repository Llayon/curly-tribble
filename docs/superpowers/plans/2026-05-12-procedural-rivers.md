# Procedural River Pathfinding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement natural-looking procedural rivers using Dijkstra's algorithm to navigate terraced terrain.

**Architecture:** 
- Update `TerrainConfig` with river settings.
- Implement `apply_rivers` using Dijkstra's algorithm (Uniform Cost Search).
- Add a "Mud Bank" post-processing pass.
- Integrate into the `spawn_map_internal` pipeline.

**Tech Stack:** Rust, Bevy 0.18, `rand` crate.

---

### Task 1: Update TerrainConfig for Rivers

**Files:**
- Modify: `src/map/terrain_gen.rs`

- [ ] **Step 1: Add river fields to TerrainConfig**

```rust
pub struct TerrainConfig {
    // ... existing fields ...
    #[inspector(min = 0, max = 30)]
    pub river_count: u32,
    #[inspector(min = 0.3, max = 0.9)]
    pub river_start_elevation: f32,
    pub generate_mud_banks: bool,
    // ... 
}
```

- [ ] **Step 2: Update Default implementation**

Set `river_count: 5`, `river_start_elevation: 0.6`, `generate_mud_banks: true`.

- [ ] **Step 3: Commit**

```bash
git add src/map/terrain_gen.rs
git commit -m "feat: add river settings to TerrainConfig"
```

---

### Task 2: Implement River Pathfinding (Dijkstra)

**Files:**
- Create: `src/map/river_gen.rs`
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Implement Dijkstra-based river routing**

```rust
// src/map/river_gen.rs
use bevy::prelude::*;
use rand::prelude::*;
use std::collections::{BinaryHeap, HashMap};
use crate::map::zoning::{MapData, TerrainType};
use crate::map::terrain_gen::TerrainConfig;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    pos: IVec2,
    cost: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.cmp(&self.cost) // Min-heap
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn apply_rivers(map_data: &mut MapData, config: &TerrainConfig, seed: u32) {
    let mut rng = StdRng::seed_from_u64(seed as u64 + 500);
    let half_w = (map_data.width / 2) as i32;
    let half_h = (map_data.height / 2) as i32;

    for _ in 0..config.river_count {
        // 1. Find source
        let mut source = None;
        for _ in 0..100 {
            let x = rng.gen_range(-half_w..half_w);
            let z = rng.gen_range(-half_h..half_h);
            if let Some(tile) = map_data.get_tile(x, z) {
                if tile.elevation > config.river_start_elevation {
                    source = Some(IVec2::new(x, z));
                    break;
                }
            }
        }

        let Some(start_pos) = source else { continue; };

        // 2. Dijkstra
        let mut pq = BinaryHeap::new();
        let mut came_from = HashMap::new();
        let mut cost_so_far = HashMap::new();

        pq.push(Node { pos: start_pos, cost: 0 });
        cost_so_far.insert(start_pos, 0);

        let mut target_pos = None;

        while let Some(Node { pos, cost }) = pq.pop() {
            let current_tile = map_data.get_tile(pos.x, pos.y).unwrap();
            
            // Check termination
            if current_tile.elevation < 0.2 || current_tile.terrain == TerrainType::Water || 
               pos.x <= -half_w || pos.x >= half_w - 1 || pos.y <= -half_h || pos.y >= half_h - 1 {
                target_pos = Some(pos);
                break;
            }

            for neighbor in [
                IVec2::new(pos.x + 1, pos.y), IVec2::new(pos.x - 1, pos.y),
                IVec2::new(pos.x, pos.y + 1), IVec2::new(pos.x, pos.y - 1),
            ] {
                if let Some(n_tile) = map_data.get_tile(neighbor.x, neighbor.y) {
                    let step_cost = if n_tile.elevation < current_tile.elevation {
                        1
                    } else if (n_tile.elevation - current_tile.elevation).abs() < 0.001 {
                        5
                    } else {
                        1000 // Uphill
                    };

                    let new_cost = cost + step_cost;
                    if !cost_so_far.contains_key(&neighbor) || new_cost < *cost_so_far.get(&neighbor).unwrap() {
                        cost_so_far.insert(neighbor, new_cost);
                        came_from.insert(neighbor, pos);
                        pq.push(Node { pos: neighbor, cost: new_cost });
                    }
                }
            }
        }

        // 3. Backtrack and mark water
        if let Some(mut curr) = target_pos {
            while let Some(&prev) = came_from.get(&curr) {
                if let Some(tile) = map_data.get_tile_mut(curr.x, curr.y) {
                    tile.terrain = TerrainType::Water;
                }
                curr = prev;
            }
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/map/river_gen.rs
git commit -m "feat: implement Dijkstra-based river pathfinding"
```

---

### Task 3: Mud Banks and Integration

**Files:**
- Modify: `src/map/river_gen.rs`
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Implement mud bank pass**

```rust
// src/map/river_gen.rs
pub fn apply_mud_banks(map_data: &mut MapData) {
    let half_w = (map_data.width / 2) as i32;
    let half_h = (map_data.height / 2) as i32;
    let mut mud_to_add = Vec::new();

    for x in -half_w..half_w {
        for z in -half_h..half_h {
            if let Some(tile) = map_data.get_tile(x, z) {
                if tile.terrain == TerrainType::Water {
                    for dx in -1..=1 {
                        for dz in -1..=1 {
                            if dx == 0 && dz == 0 { continue; }
                            let nx = x + dx;
                            let nz = z + dz;
                            if let Some(n_tile) = map_data.get_tile(nx, nz) {
                                if matches!(n_tile.terrain, TerrainType::Grass | TerrainType::Sand | TerrainType::Stone) {
                                    mud_to_add.push(IVec2::new(nx, nz));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for pos in mud_to_add {
        if let Some(tile) = map_data.get_tile_mut(pos.x, pos.y) {
            tile.terrain = TerrainType::Mud;
        }
    }
}
```

- [ ] **Step 2: Integrate into spawn_map_internal**

Call `apply_rivers` and `apply_mud_banks` in `src/map/mod.rs` after biome generation but before cave stamping.

- [ ] **Step 3: Commit**

```bash
git add src/map/mod.rs src/map/river_gen.rs
git commit -m "feat: integrate rivers and mud banks into map pipeline"
```

---

### Task 4: Verification

- [ ] **Step 1: Write integration test**

Create `tests/river_gen_test.rs` to verify rivers are spawned and flow correctly.

- [ ] **Step 2: Run and Tuning**

Launch the game and verify rivers visually.

- [ ] **Step 3: Final Commit**

```bash
git add tests/river_gen_test.rs
git commit -m "test: verify procedural river generation"
```
