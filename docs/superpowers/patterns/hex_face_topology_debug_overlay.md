# SOP: HexFaceTopology Debug Overlay

## Scope

`HexFaceTopology` is a diagnostic data path. It is not a replacement for the
production `TerrainTopology`, terrain mesh, Height3D surface, picking path, or
any gameplay system.

The runtime flow is:

```text
MapData + WorldSeed
        -> LogicalMapInputs fingerprint
        -> generate_hex_face_topology
        -> HexFaceTopology resource
        -> optional Gizmos overlay
```

When the overlay is disabled, no debug primitives are drawn and the existing
terrain rendering path is unchanged.

## Regeneration Contract

`FaceTopologyRuntimePlugin` runs in `GameSet::Visuals`, after map logic. It
rebuilds the topology when one of these conditions may have changed:

- initial resource state;
- `GenerateMapEvent` or `RebuildMeshEvent`;
- `MapData` change detection;
- `WorldSeed` change detection.

Before generation, the system compares `LogicalMapInputs`, consisting only of
map width, map height, world seed, and sorted `HexCoord` tile membership. Tile
content such as factions, resources, treasures, and terrain attributes does not
trigger regeneration when the logical map is unchanged.

The generated value is stored as the authoritative resource for the diagnostic
`HexFaceTopology` path. It is not authoritative for `TerrainTopology`, terrain
rendering, Height3D, picking, or gameplay. The debug cache is derived from that
same value. A failed generation never stores partial data. If the failed inputs
differ from the last successful map, the topology and cache are cleared; a
valid topology for the same logical map is retained. The failed fingerprint
prevents repeated per-frame error logs.

## Debug Controls

The overlay starts disabled:

| Key | Action |
|-----|--------|
| `F7` | Enable or disable all topology diagnostics |
| `F6` | Toggle canonical shared-vertex markers |
| `F5` | Toggle limited HalfEdge direction arrows |

Regular outlines and warped outlines default to enabled when `F7` is turned on.
Keyboard handling and drawing are gated by `GameState::Editing`; `F5`, `F6`,
and `F7` do nothing outside that state. The overlay is available only from
`EditorPhase::Shape` through `EditorPhase::Balance`. It is hidden from
`Height3D` and later phases.

## Geometry Sources

- Regular outlines use the existing ideal logical hex geometry.
- Warped outlines use `VertexId` and `MapVertex.position` from the stored
  `HexFaceTopology`; the drawing system never recomputes displacement.
- Shared markers are extracted once per canonical `VertexId`.
- Regular and warped segments are cached with one canonical undirected key per
  logical map edge. Regular edges use ordered `SharedCornerKey` pairs; warped
  edges use `(min(VertexId), max(VertexId))`. Internal borders are drawn once.
- HalfEdge arrows are sampled rather than drawn for all directed edges.

`HexFaceDebugCache` is rebuilt whenever the authoritative topology is replaced.
It contains only derived debug data and does not mutate `MapData` or topology.

## Validation

The debug helpers are pure and covered by unit tests for defaults, unique edge
counts, shared vertex identity, phase visibility, and non-mutation. Runtime
tests cover initial population, same-input no-op behavior, seed changes,
logical tile membership changes, content-only changes, and failure cleanup.

Native validation should capture disabled, wide enabled, close shared-border,
shared-vertex, and HalfEdge-arrow views. The normal UI remains visible in every
view.
