# ADR 0008: Derived Height Constraint Graph & Cliff-Seam Height Domains

## Context
Milestone M4 introduced `HeightConstraintSet`, which compiles raw `MapData` intentions into surface-bound constraints (`RegionHeightConstraint` and `CliffHeightConstraint`). However, assigning height degrees of freedom directly to `SurfaceVertexId` fails at cliff seams: two sides of a cliff share the same XZ `SurfaceVertexId`, making vertical elevation drops impossible if a vertex possesses only a single Y value.

## Decision
We introduce `HeightConstraintGraph` (Milestone M4.1) as a pure derived combinatorial resource.
1. `SurfaceTopology` remains an un-mutated 2-manifold XZ surface model.
2. Height degrees of freedom are represented by `HeightNodeId`.
3. face corners (`surface.faces.len() * 3`) are partitioned into equivalence classes via DSU. Non-cliff reciprocal half-edges union face corners, while authored cliff seam half-edges block unioning.
4. As a result, a single `SurfaceVertexId` splits into multiple independent `HeightNodeId` values across cliff seams.
5. The model contains NO scalar Y values, floats, solver logic, or mesh alterations.

## Consequences
- Enables discrete elevation drop solving without altering surface 2D topology.
- Guarantees occurrence partition completeness and structural continuity across non-cliff faces.
- Prevents invalid 3D manifold topologies while providing discrete Y degrees of freedom.
