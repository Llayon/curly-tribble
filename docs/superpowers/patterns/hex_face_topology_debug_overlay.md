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

`LogicalMapInputs` also includes the selected `HexDeformationProfile`, so a
profile switch is detected as a fingerprint change and regenerates the
diagnostic topology exactly once without altering `MapData` or `WorldSeed`.

## Debug Controls

The overlay starts disabled:

| Key | Action |
|-----|--------|
| `F7` | Enable or disable all topology diagnostics |
| `F6` | Toggle canonical shared-vertex markers |
| `F5` | Toggle limited HalfEdge direction arrows |
| `F8` | Cycle experimental deformation profile (`Subtle` -> `Organic` -> `PagoniaLike`) |

Regular and warped outlines are enabled and disabled together. Keyboard
handling and drawing are gated by `GameState::Editing`; `F5`, `F6`, `F7`, and
`F8` do nothing outside that state. The overlay is available only from
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

## Compatibility and Determinism Lock

Golden connectivity and geometry fingerprints are literal constants in
`tests_compat.rs`, extracted by running the identical fingerprint
implementation in a detached worktree at commit `158e5f2` (the pre-profile
Subtle output). The canonical fixtures are:

| Fixture | Connectivity | Geometry |
|---------|--------------|----------|
| 40x40 seed 42 | `ced2a6625361af97` | `2c69358d1bde2489` |
| 40x40 seed 99 | `4204f1084ab83e7c` | `3222156361ed2849` |
| L-shape seed 42 | `9ed9d5c5d7b6c2ab` | `5575c4b2e0910e73` |
| Diagonal seed 7 | `c6f10fe0442b2820` | `bb217bd74b1b3c45` |
| Seven-hex seed 42 | `91c095c7e82cee27` | `2531f9b9a3b17f8f` |

The stable hash is project-owned FNV-1a 64-bit over big-endian fields: map
dimensions, `WorldSeed`, ascending `VertexId`s with `f32::to_bits()` positions,
sorted face cycles, and half-edge endpoints/twins/ownership. It has no
`usize`-width dependence, no `HashMap` iteration order, and excludes diagnostic
`acos`-based metrics. A seed scan at `158e5f2` found no reduction/fallback case,
so backoff compatibility is documented as untested, not asserted.

## Acceptance Criteria

`ProfileAcceptanceCriteria` (in `acceptance.rs`) centralizes measured-output
thresholds, distinct from the generator's input config (component magnitude
ranges are an input range, not a final displacement guarantee). Generation
fails hard if the measured final displacement exceeds the profile's absolute
cap (ratio of `HEX_SIZE`, 1e-3 tolerance). Visual misses emit a single WARN at
regeneration time and fail the canonical 40x40 fixture tests; they never affect
production terrain.

## Experimental Profile Validation

Profiles are validated with unit tests that require:

- `Subtle` is the default and remains bit-compatible with the pre-profile
  warped output (existing golden displacement vectors unchanged).
- Per-vertex and combined displacement vectors for `Organic` and `PagoniaLike`
  match exact `f32::to_bits()` golden references (determinism lock).
- The correlated field is spatially related, non-constant, and handles negative
  coordinates deterministically.
- Topology identity (face/vertex/half-edge/paired/border counts) and map data
  are preserved across all profiles, and a profile change regenerates exactly
  once without touching map data.

Run the full 256-seed, three-profile stress suite (4,608 topologies) explicitly:

```text
cargo test --lib full_hex_deformation_profiles_stress_256_seeds -- --ignored
```

The fast loop that runs with the normal suite is:

```text
cargo test --lib fast_profile_stress_covers_all_profiles_and_shapes
```
