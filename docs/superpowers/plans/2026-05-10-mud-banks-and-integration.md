# Mud Banks and Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement mud bank generation and integrate river/mud generation into the main map generation pipeline.

**Architecture:** Add a new pass `apply_mud_banks` to `river_gen.rs` that identifies tiles adjacent to water and turns them into mud. Integrate both `apply_rivers` and `apply_mud_banks` into `spawn_map_internal` in `mod.rs`.

**Tech Stack:** Bevy (IVec2, MapData, TerrainType), Rust.

---

### Task 1: Implement Mud Bank Pass

**Files:**
- Modify: `src/map/river_gen.rs`
- Test: `tests/mud_banks_test.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::zoning::{MapData, TerrainType, TileData};

    #[test]
    fn test_apply_mud_banks() {
        let mut map_data = MapData {
            width: 4,
            height: 4,
            tiles: vec![TileData::default(); 16],
        };
        // Set up a 4x4 map with water in the middle
        // (x, z) ranges from -2 to 1
        // (0,0) is water, neighbors should become mud
        if let Some(tile) = map_data.get_tile_mut(0, 0) {
            tile.terrain = TerrainType::Water;
        }
        if let Some(tile) = map_data.get_tile_mut(1, 1) {
            tile.terrain = TerrainType::Grass;
        }

        apply_mud_banks(&mut map_data);

        let tile = map_data.get_tile(1, 1).unwrap();
        assert_eq!(tile.terrain, TerrainType::Mud, "Grass neighbor of water should be mud");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test mud_banks_test`
Expected: FAIL (apply_mud_banks not defined)

- [ ] **Step 3: Implement `apply_mud_banks` in `src/map/river_gen.rs`**

```rust
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

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test mud_banks_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/map/river_gen.rs
git commit -m "feat: implement mud bank pass"
```

### Task 2: Integrate into spawn_map_internal

**Files:**
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Locate `spawn_map_internal` and add integration calls**

Add `river_gen::apply_rivers` and `river_gen::apply_mud_banks` after biome generation and before cave stamping.

```rust
    // ... after biome generation loop ...
    
    // Apply Rivers and Mud Banks
    river_gen::apply_rivers(map_data, terrain_config, seed.value());
    river_gen::apply_mud_banks(map_data);

    // ... before cave stamping loop ...
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/map/mod.rs
git commit -m "feat: integrate rivers and mud banks into map pipeline"
```
