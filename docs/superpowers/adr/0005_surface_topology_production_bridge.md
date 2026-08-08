# ADR 0005: SurfaceTopology Production Terrain Bridge

## Status
Accepted (Milestone M3.1)

## Context
In Milestone M3, `SurfaceTopology` was introduced as a derived semantic 2-manifold surface layer built side-by-side with authoritative `HexFaceTopology`, while production rendering (`handle_rebuild_mesh`) continued calling the legacy direct adapter `derive_terrain_topology(&map_data, &face_topology)`.

To complete the semantic surface foundation without altering visual rendering or breaking overlay systems, production ground terrain rendering must route through `SurfaceTopology`.

## Decision
1. Create `derive_terrain_topology_from_surface(&SurfaceTopology)` in `src/map/surface_topology/terrain_adapter.rs` as a pure projection adapter.
2. The adapter consumes only `&SurfaceTopology` and projects `vertices_xz`, `triangles`, `triangle_cells`, and `vertex_influences` (derived strictly from semantic face incidence).
3. Switch production ground mesh rebuild in `handle_rebuild_mesh` (`src/map/systems.rs`) to consume `Res<SurfaceTopology>` and derive `TerrainTopology` via `derive_terrain_topology_from_surface`.
4. Update `MapPlugin` system scheduling to run `handle_rebuild_mesh` strictly `.after(surface_topology::runtime::regenerate_surface_topology)`.
5. Retain `derive_terrain_topology(&map_data, &face_topology)` in `src/map/topology_adapter.rs` strictly as a reference test oracle for 144-case and 4,608-case bit-exact compatibility matrix tests.
6. Enforce production decoupling with architecture guards forbidding `HexFaceTopology`, `MapData`, `HEX_SIZE`, or geometry constants in `terrain_adapter.rs` and forbidding direct `derive_terrain_topology(&map_data` calls in production `systems.rs`.

## Consequences
- **Positive**:
  - `SurfaceTopology` becomes the authoritative input for production ground terrain rendering.
  - 100% bit-exact rendering compatibility proven across 144 canonical and 4,608 extended matrix cases.
  - Zero regression in height, water overlays, roof overlays, cliff bindings, or map files.
  - Decouples ground terrain mesh creation from regular hex geometry and coarse face generation.
- **Negative**:
  - Caches `TerrainTopology` representation as an intermediate rendering bridge until future milestones refactor height solver and rendering.
