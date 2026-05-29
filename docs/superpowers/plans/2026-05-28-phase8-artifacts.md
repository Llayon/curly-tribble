# Phase 8/9: Artifacts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Artifacts as independent entities with customizable locations (Ground, Treasure, Trade) and specialized editor tools.

**Architecture:** 
- **Data**: Extract `Artifact` into a distinct Component with an `ArtifactLocation` state enum.
- **Auto-Fill**: During the transition to the Artifacts phase, parse all `TreasureDeposit` contents, spawn `Artifact` entities for each artifact item, and replace the inline item with an `ArtifactRef(Entity)`.
- **UI & Tools**: Introduce an Artifacts hierarchy in the inspector. Implement a ground placement tool that updates the artifact's location state and coordinates.
- **Visuals**: Render ground artifacts as bright green gizmo spheres.

**Tech Stack:** Bevy 0.18.1, egui.

---

### Task 1: Artifact Data Structures

**Files:**
- Create: `src/map/artifacts.rs`
- Modify: `src/map/treasures.rs`
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Define `Artifact` and `ArtifactLocation` in `artifacts.rs`**
```rust
use bevy::prelude::*;
use crate::map::{ArtifactType, HexCoord, ResourceType, TargetEntity};

#[derive(Debug, Clone, Reflect, PartialEq)]
pub struct TradeConfig {
    pub faction_id: u32,
    pub cost_type: ResourceType,
    pub cost_amount: u32,
    pub unlock_condition: String,
}

#[derive(Debug, Clone, Reflect, PartialEq)]
pub enum ArtifactLocation {
    InTreasure(TargetEntity),
    OnGround(HexCoord),
    InTrade(TradeConfig),
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Artifact {
    pub artifact_type: ArtifactType,
    pub location: ArtifactLocation,
}

#[derive(Bundle)]
pub struct ArtifactBundle {
    pub artifact: Artifact,
    pub name: Name,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}
```

- [ ] **Step 2: Update `TreasureItem` in `treasures.rs`**
```rust
// In src/map/treasures.rs
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub enum TreasureItem {
    Gold(u32),
    Resources { resource: ResourceType, amount: u32 },
    // Artifact definition before Phase 8
    ArtifactDef(ArtifactType),
    // Artifact reference after auto-fill
    ArtifactRef, 
    TreasureMap, 
}
// Add TargetEntity to ArtifactRef relation
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct ContainsArtifact {
    pub artifact: TargetEntity,
}
```

- [ ] **Step 3: Register in `MapPlugin` (`src/map/mod.rs`)**
```rust
// src/map/mod.rs
pub mod artifacts;
pub use artifacts::{Artifact, ArtifactLocation, TradeConfig, ArtifactBundle};
// inside build()
app.register_type::<artifacts::Artifact>()
   .register_type::<artifacts::ArtifactLocation>()
   .register_type::<artifacts::TradeConfig>()
   .register_type::<treasures::ContainsArtifact>();
```

- [ ] **Step 4: Run `cargo check --quiet`**

---

### Task 2: Auto-Fill Transition Logic

**Files:**
- Modify: `src/map/phase_transitions.rs`
- Modify: `src/map/generation/treasures.rs` (update `spawn_treasure` to use `ArtifactDef`)

- [ ] **Step 1: Implement `extract_artifacts` system in `phase_transitions.rs`**
```rust
// Add this system to PhaseTransitionsPlugin running on state change to Artifacts
pub fn extract_artifacts_on_phase_change(
    mut commands: Commands,
    mut q_treasures: Query<(Entity, &mut crate::map::TreasureDeposit)>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    if *phase.get() != crate::game_state::EditorPhase::Artifacts { return; }

    for (treasure_ent, mut deposit) in &mut q_treasures {
        let mut new_contents = Vec::new();
        for item in deposit.contents.drain(..) {
            if let crate::map::TreasureItem::ArtifactDef(a_type) = item {
                // Spawn artifact
                let art_ent = commands.spawn(crate::map::ArtifactBundle {
                    artifact: crate::map::Artifact {
                        artifact_type: a_type,
                        location: crate::map::ArtifactLocation::InTreasure(treasure_ent),
                    },
                    name: Name::new(format!("{:?}", a_type)),
                    transform: Transform::default(),
                    global_transform: GlobalTransform::default(),
                    visibility: Visibility::Hidden, // Hidden inside chest
                    inherited_visibility: InheritedVisibility::default(),
                }).id();
                
                // Add relation
                commands.entity(treasure_ent).with_children(|parent| {
                    parent.spawn((Name::new("Contains Artifact"), crate::map::ContainsArtifact { artifact: art_ent }));
                });
                new_contents.push(crate::map::TreasureItem::ArtifactRef);
            } else {
                new_contents.push(item);
            }
        }
        deposit.contents = new_contents;
    }
}
```

