# Standard Operating Procedures: Reactive Faction Relocation

## Overview
Factions in the editor are represented by `FactionMarker` entities. These entities must always occupy a valid land hex. The relocation pipeline ensures this invariant is maintained reactive-ly.

## Components
- **FactionMarker**: Stores `FactionType` and current `HexCoord`.
- **Relocation System**: Monitors `MapData` changes.

## Logic Flow
1. **Trigger**: System detects `MapData` has been modified (either via phase change or tool use).
2. **Invalidation Check**: For each faction, check `map_data.get_tile(marker.hex_coord)`.
3. **If `is_ocean`**:
    - Initialize BFS queue with `marker.hex_coord`.
    - Traverse hex neighbors layer by layer.
    - **Stop** at the first hex where `!tile.is_ocean`.
    - **Update** `marker.hex_coord` and the entity's `Transform`.
4. **If valid**: Update `Transform` to match potential grid coordinate shifts.

## Visual Filter (Factions Phase)
During `EditorPhase::Factions`, the renderer MUST:
- Set `is_factions_filter = true`.
- Color all non-ocean tiles to `[0.15, 0.15, 0.18]` (Dark Gray).
- Preserve deep blue ocean colors.
- Enable `draw_factions_gizmos` to overlay colored hexagons for each faction.

## Topology Validation
Transition from `Factions` phase to `Landscape` phase is BLOCKED if:
- `total_land == 0`.
- The continent is not a single connected component (BFS test).
- There are isolated ocean lakes within the continent (BFS test from border).
