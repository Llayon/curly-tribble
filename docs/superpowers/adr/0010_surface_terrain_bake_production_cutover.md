# ADR 0010: Surface Terrain Bake Production Cutover (Milestone M5.1)

## Context

Milestone M5 introduced the deterministic height domain (`SurfaceHeightLayer`,
`HeightConstraintGraph`, `SurfaceTopology`) but kept production terrain
rendering on the legacy path: `SpawnGlobalTerrainCommand` accepted an optional
`SurfaceTerrainBake` and fell back to `create_global_map_meshes` (per-tile
`TileData.elevation` meshing) whenever the bake was absent. M5.1's bake
(`SurfaceTerrainBake`, `TerrainBakePlugin`) produces the authoritative ground
geometry — vertices from split `HeightNodeId`s, cliff wall quads from
`CliffWallSegment`s, and bit-exact XZ positions from `SurfaceTopology`.

Leaving the legacy fallback in production created a silent dual-path hazard:
any code path that failed to supply a bake would silently render the old
elevation-based terrain, diverging from the validated height domain and
breaking the M5/M5.1 invariants (split cliffs, bit-exact heights).

## Decision

1. **Bake is mandatory in production**:
   - `SpawnGlobalTerrainCommand.bake` is a required `SurfaceTerrainBake` field;
     the `Option` and the `create_global_map_meshes` fallback are removed from
     production code. The legacy generator remains only as test-only reference
     (bit-regression oracle).
   - `create_global_map_meshes_from_bake` is the single production entry point.

2. **Fail-closed rebuild gating**:
   - `handle_rebuild_mesh` runs only when `TerrainBakeGenerationState.last_outcome
     == TerrainBakeGenerationOutcome::Success`. On any other state the rebuild
     is skipped and the previously rendered terrain stays intact.
   - `SurfaceTopology` and the old despawn-everything cycle are removed from
     `handle_rebuild_mesh`; topology flows exclusively through
     `derive_terrain_topology_from_bake`.

3. **Transactional visual swap**:
   - The command generates all meshes first (pure, no world mutation). On
     failure it logs and returns, leaving old assets and visuals untouched.
   - Only after successful generation does it retire old mesh assets
     (`Assets<Mesh>::remove`), despawn old `MapVisualEntity` instances, and
     spawn the new `GlobalTerrainBundle` / `WaterBundle` / `MountainRoofBundle`.

4. **Independent validation** (`validate_surface_terrain_bake`):
   - Structural length checks (surface faces vs graph face_nodes, heights vs
     graph nodes, bake vertices vs graph nodes, bake faces vs surface faces).
   - Strict per-vertex checks: node index identity, surface-vertex identity,
     bit-exact XZ position, normalized height bit-exact and in `[0.0, 1.0]`,
     owner-hex set built strictly from incident faces (no silent skipping).
   - Exact cliff-wall set equality (not just counts) plus wall node range
     checks; `split_surface_vertex_count` recomputed independently.

5. **Architecture enforcement**:
   - The M3.1 guard `test_production_terrain_routes_through_surface_topology`
     is inverted and replaced with `test_production_terrain_routes_through_height_bake`.
   - New guards: `test_production_terrain_command_is_bake_only`
     (mandatory bake, no legacy call) and `test_terrain_bake_core_decoupling`
     (terrain_bake core files forbidden from touching `MapData`, `TileData`,
     `TerrainConfig`, legacy topology, `MAX_HEIGHT`, `compute_vertex_heights`,
     `TerrainHeightMode`, `.elevation`, `create_global_map_meshes`,
     `SpawnGlobalTerrainCommand`, `EditorPhase`).

6. **Test proof of production equivalence**:
   - Wall rendering: full cliff → 2 triangles, taper → 1, equal heights → 0,
     flat phase → 0; relief Y scales by `MAX_HEIGHT`.
   - `production_40x40_bake_smoke` now flows through
     `create_global_map_meshes_from_bake` with mesh invariants (finite
     positions/normals, ground indices == face node ids, wall indices above
     ground vertex count).
   - `bake_path_water_roof_bit_matches_legacy` proves water/roof meshes are
     bit-identical between the legacy generator and the bake path.
   - Negative validator tests: tampered split count, cleared owner hexes,
     dropped wall segment, truncated heights each yield the typed error.

## Consequences

- Production rendering is driven exclusively by the validated height domain;
  silent drift back to elevation-based terrain is impossible (enforced by
  architecture guards).
- Rebuilds are atomic from the player's perspective: a failed bake leaves the
  previous terrain fully rendered; only a `Success` outcome triggers a swap.
- The legacy `create_global_map_meshes` survives solely as a test oracle for
  bit-regression, keeping the M3.1 pixel parity promise checkable.
- `TerrainConfigFingerprint`-based rebuild suppression (from the preceding
  commit) is unchanged; this ADR only completes the production cutover.