# HeightConstraint Binding (Pattern SOP)

## Overview

`HeightConstraintSet` represents derived height intent bound directly to `SurfaceTopology` face and half-edge identities. Milestone M4 compiles persistent authoring intent (`LandscapeFeature`s and `EdgeData` cliffs) onto `SurfaceTopology` without computing Y positions, modifying meshes, or altering current height rendering.

## Architecture Pipeline

```
PERSISTENT AUTHORING (MapData)
 ├─ LandscapeFeature::{Mountain, Plateau, Lake, River}
 └─ EdgeData { EdgeType::Cliff, CliffLowerSide::{Unresolved, A, B} }
              │
              ▼
HEX TOPOLOGY (HexFaceTopology)
              │
              ▼
SEMANTIC SURFACE (SurfaceTopology)
              │
              ▼
compile_height_constraints(&MapData, &SurfaceTopology) -> HeightConstraintSet (Resource)
 ├─ RegionHeightConstraint (HexCoord -> SurfaceFaceId vector)
 └─ CliffHeightConstraint (EdgeCoord -> SurfaceBoundarySegment vector)
              │
              ▼
(Future M4.1 Constraint Graph & M5 SurfaceHeightSolver)
```

## Core Design & Binding Rules

1. **Pure Compiler API**:
   `compile_height_constraints` accepts **ONLY** `&MapData` and `&SurfaceTopology`. It does not accept or depend on `HexFaceTopology`, `TerrainTopology`, `BoundCliffEdges`, `SurfaceVertexSource`, `WorldSeed`, `TerrainConfig`, or `TileData.elevation`.
2. **1-to-1 Region Intent Mapping**:
   - `LandscapeFeature::Mountain` $\to$ `RegionHeightIntent::Mountain`
   - `LandscapeFeature::Plateau` $\to$ `RegionHeightIntent::Plateau`
   - `LandscapeFeature::Lake` $\to$ `RegionHeightIntent::Lake`
   - `LandscapeFeature::River` $\to$ `RegionHeightIntent::River`
   - `LandscapeFeature::None` $\to$ omitted from region constraints.
3. **Face Region Completeness**:
   `RegionHeightConstraint.faces` contains the exact complete set of `SurfaceFaceId`s matching `surface.hex_to_faces[&hex]`.
4. **Surface Half-Edge Twin Boundary Binding**:
   `CliffHeightConstraint.segments` binds logical `EdgeCoord`s using reciprocal surface half-edge twins (`a.twin == b`) where `incident_face` owner hexes match `edge.a` and `edge.b`.
5. **Cliff Lower-Side Preservation**:
   Authored `CliffLowerSide` (`Unresolved`, `A`, `B`) is preserved 1-to-1 without auto-resolution.
6. **Bidirectional Completeness Validation**:
   `validate_height_constraint_set` enforces an exact 1-to-1 match between authored regions/cliffs and published constraints, failing closed if any constraint is missing, unauthored, or incomplete.
7. **Semantic Fingerprinting & Retry Prevention**:
   `HeightConstraintLogicalInputs` caches authored features and cliffs. Recompilation is skipped for non-semantic `MapData` edits (e.g. elevation change) but triggers whenever `surface.is_changed() == true`. Failed compilations clear published constraints and do not loop-log without input changes.
8. **Elevation & Renderer Independence**:
   M4 computes ZERO Y heights, modifies NO terrain meshes, sends NO `RebuildMeshEvent`s, and does NOT alter current height rendering.
