# ADR 0004: Semantic SurfaceTopology Foundation

## Status
Accepted

## Context
Prior to Milestone M3, terrain mesh generation relied on a direct conversion adapter (`derive_terrain_topology`) from coarse `HexFaceTopology` to an un-indexed rendering representation (`TerrainTopology`). To support future scalar heightfields, continuous editing, and advanced landform semantics without modifying production rendering prematurely, a strongly-typed semantic surface model was required.

## Decision
1. Introduce `SurfaceTopology` as a side-by-side derived resource generated strictly from `&HexFaceTopology`.
2. Standardize typed IDs (`SurfaceVertexId`, `SurfaceHalfEdgeId`, `SurfaceFaceId`) on `usize`.
3. Model explicit coarse vertex provenance (`HexCorner`, `HexEdgeMidpoint`, `HexCenter`, `HexRadialMidpoint`) linking surface vertices back to coarse `VertexId`s.
4. Build a global 2-manifold half-edge mesh with reciprocal twin connectivity.
5. Defer production rendering switch to Milestone M3.1. Direct adapter `derive_terrain_topology` remains active for production rendering in M3.

## Consequences
- Clean separation of coarse planar topology, surface triangulation, elevation fields, and rendering overlays.
- Zero risk to production rendering during M3.
- Proven bit-exact compatibility with direct adapter outputs.
