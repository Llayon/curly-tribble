# SurfaceTopology Production Terrain Bridge (Pattern SOP)

## Overview

`SurfaceTopology` serves as the authoritative semantic surface model for the terrain engine. Milestone M3.1 bridges `SurfaceTopology` to the existing `TerrainTopology` rendering representation via a pure projection adapter (`derive_terrain_topology_from_surface`).

## Architecture Pipeline

```
MapData (Logical tiles & elevation)
   │
   ▼
HexFaceTopology (Authoritative coarse XZ boundary)
   │
   ▼
SurfaceTopology (Authoritative semantic 2-manifold surface)
   │
   ▼
derive_terrain_topology_from_surface(surface) -> TerrainTopology (Render compatibility)
   │
   ├──────────────► compute_vertex_heights(map_data, terrain_topology) -> Y positions
   │
   ▼
SpawnGlobalTerrainCommand -> Mesh creation
```

## Core Design & Coupling Rules

1. **Pure Projection**:
   `derive_terrain_topology_from_surface` accepts **ONLY** `&SurfaceTopology`. It does not accept or depend on `MapData`, `HexFaceTopology`, coarse `VertexId`/`FaceId`, `HEX_SIZE`, `to_world()`, `SurfaceVertexSource`, or elevation.
2. **1-to-1 Vertex Mapping**:
   `TerrainTopology.vertices_xz[i]` maps 1-to-1 to `SurfaceTopology.vertices[i].position`.
3. **Owner Hex Triangle Cells**:
   `TerrainTopology.triangle_cells[i]` is copied directly from `SurfaceFace.owner_hex`.
4. **Semantic Incidence Influences**:
   `TerrainTopology.vertex_influences[i]` is derived strictly from semantic face incidence (`surface.faces` $\to$ `owner_hex` $\to$ `sort` + `dedup`).
5. **Fail-Closed Runtime Handling**:
   `handle_rebuild_mesh` consumes `SurfaceTopology` and runs after `regenerate_surface_topology`. If adapter derivation fails on a non-empty map, it logs an ERROR and aborts rebuild without despawning visible terrain.
6. **Valid Empty Surface**:
   `SurfaceTopology::default()` projects to `Ok(TerrainTopology::default())`, allowing clear transitions on empty maps.
7. **Elevation-Only Rebuilds**:
   Modifying `TileData.elevation` without tile membership changes does NOT trigger `HexFaceTopology` or `SurfaceTopology` regeneration. Rebuild uses existing `SurfaceTopology` with updated Y positions calculated via `compute_vertex_heights`.
8. **Reference Oracle Retention**:
   The direct coarse adapter `derive_terrain_topology(&map_data, &face_topology)` in `topology_adapter.rs` is retained strictly as a test oracle for 144-case and 4,608-case equivalence proofs.
