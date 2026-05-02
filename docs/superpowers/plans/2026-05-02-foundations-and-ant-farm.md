# The Dark Narrative: Foundations & Ant Farm Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a functional "ant farm" prototype where pawns move on a tilemap, have needs (hunger), and can interact with player-defined zones.

**Architecture:** Uses Bevy ECS with separate systems for Need decay, Goal-setting (Brain), and Movement. Maps are tile-based with an abstraction for Zoning.

**Tech Stack:** Bevy 0.18, Rust.

---

### Task 1: Camera & World Setup

**Files:**
- Create: `src/camera.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Implement Camera plugin**
Create `src/camera.rs` with a basic 2D/3D camera and panning logic.

```rust
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
           .add_systems(Update, move_camera);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    let mut transform = query.single_mut();
    let speed = 10.0;
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { direction.z -= 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.z += 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }
    transform.translation += direction.normalize_or_zero() * speed * time.delta_secs();
}
```

- [ ] **Step 2: Update main.rs to use the plugin**
```rust
mod camera;
use camera::CameraPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraPlugin)
        .run();
}
```

- [ ] **Step 3: Verify build**
Run: `cargo check`
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add src/main.rs src/camera.rs
git commit -m "feat: add basic camera control"
```

---

### Task 2: Basic Tilemap & Zoning Data

**Files:**
- Create: `src/map/mod.rs`, `src/map/zoning.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Define Tile and Zone components**
In `src/map/zoning.rs`:
```rust
use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    Empty,
    FoodStockpile,
    Housing,
}

#[derive(Component)]
pub struct Tile;

#[derive(Component)]
pub struct Zone(pub ZoneType);
```

- [ ] **Step 2: Implement Map setup**
In `src/map/mod.rs`, spawn a 10x10 grid of planes representing tiles.

```rust
use bevy::prelude::*;
pub mod zoning;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_map);
    }
}

fn spawn_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    let white_mat = materials.add(Color::WHITE);
    
    for x in -5..5 {
        for z in -5..5 {
            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(white_mat.clone()),
                Transform::from_xyz(x as f32, 0.0, z as f32),
                super::map::zoning::Tile,
            ));
        }
    }
}
```

- [ ] **Step 3: Register MapPlugin in main.rs**
```rust
mod map;
use map::MapPlugin;
// ... add to App
```

- [ ] **Step 4: Commit**
```bash
git add src/map/ git commit -m "feat: spawn basic 10x10 tile grid"
```

---

### Task 3: The First Pawn & Needs System

**Files:**
- Create: `src/pawn/mod.rs`, `src/pawn/needs.rs`

- [ ] **Step 1: Define Pawn and Hunger**
In `src/pawn/mod.rs`:
```rust
use bevy::prelude::*;
pub mod needs;

#[derive(Component)]
pub struct Pawn;

#[derive(Component)]
pub struct Hunger(pub f32); // 0.0 (full) to 100.0 (starving)
```

- [ ] **Step 2: Implement Hunger decay system**
In `src/pawn/needs.rs`:
```rust
use bevy::prelude::*;
use super::Hunger;

pub fn update_hunger(time: Res<Time>, mut query: Query<&mut Hunger>) {
    for mut hunger in &mut query {
        hunger.0 += 1.0 * time.delta_secs(); // Increases by 1 every second
    }
}
```

- [ ] **Step 3: Spawn a test pawn**
Add a startup system in `src/pawn/mod.rs` to spawn a cube representing a pawn.
```rust
pub struct PawnPlugin;

impl Plugin for PawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_test_pawn)
           .add_systems(Update, needs::update_hunger);
    }
}

fn spawn_test_pawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.5)))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(0.0, 0.25, 0.0),
        super::pawn::Pawn,
        super::pawn::Hunger(0.0),
    ));
}
```

- [ ] **Step 4: Commit**
```bash
git add src/pawn/ git commit -m "feat: add pawn with hunger decay system"
```

---

### Task 4: Simple Brain & Movement (Eating Loop)

**Files:**
- Create: `src/pawn/brain.rs`
- Modify: `src/economy.rs` (create this file for global resources)

- [ ] **Step 1: Create Global Resources**
In `src/economy.rs`:
```rust
use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct GlobalResources {
    pub food: f32,
}
```

- [ ] **Step 2: Implement Brain logic**
In `src/pawn/brain.rs`, if hunger > 50, "eat" from global resources and reduce hunger.
```rust
use bevy::prelude::*;
use super::{Hunger};
use crate::economy::GlobalResources;

pub fn think(mut resources: ResMut<GlobalResources>, mut query: Query<&mut Hunger>) {
    for mut hunger in &mut query {
        if hunger.0 > 50.0 && resources.food > 0.0 {
            resources.food -= 1.0;
            hunger.0 = 0.0;
            info!("Pawn ate! Remaining food: {}", resources.food);
        }
    }
}
```

- [ ] **Step 3: Integrate into App**
Add `GlobalResources` as a resource and register `think` system.

- [ ] **Step 4: Final verification**
Run: `cargo run`
Expected: A window with a grid and a red cube. Check console for "Pawn ate!" after hunger passes 50 (provided you manually give initial food in a setup system).

- [ ] **Step 5: Commit**
```bash
git add .
git commit -m "feat: implement basic hunger/eating logic"
```
