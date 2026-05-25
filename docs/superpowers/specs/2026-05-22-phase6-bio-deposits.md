# Design Spec: Phase 6 - Plants, Animals, and Fish (Bio-Deposits)

## Context
Phase 6 transition the map from a structured landscape (Phases 1-5) into a resource-rich environment. Following the *Pioneers of Pagonia* model, biological resources are managed as "Deposits" - discrete clusters of harvestable flora or fauna.

## 1. Data Structures (ECS)

### Deposit Types
```rust
pub enum DepositType {
    // Animals
    Rabbit,
    Deer,
    Boar,
    // Plants
    WildFlax,
    Raspberries,
    Pumpkin,
    WildWheat,
    // Aquatic
    OceanFish,
}
```

### Components
- **`ResourceDeposit`**:
    - `deposit_type: DepositType`
    - `amount: u32` (e.g., 24 units)
    - `hex_coord: HexCoord`
    - `habitat_valid: bool` (cached validation result)
- **`PoiMarker` (Existing)**: Used for visibility and selection in the editor.

## 2. Editor Tools (UI)
- **Tool Sidebar**: A scrollable list of all `DepositType` variants (similar to Pagonia's "Select Entity" popup).
- **Placement Modes**:
    - **Click**: Spawn a single deposit in the targeted hex.
    - **Brush (Drag)**: Scatter small deposits (e.g., amount/4) across multiple hexes while dragging.
- **Properties Panel**:
    - Edit `amount` for the selected deposit.
    - Display habitat warnings (e.g., "⚠️ Missing Forest Habitat").

## 3. Business Logic & Constraints
- **Habitat Check**:
    - `Deer` and `Boar` require `ForestType != None` in the current or adjacent hexes.
    - `OceanFish` requires `is_ocean == true`.
    - `Plants` require `TerrainType` traits `allow_plants == true`.
- **Exclusivity**: Only one biological deposit type per hex (prevents overlapping icons).

## 4. Visualization
- **Billboard Icons**: Use 2D texture quads that always face the camera.
- **Color Coding**: 
    - Green/Yellow for plants.
    - Brown for land animals.
    - Blue for fish.
- **Validation Feedback**: Red outline or tint on the icon if `habitat_valid == false`.

## 5. Procedural Step (Auto-Fill)
Upon entering Phase 6, if no bio-deposits exist:
- Scatter `OceanFish` in coastal ocean hexes.
- Spawn `Deer`/`Boar` in large forest clusters.
- Distribute `WildFlax` and `Berries` in fertile plains.

## Verification
- **Guard #22**: No `.unwrap()` in placement logic.
- **Visuals**: Icons must be clearly visible above the terrain mesh (Y offset).
