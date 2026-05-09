# Procedural Generation Overhaul Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a hybrid procedural generation system with Voronoi mountain ranges, Whittaker biomes, and seamless cave exploration.

**Architecture:** A multi-layered generation pipeline: Macro (Voronoi) -> Climate (Humidity/Temp) -> Micro (Fbm Noise) -> Scattering. Uses a "Stamp" system for caves with dithered transparency.

**Tech Stack:** Bevy 0.18.1, `noise-rs`, `rand`.

---

### Task 1: Refactor Map Data Structures

**Files:**
- Modify: `src/map/zoning.rs` (Define `TileData`, update `TerrainType`)
- Modify: `src/map/mod.rs` (Add `MapData` resource)

- [ ] **Step 1: Update `src/map/zoning.rs` with new terrain types and TileData**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainType {
    #[default]
    Grass,
    Mud,
    Water,
    Sand,
    Stone,
    CaveFloor,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TileData {
    pub terrain: TerrainType,
    pub elevation: f32,
    pub humidity: f32,
    pub temperature: f32,
    pub is_roofed: bool,
}
```

- [ ] **Step 2: Add `MapData` resource to `src/map/mod.rs`**

```rust
#[derive(Resource, Default)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<crate::map::zoning::TileData>,
}

impl MapData {
    pub fn get_tile(&self, x: i32, z: i32) -> Option<&crate::map::zoning::TileData> {
        let ux = (x + (self.width as i32 / 2)) as u32;
        let uz = (z + (self.height as i32 / 2)) as u32;
        if ux < self.width && uz < self.height {
            Some(&self.tiles[(uz * self.width + ux) as usize])
        } else {
            None
        }
    }
}
```

- [ ] **Step 3: Run `cargo check` to verify types**

### Task 2: Implement Voronoi Skeleton (The Bones)

**Files:**
- Modify: `src/map/mod.rs` (Implement Voronoi-based elevation)

- [ ] **Step 1: Implement a simple Voronoi generator for mountain ranges**

```rust
fn generate_voronoi_heights(width: u32, height: u32, seed: u32) -> Vec<f32> {
    let mut rng = rand::prelude::StdRng::seed_from_u64(seed as u64);
    let points: Vec<Vec2> = (0..10).map(|_| Vec2::new(rng.gen_range(0.0..width as f32), rng.gen_range(0.0..height as f32))).collect();
    
    (0..width * height).map(|i| {
        let x = (i % width) as f32;
        let z = (i / width) as f32;
        let mut min_dist = f32::MAX;
        for p in &points {
            min_dist = min_dist.min(p.distance(Vec2::new(x, z)));
        }
        min_dist / (width as f32 / 2.0) // Normalized elevation
    }).collect()
}
```

- [ ] **Step 2: Update `spawn_map` to use Voronoi for elevation**

### Task 3: Implement Whittaker Biomes (The Climate)

**Files:**
- Modify: `src/map/mod.rs` (Apply humidity/temp noise and mapping)

- [ ] **Step 1: Implement Whittaker mapping function**

```rust
fn get_terrain_from_climate(temp: f32, humid: f32, elev: f32) -> TerrainType {
    if elev < 0.2 { return TerrainType::Water; }
    if elev < 0.25 { return TerrainType::Sand; }
    if elev > 0.8 { return TerrainType::Stone; }
    
    if humid > 0.7 {
        if temp < 0.3 { TerrainType::Mud } else { TerrainType::Grass }
    } else if humid < 0.3 {
        if temp > 0.7 { TerrainType::Sand } else { TerrainType::Grass }
    } else {
        TerrainType::Grass
    }
}
```

### Task 4: Cave Stamp System & Seamless Cutaway

**Files:**
- Modify: `src/map/mod.rs` (Stamp injection)
- Create: `src/map/visibility.rs` (Dithered transparency/Visibility toggle)

- [ ] **Step 1: Implement basic Stamp injection**

```rust
fn apply_cave_stamp(map: &mut MapData, x: i32, z: i32) {
    // Simple 3x3 cave room for now
    for dx in -1..=1 {
        for dz in -1..=1 {
            if let Some(tile) = map.get_tile_mut(x + dx, z + dz) {
                tile.terrain = TerrainType::CaveFloor;
                tile.is_roofed = true;
            }
        }
    }
}
```

- [ ] **Step 2: Implement visibility system using `is_roofed`**
    - Add a system that queries `MapData` and `Pawn` positions.
    - If a pawn is on a `is_roofed` tile, set `Visibility::Hidden` for all entities in that zone marked as `Roof`.

### Task 5: Resource Scattering (Smart Placement)

**Files:**
- Modify: `src/map/resources.rs` (Update scattering rules)

- [ ] **Step 1: Update `spawn_resources` to use `MapData`**
    - Trees only on `TerrainType::Grass`.
    - Rocks near `TerrainType::Stone`.
    - Berry bushes in `TerrainType::Mud` (Swamps).

---

## Self-Review
1. **Spec coverage:** Voronoi, Whittaker, Stamps, and Seamless visibility are all covered.
2. **Placeholder scan:** No TBDs. Actual code snippets provided for logic.
3. **Type consistency:** `TerrainType` and `MapData` definitions are consistent across tasks.
