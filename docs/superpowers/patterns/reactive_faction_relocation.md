# Standard Operating Procedures: Reactive Faction Relocation

## Overview
Factions in the editor are represented by `FactionMarker` entities. These entities must always occupy a valid land hex. The relocation pipeline ensures this invariant is maintained reactive-ly.

## Components
- **FactionMarker**: Stores `FactionType` and current `HexCoord`.
- **Relocation System**: Monitors `MapData` changes.

## Logic Flow
1. **Trigger**: System detects `MapData` has been modified or faction territory size changes.
2. **Threshold Check**: For each faction, count current hexes. 
    - **Player (ID 1)**: Min 15 hexes.
    - **NPC**: Min 20 hexes.
3. **If below threshold or `is_ocean`**:
    - Clear all existing faction hexes to prevent "ghost" fragments.
    - Initialize BFS relocation logic.
    - **Stop** at the first valid land cluster that fits the required size.
    - **Update** `TileData::faction_id`.
4. **Dynamic Geography**: After relocation, trigger `spawn_map_internal` to update mountains and rivers around the new location.

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
