# ADR 0009: Deterministic Surface Height Layer & Height Solver (Milestone M5)

## Context
Prior to Milestone M5, height in Savage Fantasy was represented implicitly via per-tile scalar `TileData.elevation` or per-vertex calculations during mesh generation. This approach could not represent sharp vertical cliff discontinuities across split topological nodes.

Milestone M5 introduces `SurfaceHeightLayer`, storing one scalar normalized height (`f32` in domain `[0.0, 1.0]`) per split `HeightNodeId` derived via a deterministic Jacobi height solver (`SurfaceHeightSolver`).

## Decision

1. **Height Domain & Unit System**:
   - Hardcoded domain `[0.0, 1.0]` across all solver operations. `min_height` and `max_height` are removed from solver configuration.

2. **Side-by-Side Execution Policy**:
   - `SurfaceHeightLayer` runs purely side-by-side with zero changes to production rendering or mesh generation (`compute_vertex_heights()`, `TileData.elevation`, and `TerrainTopology` remain untouched).
   - No `RebuildMeshEvent` is dispatched by M5.

3. **Stage Pipeline & Typed Diagnostics**:
   - `derive_legacy_height_guide` -> `LegacyHeightGuide` (averages tile elevation per `HeightNodeId`; pins ocean to `0.0`).
   - `compile_height_targets` -> `HeightTargetField` (calculates preferred targets `{ target, weight }` per `HeightNodeId`).
   - `compile_hard_constraints` -> `CompiledHeightHardConstraints` (extracts DAG edges and interval bounds `[lower, upper]`).
   - `solve_surface_heights` -> `SurfaceHeightLayer` (Jacobi relaxation + topological hard-cliff projections).
   - `validate_surface_height_layer` -> verifies invariants and stats match.

4. **Determinism & Seam Resolution**:
   - Topological sorting uses `BTreeSet<HeightNodeId>` to guarantee canonical lowest node selection order.
   - Note: A numeric ordering may emerge across an unresolved seam from independent soft targets, but it is not persistent semantic resolution.

5. **Runtime Lifecycle & Fail-Closed**:
   - Strict no-retry policy: logical input fingerprints recorded before execution.
   - Transactional fail-closed: on any stage failure, all derived resources (`guide`, `targets`, `layer`) are cleared to default and failure state recorded.

## Consequences

- Standardized scalar heights attached to topological `HeightNodeId` ready for M5.1 rendering integration.
- Full 144-case canonical matrix and synthetic 4,608 matrix test coverage guarantees bit-exact determinism across platforms.