- [ ] **Step 2: Update `TreasureItem::Artifact` references in project**
Fix `src/map/generation/bio.rs`, `src/map/generation/treasures.rs`, `src/ui/panels/inspector.rs` to use `ArtifactDef` instead of `Artifact`.

- [ ] **Step 3: Register `extract_artifacts_on_phase_change` in `PhaseTransitionsPlugin`**

---

### Task 3: Editor State & Tool Logic

**Files:**
- Modify: `src/game_state.rs`
- Create: `src/map/tools/artifact.rs`
- Modify: `src/map/tools/mod.rs`

- [ ] **Step 1: Add state to `game_state.rs`**
```rust
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Reflect)]
pub enum EditorPhase {
    // ...
    Treasures,
    Artifacts, // Add this
    Height3D,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct ArtifactToolState {
    pub selected_artifact: Option<Entity>,
    pub placing_on_ground: bool,
}
```
Register `ArtifactToolState` in `GameStatePlugin`.

- [ ] **Step 2: Implement `handle_artifact_tools` in `artifact.rs`**
```rust
use bevy::prelude::*;
use crate::game_state::{EditorPhase, ArtifactToolState};
use crate::map::{Artifact, ArtifactLocation, HEX_SIZE, HexCoord};
use crate::map::tools::utils::get_mouse_world_pos;

pub struct ArtifactToolPlugin;
impl Plugin for ArtifactToolPlugin { fn build(&self, _app: &mut App) {} }

pub fn handle_artifact_tools(
    phase: Res<State<EditorPhase>>,
    mut state: ResMut<ArtifactToolState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q_artifacts: Query<&mut Artifact>,
) {
    if *phase.get() != EditorPhase::Artifacts || !state.placing_on_ground { return; }

    let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) else { return; };
    
    if mouse_input.just_pressed(MouseButton::Left) {
        if let Some(entity) = state.selected_artifact {
            if let Ok(mut artifact) = q_artifacts.get_mut(entity) {
                let coord = HexCoord::from_world(world_pos, HEX_SIZE);
                artifact.location = ArtifactLocation::OnGround(coord);
            }
        }
        state.placing_on_ground = false; // Turn off tool after placement
    }
    if mouse_input.just_pressed(MouseButton::Right) {
        state.placing_on_ground = false;
    }
}
```

- [ ] **Step 3: Register in `src/map/mod.rs`**

---

### Task 4: Visualization (Gizmos)

**Files:**
- Modify: `src/economy/mesh_gen/treasures.rs` (or create `artifacts.rs` in `mesh_gen`)

- [ ] **Step 1: Implement `draw_artifact_gizmos` in `treasures.rs`**
```rust
use crate::map::artifacts::{Artifact, ArtifactLocation};

pub fn draw_artifact_gizmos(
    mut gizmos: Gizmos,
    q_artifacts: Query<&Artifact>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    if *phase.get() < crate::game_state::EditorPhase::Artifacts { return; }

    for artifact in q_artifacts.iter() {
        if let ArtifactLocation::OnGround(coord) = artifact.location {
            let pos = coord.to_world(crate::map::HEX_SIZE);
            gizmos.sphere(pos + Vec3::Y * 0.5, 0.3, Color::srgb(0.0, 1.0, 0.0)); // Bright Green
        }
    }
}
```

- [ ] **Step 2: Register `draw_artifact_gizmos` in `MeshGenPlugin`**

---

### Task 5: UI Integration (Timeline & Inspector)

**Files:**
- Modify: `src/ui/panels/bottom_bar.rs`
- Modify: `src/ui/panels/inspector.rs`

- [ ] **Step 1: Update Timeline in `bottom_bar.rs`**
Insert `(EditorPhase::Artifacts, "8. Artifacts")` before `Height3D`.

- [ ] **Step 2: Implement Artifact Hierarchy & Properties in `inspector.rs`**
```rust
// Under Faction Hierarchy or inside it
if *current_phase == EditorPhase::Artifacts {
    ui.collapsing("🏺 Artifacts", |ui| {
        // Query all Artifacts and display them.
        // Let user select one -> sets ArtifactToolState::selected_artifact.
    });
}

// In Selection Properties (if an artifact is selected):
if let Some(art_ent) = artifact_state.selected_artifact {
    if let Ok(mut artifact) = q_artifacts.get_mut(art_ent) {
        ui.label(format!("Artifact: {:?}", artifact.artifact_type));
        
        // Location dropdown
        // If changed to OnGround, set artifact_state.placing_on_ground = true.
        // If changed to InTrade, show TradeConfig inputs.
    }
}
```

- [ ] **Step 3: Run `cargo check` and fix any UI mutability issues.**
