# Dynamic Terrain Reconfiguration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a dynamic, UI-driven terrain regeneration system using `bevy-inspector-egui` and `bevy_egui`.

**Architecture:** 
- `TerrainConfig` resource for parameter tuning.
- `GenerateMapEvent` for triggering regeneration.
- `MapEntity` marker for cleanup via `despawn_recursive`.
- Custom Egui window for "Regenerate World" and "Randomize Seed".

**Tech Stack:** Rust, Bevy 0.18, `bevy-inspector-egui`, `bevy_egui`.

---

### Task 1: Dependencies and Config Resource

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/map/terrain_gen.rs`

- [ ] **Step 1: Add dependencies to Cargo.toml**

```toml
bevy-inspector-egui = "0.28" # Adjust for Bevy 0.18 compatibility if needed
bevy_egui = "0.32"
```

- [ ] **Step 2: Define TerrainConfig and update TerrainGenerator**

```rust
// src/map/terrain_gen.rs
use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

#[derive(Resource, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct TerrainConfig {
    pub map_width: u32,
    pub map_height: u32,
    pub seed: u32,
    #[inspector(min = 0.001, max = 0.1)]
    pub macro_freq: f64,
    #[inspector(min = 1.0, max = 25.0)]
    pub macro_height: f32,
    #[inspector(min = 1.0, max = 10.0)]
    pub macro_sharpness: f32,
    #[inspector(min = 0.001, max = 0.1)]
    pub plateau_freq: f64,
    #[inspector(min = 1.0, max = 10.0)]
    pub plateau_height: f32,
    #[inspector(min = 1.0, max = 10.0)]
    pub plateau_steps: f32,
    #[inspector(min = 0.001, max = 0.1)]
    pub warp_freq: f64,
    #[inspector(min = 0.0, max = 20.0)]
    pub warp_strength: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            map_width: 120,
            map_height: 120,
            seed: 42,
            macro_freq: 0.03,
            macro_height: 12.0,
            macro_sharpness: 4.0,
            plateau_freq: 0.04,
            plateau_height: 4.0,
            plateau_steps: 4.0,
            warp_freq: 0.02,
            warp_strength: 5.0,
        }
    }
}

// Update TerrainGenerator::get_elevation to accept &TerrainConfig
```

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml src/map/terrain_gen.rs
git commit -m "feat: add TerrainConfig and update dependencies"
```

---

### Task 2: Event and Cleanup Logic

**Files:**
- Modify: `src/map/mod.rs`
- Modify: `src/map/zoning.rs`
- Modify: `src/economy/mesh_gen.rs`

- [ ] **Step 1: Define MapEntity and GenerateMapEvent**

```rust
// src/map/mod.rs
#[derive(Component)]
pub struct MapEntity;

#[derive(Event)]
pub struct GenerateMapEvent;
```

- [ ] **Step 2: Attach MapEntity to all spawned entities**

Update `spawn_map` (logical tiles) and `SpawnGlobalTerrainCommand` (terrain, water, roofs) to include `MapEntity`.

- [ ] **Step 3: Commit**

```bash
git add src/map/mod.rs src/map/zoning.rs src/economy/mesh_gen.rs
git commit -m "feat: add MapEntity marker and GenerateMapEvent"
```

---

### Task 3: MapPlugin Refactor and UI

**Files:**
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Refactor MapPlugin for dynamic gen**

```rust
// src/map/mod.rs
impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainConfig>()
            .register_type::<TerrainConfig>()
            .add_plugins(bevy_inspector_egui::quick::ResourceInspectorPlugin::<TerrainConfig>::default())
            .add_event::<GenerateMapEvent>()
            .add_systems(Startup, |mut ev: EventWriter<GenerateMapEvent>| { ev.send(GenerateMapEvent); })
            .add_systems(Update, (regeneration_ui, handle_regeneration));
    }
}
```

- [ ] **Step 2: Implement handle_regeneration (cleanup + spawn)**

```rust
fn handle_regeneration(
    mut commands: Commands,
    mut ev_gen: EventReader<GenerateMapEvent>,
    q_map_entities: Query<Entity, With<MapEntity>>,
    config: Res<TerrainConfig>,
    // ... other spawn_map params
) {
    for _ in ev_gen.read() {
        // 1. Cleanup
        for entity in &q_map_entities {
            commands.entity(entity).despawn_recursive();
        }
        // 2. Spawn (call spawn_map logic or trigger it)
    }
}
```

- [ ] **Step 3: Implement themed regeneration_ui**

```rust
fn regeneration_ui(
    mut contexts: bevy_egui::EguiContexts,
    mut config: ResMut<TerrainConfig>,
    mut ev_gen: EventWriter<GenerateMapEvent>,
) {
    // Styling: dark stone (#1A1A1A) and bronze (#CD7F32)
    // Buttons: "Regenerate World", "Randomize Seed"
}
```

- [ ] **Step 4: Commit**

```bash
git add src/map/mod.rs
git commit -m "feat: implement themed regeneration UI and event handling"
```

---

### Task 4: Verification

- [ ] **Step 1: Run and verify**

1. Launch the game.
2. Change noise parameters in the Inspector.
3. Click "Randomize Seed".
4. Click "Regenerate World".
5. Verify old map is gone and new map matches parameters.

- [ ] **Step 2: Final Commit**

```bash
git add .
git commit -m "chore: finish dynamic terrain reconfiguration"
```
