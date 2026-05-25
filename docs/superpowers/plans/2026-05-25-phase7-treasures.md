# Phase 7: Treasures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the "Discovery" layer with visible/hidden treasures, artifacts, and a visual linking system for treasure maps.

**Architecture:** 
- **Data**: Entity-based `TreasureDeposit` with strict enum-based contents.
- **Visuals**: Phase-cumulative spheres (Green for Ruins, Orange for Clues) with persistent connection lines.
- **UX**: A state-machine-driven `LinkTool` with elastic-band gizmos and hover-validation.
- **Pipeline**: Non-destructive auto-fill for treasures based on island geography.

**Tech Stack:** Bevy 0.18.1, egui, rand, noise.

---

### Task 1: ECS Foundation & Core Types

**Files:**
- Create: `src/map/treasures.rs`
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Define types and components in `treasures.rs`**
```rust
use bevy::prelude::*;
use crate::map::{HexCoord, MapEntity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ResourceType { Wood, Stone, Iron, Gold, Food }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtifactType { AncientRelic, TradeLedger, MagicCompass }

#[derive(Debug, Clone, Reflect)]
pub enum TreasureItem {
    Gold(u32),
    Resources { resource: ResourceType, amount: u32 },
    Artifact(ArtifactType),
    TreasureMap(Entity),
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct TreasureDeposit {
    pub is_visible: bool,
    pub contents: Vec<TreasureItem>,
    pub hex_coord: HexCoord,
}

#[derive(Bundle)]
pub struct TreasureBundle {
    pub treasure: TreasureDeposit,
    pub name: Name,
    pub marker: MapEntity,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}
```

- [ ] **Step 2: Register types and declare module in `src/map/mod.rs`**
```rust
pub mod treasures;
// ...
app.register_type::<treasures::TreasureDeposit>()
   .register_type::<treasures::TreasureItem>()
   .register_type::<treasures::ResourceType>()
   .register_type::<treasures::ArtifactType>();
```

- [ ] **Step 3: Run `cargo check --quiet`**

---

### Task 2: Visual Layer & Gizmos

**Files:**
- Create: `src/economy/mesh_gen/treasures.rs`
- Modify: `src/economy/mesh_gen/mod.rs`

- [ ] **Step 1: Implement `draw_treasure_gizmos`**
Logic:
1. Draw Emerald Green spheres for `is_visible == true`.
2. Draw Amber Orange spheres for `is_visible == false`.
3. Draw persistent thin white lines for `TreasureMap(Entity)` links.
4. Ensure it respects `current_phase >= EditorPhase::Treasures`.

- [ ] **Step 2: Register in `MeshGenPlugin` Update schedule**

---

### Task 3: Interactive Placement & Link Tool

**Files:**
- Modify: `src/game_state.rs`
- Create: `src/map/tools/treasure.rs`
- Modify: `src/map/tools/mod.rs`

- [ ] **Step 1: Add `LinkToolState` and `CurrentTool::treasure_mode`**
- [ ] **Step 2: Implement `handle_treasure_tools`**
Logic:
1. If mode is `Link`, implement the elastic-band state machine.
2. If mode is `Spawn`, spawn `TreasureBundle`.
3. Use the `get_mouse_world_pos` helper for picking.

---

### Task 4: UI Integration (Timeline & Inspector)

**Files:**
- Modify: `src/ui/panels/tools.rs`
- Modify: `src/ui/panels/inspector.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add "7. Treasures" to timeline**
- [ ] **Step 2: Implement Treasure Sidebar (Spawn vs Link toggle)**
- [ ] **Step 3: Update Inspector to show `contents` vector for selected Treasure**

---

### Task 5: Auto-Fill & Validation

**Files:**
- Modify: `src/map/generation/npcs.rs`
- Modify: `src/map/validation.rs`

- [ ] **Step 1: Implement `auto_spawn_treasures`**
- [ ] **Step 2: Implement `validate_treasures`**
Logic: Ensure hidden treasures aren't in the ocean.
