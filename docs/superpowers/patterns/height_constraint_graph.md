# Pattern: Derived Height Constraint Graph & Cliff-Seam Height Domains

## Standard Operating Procedure

### Overview
`HeightConstraintGraph` translates `SurfaceTopology` + `HeightConstraintSet` (M4) into discrete scalar height degrees of freedom (`HeightNodeId`).

### Key Laws
1. **XZ Surface Invariance**: `SurfaceTopology` 2D manifold geometry is NEVER modified by height graph generation.
2. **Cliff Seam Partitioning**: Non-cliff adjacent face corners share `HeightNodeId`. Authored cliff edges block unioning, splitting a `SurfaceVertexId` into separate `HeightNodeId` values on side A and side B.
3. **Purity & Decoupling**: No `f32`/`f64` height values, no solver logic, no direct `MapData`/`TileData` imports.
4. **M4 Outcome Gate**:
   - `Uninitialized` $\to$ No-op.
   - `Failure` $\to$ Clear graph resource and increment `failure_count`.
   - `Success` $\to$ Execute graph build and validation.

### Diagnostics Taxonomy
- `CollapsedCliffSample` (Info): Both sides of a cliff share the same `HeightNodeId`.
- `UnresolvedCliff` (Warning): Cliff lower side is `Unresolved`.
- `UnsplittableCliff` (Error): Topologically impossible split.
- `OpposedCliffOrdering` (Error): 2-node cycle (A < B and B < A).
- `DirectedCliffCycle` (Error): SCC cycle with $\ge 3$ nodes.
