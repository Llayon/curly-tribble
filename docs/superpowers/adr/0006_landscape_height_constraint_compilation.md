# ADR 0006: Landscape Height Constraint Compilation

## Status
Accepted (Milestone M4)

## Context
Following Milestone M3 (Semantic SurfaceTopology Foundation) and Milestone M3.1 (Production Terrain Bridge), persistent authoring intent (`MapData`'s `LandscapeFeature`s and `EdgeData` cliffs) needs to be compiled onto `SurfaceTopology` face and half-edge identities to prepare for constraint graph construction (M4.1) and surface height solving (M5).

## Decision
1. Create `src/map/height_constraints/` defining `HeightConstraintSet`, `RegionHeightConstraint`, `CliffHeightConstraint`, and `SurfaceBoundarySegment`.
2. Implement pure, total `compile_height_constraints(&MapData, &SurfaceTopology)` deriving region and cliff constraints directly on `SurfaceTopology` face and half-edge identities.
3. Bind cliff segments via reciprocal surface half-edge twins matching `EdgeCoord` cell owners without querying `BoundCliffEdges` or `HexFaceTopology`.
4. Implement 2-way completeness validation in `validate_height_constraint_set` proving exact 1-to-1 equivalence between authored intent and derived constraint sets.
5. Implement semantic input fingerprinting (`HeightConstraintLogicalInputs`) in `regenerate_height_constraints` to prevent unnecessary recompiles on elevation or non-semantic edits and stop retry loops on invalid inputs.
6. Enforce zero Y calculation, zero mesh modification, zero `RebuildMeshEvent`, and zero change to production height rendering.
7. Lock architectural decoupling via `test_height_constraints_decoupling` forbidding `TerrainTopology`, `derive_terrain_topology`, `compute_vertex_heights`, `MAX_HEIGHT`, `.elevation`, `HEX_SIZE`, `to_world(`, `face_topology`, `HexFaceTopology`, `BoundCliffEdges`, `SurfaceVertexSource`, `RebuildMeshEvent`.

## Consequences
- **Positive**:
  - Persistent authoring intent is bound directly to 2-manifold `SurfaceTopology` face and boundary identities.
  - Complete 1-to-1 bidirectional validation proven on 144 canonical and 4,608 extended matrix cases.
  - Decoupled from legacy coarse topology and rendering models.
  - Zero regression in height rendering, water overlays, roof overlays, cliff editing, or map files.
- **Negative**:
  - Constraints remain unconsumed until Milestone M4.1 (Constraint Graph) and Milestone M5 (SurfaceHeightSolver).
