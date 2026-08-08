# SurfaceTopology Standards (SOP)

## Architectural Principle
`SurfaceTopology` is a derived, side-by-side semantic triangulated terrain surface layer derived **STRICTLY** from authoritative `HexFaceTopology`.

```
MapData
  │
  │ logical map membership (width, height, seed, tiles)
  ▼
HexFaceTopology
  │
  │ authoritative XZ + coarse topology (faces, vertices, half-edges)
  ▼
SurfaceTopology (M3)
```

## Key Invariants
1. **Decoupled Input**: `generate_surface_topology` accepts `&HexFaceTopology` only. It has zero dependency on `MapData`, `HEX_SIZE`, cliff edits, or elevation fields.
2. **Fixed24 Geometry Law**: Exactly 24 triangles per cell (1 center, 6 radial midpoints, 6 edge midpoints, 6 corners).
3. **Explicit Coarse Provenance**:
   - `HexCorner { source_vertex: VertexId }`
   - `HexEdgeMidpoint { source_a: VertexId, source_b: VertexId }` (with `source_a < source_b`)
   - `HexCenter { hex: HexCoord }`
   - `HexRadialMidpoint { hex: HexCoord, source_corner: VertexId }`
4. **Typed IDs on `usize`**: `SurfaceVertexId`, `SurfaceHalfEdgeId`, `SurfaceFaceId` wrap `usize` (`.index() -> usize`, `.new(usize)`).
5. **No Redundant Self-IDs**: `SurfaceFace` and `SurfaceHalfEdge` do not store redundant `id` fields. Vector position is authoritative.
6. **Global 2-Manifold Connectivity**: Every directed half-edge has `origin`, `destination`, `next`, `prev`, `incident_face`, and reciprocal `twin`. Edge incidence is strictly 1 (boundary) or 2 (manifold pair).
