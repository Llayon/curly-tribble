# Advanced Terrain Generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a sophisticated procedural terrain generator for Bevy using the `noise` crate, featuring ridged mountains with passes and terraced plateaus.

**Architecture:** A `TerrainGenerator` Bevy Resource will wrap multiple noise instances (`Fbm`, `Perlin`). It will be initialized at startup and accessed via `Res<TerrainGenerator>`.

**Tech Stack:** Rust, Bevy 0.18, `noise` crate.

---

### Task 1: Create TerrainGenerator Resource

**Files:**
- Create: `src/map/terrain_gen.rs`

- [ ] **Step 1: Define the TerrainGenerator struct and constants**

```rust
// src/map/terrain_gen.rs
use bevy::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

pub const MACRO_FREQ: f64 = 0.03;
pub const PASS_FREQ: f64 = 0.01;
pub const MACRO_HEIGHT: f32 = 12.0;
pub const MACRO_SHARPNESS: f32 = 4.0;

pub const PLATEAU_FREQ: f64 = 0.04;
pub const PLATEAU_HEIGHT: f32 = 4.0;
pub const PLATEAU_STEPS: f32 = 4.0;
pub const WARP_FREQ: f64 = 0.02;
pub const WARP_STRENGTH: f32 = 5.0;

#[derive(Resource)]
pub struct TerrainGenerator {
    macro_noise: Fbm<Perlin>,
    pass_noise: Perlin,
    plateau_noise: Fbm<Perlin>,
    warp_noise_x: Perlin,
    warp_noise_z: Perlin,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            macro_noise: Fbm::<Perlin>::new(seed),
            pass_noise: Perlin::new(seed + 1),
            plateau_noise: Fbm::<Perlin>::new(seed + 2),
            warp_noise_x: Perlin::new(seed + 3),
            warp_noise_z: Perlin::new(seed + 4),
        }
    }

    pub fn get_elevation(&self, x: f32, z: f32) -> f32 {
        let x64 = x as f64;
        let z64 = z as f64;

        // 1. MACRO: Ridged Mountains with Pass Mask
        let ridge_val = self.macro_noise.get([x64 * MACRO_FREQ, z64 * MACRO_FREQ]);
        let ridge = (1.0 - ridge_val.abs()).powf(MACRO_SHARPNESS as f64) as f32;

        let pass_val = self.pass_noise.get([x64 * PASS_FREQ, z64 * PASS_FREQ]);
        let pass_mask = ((pass_val + 1.0) * 0.5) as f32;
        
        let mountains = ridge * pass_mask * MACRO_HEIGHT;

        // 2. MICRO: Domain Warped Terraced Plateaus
        let wx = self.warp_noise_x.get([x64 * WARP_FREQ, z64 * WARP_FREQ]) as f32 * WARP_STRENGTH;
        let wz = self.warp_noise_z.get([x64 * WARP_FREQ + 100.0, z64 * WARP_FREQ + 100.0]) as f32 * WARP_STRENGTH;
        
        let plateau_val = self.plateau_noise.get([(x64 + wx as f64) * PLATEAU_FREQ, (z64 + wz as f64) * PLATEAU_FREQ]);
        let plateau_base = ((plateau_val + 1.0) * 0.5) as f32;

        let plateaus = self.smoothstep_terracing(plateau_base, PLATEAU_STEPS) * PLATEAU_HEIGHT;

        // 3. BLENDING: Max() for predictable range
        mountains.max(plateaus)
    }

    fn smoothstep_terracing(&self, val: f32, steps: f32) -> f32 {
        let scaled = val * steps;
        let floor_val = scaled.floor();
        let fract_val = scaled - floor_val;
        
        // Cubic Hermite Interpolation (Smoothstep) for passable slopes
        let smoothed_fract = fract_val * fract_val * (3.0 - 2.0 * fract_val);
        (floor_val + smoothed_fract) / steps
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/map/terrain_gen.rs
git commit -m "feat: add TerrainGenerator resource with ridged noise and terracing"
```

---

### Task 2: Register Resource and Update Config

**Files:**
- Modify: `src/map/mod.rs`
- Modify: `src/map/zoning.rs`

- [ ] **Step 1: Update MAX_HEIGHT in zoning.rs**

```rust
// src/map/zoning.rs
pub const MAX_HEIGHT: f32 = 12.0; // Increased to match MACRO_HEIGHT
```

- [ ] **Step 2: Initialize TerrainGenerator in MapPlugin**

```rust
// src/map/mod.rs
use terrain_gen::TerrainGenerator;

// Inside build method
let seed_val = 42; // or get from WorldSeed if already initialized
app.insert_resource(TerrainGenerator::new(seed_val));
```

- [ ] **Step 3: Commit**

```bash
git add src/map/mod.rs src/map/zoning.rs
git commit -m "refactor: register TerrainGenerator resource and update MAX_HEIGHT"
```

---

### Task 3: Integrate with Map Spawning and Camera

**Files:**
- Modify: `src/map/mod.rs`
- Modify: `src/camera.rs`

- [ ] **Step 1: Use TerrainGenerator in spawn_map**

```rust
// src/map/mod.rs
fn spawn_map(
    mut commands: Commands,
    terrain_gen: Res<TerrainGenerator>, // Use resource
    mut map_data: ResMut<MapData>,
    // ...
) {
    // ... inside loop
    let elevation = terrain_gen.get_elevation(x as f32, z as f32);
    // Note: get_elevation already returns absolute height, 
    // but TileData stores 0..1 scale. We should divide by MAX_HEIGHT.
    tile_data.elevation = elevation / MAX_HEIGHT;
}
```

- [ ] **Step 2: Raise Camera height in setup_camera**

```rust
// src/camera.rs
fn setup_camera(mut commands: Commands) {
    commands.spawn(MainCameraBundle {
        // ...
        transform: Transform::from_xyz(0.0, 30.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        // ...
    });
}
```

- [ ] **Step 3: Commit**

```bash
git add src/map/mod.rs src/camera.rs
git commit -m "feat: integrate TerrainGenerator and adjust camera height"
```

---

### Task 4: Verification

**Files:**
- Create: `tests/terrain_gen_test.rs`

- [ ] **Step 1: Write verification test**

```rust
use savage_fantasy::map::terrain_gen::TerrainGenerator;

#[test]
fn test_terrain_height_ranges() {
    let gen = TerrainGenerator::new(42);
    for x in -100..100 {
        for z in -100..100 {
            let h = gen.get_elevation(x as f32, z as f32);
            assert!(h >= 0.0 && h <= 12.0);
        }
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test`

- [ ] **Step 3: Final Commit**

```bash
git add tests/terrain_gen_test.rs
git commit -m "test: verify terrain generator output ranges"
```
