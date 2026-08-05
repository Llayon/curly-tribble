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

## Blend Reliability Floor (near-zero direction stabilization)

The law lives in `blend_policy.rs` (a dependency leaf: activation mode,
threshold ratio, preference margin; comparison cross-multiplied in `i128` so
the boundary is never rounded); the shared arithmetic lives in
`blend_diagnostics.rs`; `blend.rs` combines them. The weight decides the
direction, the stronger component decides the magnitude (see ADR 0007). The
weighted *sum* can still near-cancel even when its anti-parallel components
both have mass; those corners are oriented only by integer rounding noise. The
reliability floor corrects **only** them:

- `MIN_RELIABLE_DIRECTION_RATIO_Q16 = 1/64`: corners above the ratio keep the
  raw integer normalization bit for bit (locked by
  `reliable_directions_are_bit_identical_to_the_raw_law`); unreliable corners
  are projected onto the *reference* component until the projection reaches the
  floor, then normalized as before. The floor is an **open** upper bound:
  exactly `floor`, `floor+1`, `floor+2` stay raw, only `floor-1`/`floor-2`
  correct (`tests_blend_boundary.rs`).
- `blend_reference` picks the larger **weighted** component, resolving
  near-ties toward the coherent correlated field via
  `CORRELATED_PREFERENCE_MARGIN_Q16` (1/8). At a 1/64 floor every stabilized
  corner on the canonical 256-seed matrix picks `Correlated` (`local_ref = 0`).
- The production surface is locked by a two-level baseline contract
  (`tests_blend_candidate_geometry.rs`): public entry points == explicit
  production-policy pipeline (fast matrix), which in turn matches the literal
  `9ad12ae` geometry/connectivity fingerprints for `Organic`/`PagoniaLike` at
  seeds 42 and 194.
- Invariants enforced on the fast seeds:
  `stabilized_directions_keep_a_minimum_projection_onto_the_reference` (target
  magnitude preserved, corrected length at the floor) and
  `adjacent_displacement_direction_audit_on_canonical_map` (worst
  both-stabilized adjacent dot `>= -0.1`; the pre-existing global `-1.0`
  extremes are non-near-zero local flips and must never regress).
- `near_zero_blend_direction_fixtures_lock_the_weakest_measured_cases` freezes
  the weakest measured corners (Pago seed 194 weighted length 1, magnitude
  5/65536; Organic seed 64 length 8). Their raw weighted values must never
  change; their stabilized resolution is deterministic and exact.

Threshold maintenance — re-measure candidate floors 1/64, 1/32, 1/16 in both
activation modes (raw, length, projection) whenever weights or component
magnitude ranges change. Every candidate is generated through its own pipeline
(`generate_hex_face_topology_with_profile_and_policy`), so the table is an
honest generator comparison, never a re-classification:

```text
cargo test --lib full_blend_reliability_candidate_geometry_256_seeds -- --ignored
cargo test --lib full_candidate_adjacency_256_seeds -- --ignored
```

Baseline on the canonical 40x40 across 256 seeds (860,160 corners per row):
production 1/64-length stabilizes 618 (`Organic`) / 500 (`PagoniaLike`)
corners (~0.07%) with worst both-stabilized adjacent dot `−0.041` (Pago
`+0.738`), min stabilized ratio 995/1004; 1/32-length stabilizes 2,338/1,993
with no both-stabilized anti-parallel pair (`+0.45`); 1/16-length and every
projection-mode floor produce both-stabilized dots down to `−1.0` and are
rejected. The documented near-antiparallel tolerance band is
`NEAR_ANTIPARALLEL_DOT_THRESHOLD = −0.9995`; an exact `-1.0` is separately
detectable via `to_bits()`. The old 1/32 dot of `−0.986` was a re-classification
artifact; the honest value is `+0.45`. Production stays on 1/64-length
(report-only decision).

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
ranges are an input range, not a final displacement guarantee). Acceptance is
layered:

- **Level A (hard)**: all measured metrics finite; measured final displacement
  must not exceed the profile's absolute cap (single source: the profile config
  Q16 cap; `validate_profile_displacement_cap` allows the cap plus one
  `DISPLACEMENT_CAP_EPSILON`, rejects non-finite values; generation failure).
- **Level B (canonical bands)**: visual misses emit one combined WARN at
  regeneration time (stable issue ordering, profile name + geometry
  fingerprint) and fail the canonical 40x40 fixture tests; never production.
  The `Organic`/`PagoniaLike` bands are the minimum relaxation supported by the
  full 256-seed x six-shape matrix (4,608 topologies); every worst case occurs
  on the canonical 40x40 map and is locked as a fixture in `tests_quality.rs`
  (`Organic` seed 203: 161.11° max angle, seed 74: 0.4937 aspect;
  `PagoniaLike` seed 58: 175.195° max angle, seed 169: 0.3783 aspect). Bands
  hold a small margin beyond these: `Organic` 162°/0.490, `PagoniaLike`
  176°/0.370. The bands intentionally permit the measured near-flat/slender
  extremes and are not a production-readiness claim.
- **Level C (separation contract)**: `ProfileSeparationCriteria` requires
  `avg(Organic) >= avg(Subtle) + 0.015` and `avg(Pago) >= avg(Organic) + 0.015`
  on the canonical 40x40. **Fulfilled for every seed** (8 fast seeds: `Subtle`
  ≈0.100, `Organic` ≈0.151, `PagoniaLike` ≈0.201 — both gaps clear the 0.015
  floor by ~0.035; the 256-seed sweep reports a min gap of ~0.049). Enforced
  by `check_profile_separation` in
  `canonical_40x40_separation_satisfied_per_seed` and the ignored
  `full_canonical_profile_separation_stress_256_seeds`; the blend law that
  makes the separation achievable is documented in ADR 0007
  (`src/map/face_topology/blend.rs`).

## Experimental Profile Validation

Profiles are validated with unit tests that require:

- `Subtle` is the default and preserves the legacy candidate corner-displacement
  function; its geometry and connectivity are bit-compatible with `158e5f2` for
  the recorded golden fixtures only (universal legacy compatibility not claimed).
  The pre-profile golden displacement vectors remain unchanged.
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

Run the full 4,608 acceptance scan (asserts the corrected Level B bands and
zero backoff across the whole matrix) explicitly:

```text
cargo test --lib full_4608_quality_extrema_scan -- --ignored
```

The fast loops that run with the normal suite are:

```text
cargo test --lib fast_profile_stress_covers_all_profiles_and_shapes
cargo test --lib fast_144_topology_matrix_is_fully_hardened
```
