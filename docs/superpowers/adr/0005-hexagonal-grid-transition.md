# ADR 0005: Hexagonal Grid Transition

## Status
Accepted

## Context
The project initially used a standard square grid (Square Grid) for the world representation and navigation. However, to achieve a more organic look and feel, and inspired by modern strategy games like *Pioneers of Pagonia*, we decided to transition to a hexagonal grid. Hexagonal grids provide better movement symmetry (6 neighbors at equal distance) and more natural terrain boundaries.

## Decision
We transitioned the entire map system from a square grid to a **Hexagonal Grid** using the following standards:

1.  **Coordinate System**: Axial coordinates (`HexCoord { q: i32, r: i32 }`).
2.  **Orientation**: Pointy-topped hexagons.
3.  **Storage**: `HashMap<HexCoord, TileData>` in `MapData` instead of a flat `Vec`.
4.  **Mesh Generation**: Each tile is now a 7-vertex hexagon (1 center + 6 corners) with 6 triangles.
5.  **Mathematics**: Implemented `HexCoord` with neighbor discovery and world-to-grid/grid-to-world conversion using trigonometry.

## Consequences
- **Movement**: Pathfinding will need to be updated to handle 6 neighbors instead of 4 (Dijkstra/A*).
- **Geometry**: The world is now composed of true hexagonal prisms, allowing for more natural-looking coastlines and ridges.
- **Rendering**: Vertex coloring is used for terrain types, requiring `StandardMaterial` to have white base color and valid vertex color attributes.
- **Complexity**: Math for coordinate conversion and neighbor lookup is slightly more complex than square grids.
- **Temporary State**: River and mud bank generation were temporarily disabled and will be refactored to support the hexagonal grid in the next phase.
