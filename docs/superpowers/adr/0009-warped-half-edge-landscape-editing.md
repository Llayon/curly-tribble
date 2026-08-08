# ADR 0009: Warped Half-Edge Landscape Editing

## Status
Accepted (Milestone M2.1 CLOSED)

## Context
Legacy cliff editing used regular hexagonal midpoints (`coord.to_world(HEX_SIZE)`) and blind state cycling (`Unresolved` $\to$ `A` $\to$ `B` $\to$ `Unresolved`). As terrain generation evolved to use organic vertex warping and radial stabilization, visual cliff geometry diverged from regular hex midpoints, causing picking inaccuracies and ambiguous orientation.

## Decision
1. **Direct Half-Edge Pick Index**: derive `LandscapeEdgePickIndex` directly from `HexFaceTopology` internal twin pairs after topology generation in `GameSet::Visuals`.
2. **Side Classification Law**: use 2D cross-product signed area comparison of cursor position against exact face centroids `center_a` and `center_b`.
3. **Explicit Click Semantics**: LMB on Flat edge creates `Cliff(Unresolved)`; LMB on existing Cliff sets lower side to clicked face (`A` or `B`); RMB removes cliff.
4. **Vertex-Connected Stroke Propagation**: drag strokes propagate only along edges sharing `VertexId` endpoints, preventing distance-based false connects.
5. **Decoupled Lifecycle**: cliff edits mutate `MapData.edges` without emitting `RebuildMeshEvent` or regenerating topology; `rebuild_bound_cliff_edges` updates visual cliffs reactively.

## Consequences
- 100% geometric alignment between visual cliffs, picking geometry, and runtime half-edge twin bindings.
- Zero regular hex math or mesh rebuild overhead during cliff editing.
- Fully verified by 144-case matrix and architectural guard `test_cliff_edit_authoritative_topology_decoupling`.
