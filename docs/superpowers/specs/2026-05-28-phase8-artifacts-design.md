# Design Spec: Phase 8 - Artifacts & Trades

## Context
Following the *Pioneers of Pagonia* specification (Step 8/18), the Artifacts phase focuses on distributing the rare items defined during the Treasures phase. Artifacts transition from being simple loot inside a chest to becoming first-class entities that can be placed physically on the ground or integrated into the game's economy via NPC Faction Trades.

## 1. Data Structures (ECS)

### Artifact Entities
Artifacts will be extracted into their own entities to allow independent tracking and manipulation.

```rust
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Artifact {
    pub artifact_type: crate::map::ArtifactType,
    pub location: ArtifactLocation,
}

#[derive(Debug, Clone, Reflect, PartialEq)]
pub enum ArtifactLocation {
    /// Placed inside a treasure chest (default).
    InTreasure(Entity), 
    /// Placed explicitly on the map.
    OnGround(crate::map::HexCoord),
    /// Held by a faction to be traded.
    InTrade(TradeConfig),
}

#[derive(Debug, Clone, Reflect, PartialEq)]
pub struct TradeConfig {
    pub faction_id: u32,
    pub cost_type: crate::map::ResourceType,
    pub cost_amount: u32,
    pub unlock_condition: String,
}

#[derive(Bundle)]
pub struct ArtifactBundle {
    pub artifact: Artifact,
    pub name: Name,
    pub marker: crate::map::MapEntity,
}
```

### Updates to Phase 7 Data
`TreasureItem::Artifact(ArtifactType)` will remain as the definition in Phase 7. However, upon entering Phase 8, the auto-fill system will consume these definitions and spawn `ArtifactBundle` entities, linking them back via a new variant: `TreasureItem::ArtifactRef(Entity)`.

## 2. Editor UI & Interaction

### Hierarchy (Right Panel)
When in `EditorPhase::Artifacts` (Phase 8):
- The Inspector will display an "Artifacts" collapsing header.
- This lists all `Artifact` entities currently in the world.
- Selecting an artifact reveals its property editor.

### Properties Editor
For the selected Artifact, the Inspector will show:
- **Location Selector**: Dropdown to choose between `Treasure`, `Ground`, and `Trade`.
- **If Ground**:
  - Displays the current `HexCoord`.
  - A "Set Location via Click" toggle that temporarily changes the left-click behavior to pick a hex for this artifact (satisfying the "FaceIndex" requirement from Pagonia).
- **If Trade**:
  - Dropdown to select the target NPC Faction (satisfying the "Drag and Drop" reassignment requirement).
  - Inputs for Cost (Resource Type & Amount) and Unlock Condition.

## 3. Visualization
- **Ground Placement**: If `location == OnGround(coord)`, the `draw_treasure_gizmos` (or a new `draw_artifact_gizmos` system) will render a small, bright green sphere (pixel) at the hex coordinates.
- **In Trade**: Optionally draw a visual link (dashed line) from the nearest faction camp to the artifact if we want to visualize trade networks.

## 4. Phase Transition (Auto-Fill)
When transitioning from `Treasures` (Phase 7) to `Artifacts` (Phase 8):
1. Query all `TreasureDeposit`s.
2. Iterate through their `contents`.
3. If an `Artifact(type)` is found:
   - Spawn an `ArtifactBundle` with `location: InTreasure(treasure_entity)`.
   - Replace the item in the vector with `ArtifactRef(spawned_entity)`.

## 5. Architectural Compliance
- **Guard #18 (Semantic Graph)**: Ensure that `InTreasure(Entity)` is handled safely or mapped via a proper relation if strict compliance requires it (e.g., using a `HeldBy` relation component). For simplicity in UI state, enums are preferred, but if the guard trips, we will use a `TradeRelation` or `GroundLocation` component.
- **Guard #12 (Markers)**: No boolean flags. Location is handled strictly by the `ArtifactLocation` enum.
