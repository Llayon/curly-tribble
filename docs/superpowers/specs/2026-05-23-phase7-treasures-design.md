# Design Spec: Phase 7 - Treasures, Artifacts, and Maps

## Context
Phase 7 introduces the "Discovery" layer to the map. Following the *Pioneers of Pagonia* model, players uncover the world through visible landmarks and hidden clues. This spec defines the data structures and professional editor tools required to manage these treasures and their logical connections.

## 1. Data Structures (ECS)

### Strict Type Safety
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ResourceType { Wood, Stone, Iron, Gold, Food }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtifactType { AncientRelic, TradeLedger, MagicCompass }

#[derive(Debug, Clone, Reflect)]
pub enum TreasureItem {
    Gold(u32),
    Resources { resource: ResourceType, amount: u32 },
    Artifact(ArtifactType),
    TreasureMap(Entity), // Links to another TreasureDeposit
}
```

### Components & Bundles
- **`TreasureDeposit`**:
    - `is_visible: bool` (True: Green/Ruins, False: Orange/Hidden)
    - `contents: Vec<TreasureItem>`
    - `hex_coord: HexCoord`
- **`TreasureBundle` (Bevy 0.18.1 compliant)**:
    - `treasure: TreasureDeposit`
    - `name: Name`
    - `marker: MapEntity`
    - `transform: Transform`
    - `global_transform: GlobalTransform`
    - `visibility: Visibility`
    - `inherited_visibility: InheritedVisibility`

## 2. Interaction Design: The Link Tool

### State Machine (`LinkToolState`)
A global resource to manage the connection flow:
- `Idle`: Default state.
- `SelectingTarget(Entity)`: Source treasure selected; waiting for target.

### Interaction Flow
1. **Select Tool**: Choose "Link" from the left sidebar.
2. **Contextual Cursor**: 
    - Hover over nothing: Dimmed cursor.
    - Hover over `TreasureDeposit`: Highlight entity + "LINK SOURCE".
3. **Pick Source (A)**: Click a treasure. State becomes `SelectingTarget(A)`.
4. **Elastic String**: A white dashed `gizmos.line` draws from A to the world cursor.
5. **Validation Hover**:
    - Over valid B: Line turns Pulsing Green + "LINK TARGET".
    - Over invalid (A itself or cyclic): Line turns Red + "INVALID".
6. **Pick Target (B)**: Click valid treasure.
    - `A.contents.push(TreasureMap(B))`.
    - State returns to `Idle`.
7. **Cancel**: PKM or Escape to reset to `Idle`.

## 3. Visualization in Editor
- **Visible (Ruins)**: Emerald Green sphere/model.
- **Hidden (Clues)**: Amber Orange sphere/model.
- **Connections**: Persistent thin white lines between treasures that contain maps to each other.

## 4. Verification & Constraints
- **Validation**: `run_map_validation` ensures no "Hidden" treasures are in the ocean.
- **Guard #22**: Use `get_single().ok()` or `iter()` for all picking logic.
- **Reflect**: All enums and structs must derive `Reflect` for seamless `bevy_inspector_egui` integration.
