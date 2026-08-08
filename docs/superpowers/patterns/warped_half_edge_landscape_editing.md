# SOP: Warped Half-Edge Landscape Editing

## Overview
Authoritative terrain geometry and visual mesh representation in Savage Fantasy derive directly from `HexFaceTopology` (warped micro-vertices, organic noise displacement, radial stabilization).
Milestone M2.1 removes all regular hexagonal distance approximations, midpoint calculations, and `EdgeDirection` assumptions from cliff picking and authoring.

## Architectural Mandates

1. **Authoritative Pick Index (`LandscapeEdgePickIndex`)**:
   - Derived exclusively in `GameSet::Visuals` after `regenerate_hex_face_topology`.
   - Filters to internal reciprocal `HalfEdge` pairs with `Some(twin)`.
   - Maps `face_a.hex == edge.a` and `face_b.hex == edge.b`.
   - Extracts exact 3D vertex XZ positions `segment_start` and `segment_end`.
   - Computes arithmetic face centroids `center_a` and `center_b`.
   - Sorts edges deterministically by `(edge.a, edge.b)`.

2. **Warped Point-to-Segment Selection (`pick_landscape_edge`)**:
   - Uses exact point-to-segment distance algorithm with `CLIFF_PICK_RADIUS_RATIO = 0.25` of edge segment length.
   - Calculates 2D cross-product signed areas relative to `center_a` and `center_b` to classify `LogicalEdgeSide::A` vs `LogicalEdgeSide::B`.
   - Tie-breaks candidates by `distance_squared` (via `f32::total_cmp`) and `(edge.a, edge.b)`.

3. **Clicked Lower-Side Orientation (`apply_single_cliff_click`)**:
   - Initial click on Flat edge $\to$ `EdgeType::Cliff` with `CliffLowerSide::Unresolved` (`⇐ | ⇒`).
   - Click on existing Cliff edge $\to$ side-aware orientation (`Side::A` $\to$ `CliffLowerSide::A`, `Side::B` $\to$ `CliffLowerSide::B`).
   - Removes blind cycling (`Unresolved` $\to$ `A` $\to$ `B` $\to$ `Unresolved`).
   - RMB click $\to$ removes cliff from `MapData.edges`.

4. **Connected Stroke Editing (`CliffStrokeState`)**:
   - Stroke start determines mode (`PaintUnresolved`, `OrientExisting`, or `Erase`).
   - Connectivity propagation strictly checks `VertexId` identity sharing (`hit.vertices` sharing at least one `VertexId` with `previous_accepted_vertices`).
   - Deduplicates visited edges via `visited_edges` set.
   - Resets state on mouse release, phase exit, or tool change.

5. **No `RebuildMeshEvent` for Cliff Editing**:
   - Modifying `MapData.edges` during cliff authoring updates `BoundCliffEdges` in `GameSet::Visuals` without triggering topology regeneration or emitting `RebuildMeshEvent`.
