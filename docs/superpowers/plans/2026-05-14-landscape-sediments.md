# Plan: Phase 3 (Landscape) and Phase 4 (Sediments)

## Phase 3: Landscape (Topology & Elevation)
**Goal**: Define the 3D skeleton of the world.
1. **Mountains & Cliffs**: Brushes to raise terrain. Cliffs are automatically generated when elevation difference between neighbors exceeds 0.3.
2. **Lakes**: Freshwater depressions. Unlike Ocean, Lakes are internal and have a fixed water level.
3. **Plateaus**: Leveling tool to create flat buildable areas at high altitudes.
4. **Dynamic Adaptation**: Procedural terrain generation must avoid Faction centers (preserving flatness for buildings) while creating natural barriers (mountains) between them.

## Phase 4: Sediments (Surfaces & Biology)
**Goal**: Paint the "living" layer of the map.
1. **Soil & Biomes**: Brushes for `Grass`, `Mud`, `Sand`, `Stone`, `Snow`.
2. **Fertility System**: Logic to determine crop yield based on soil type and water proximity.
3. **Vegetation (Forests)**: Resource brushes for planting dense woods or scattered trees.
4. **Natural Deposits**: Placing iron, coal, and stone resources.

## Architectural Integrity
- Keep `TileData` as the single source of truth for both topology (elevation) and sediments (terrain type).
- Validation must ensure that every settlement has a path to fresh water or fertile soil.