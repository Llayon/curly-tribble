# ADR 0006: Reactive Relocation Pipeline

## Context
In professional map editors for city-building games (e.g., *Pioneers of Pagonia*), the relationship between environment data (terrain) and object data (factions, buildings) must be non-destructive and resilient to real-time changes. If a user "erases" land under a player's starting position using the Ocean tool, the player entity must not be deleted or left in an invalid state (floating over water).

## Decision
We implement a **Non-destructive Reactive Relocation Pipeline**. Instead of manual placement or deletion, the system automatically "heals" invalid placements by searching for the nearest valid terrain.

### Key Components:
1. **Validation System**: A system that runs every time `MapData` changes, checking if the tile under a `FactionMarker` has become `is_ocean`.
2. **BFS Relocation**: If a placement becomes invalid, the system performs a Breadth-First Search (BFS) on the hexagonal grid starting from the current coordinate to find the closest valid land hex.
3. **Phased Visualization**: A dedicated `Factions` editor phase with a high-contrast visual filter (Dark Gray land, Bright Blue water) to focus purely on territorial placement.

## Rationale
- **User Experience**: Prevents accidental loss of progress or "dead" entities when shaping the island.
- **Architectural Integrity**: Decouples terrain shaping from faction placement. Factions "observe" the world and adapt to its topology.
- **Pathfinding Safety**: Ensures that all factions always start on traversable land, satisfying future logistics requirements.

## Consequences
- **Performance**: Validation and BFS search must be efficient. Currently limited to a 400-hex search radius to prevent freezes on completely submerged maps.
- **Persistence**: User manual relocations (future tool) must be respected unless the tile becomes ocean.
- **Visuals**: Requires a specialized rendering pass in the mesh generator to support the phase-specific filter.
