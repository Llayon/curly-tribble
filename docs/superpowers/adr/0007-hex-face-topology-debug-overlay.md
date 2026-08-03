# ADR 0007: HexFaceTopology Diagnostic Overlay

* **Статус**: Accepted
* **Дата**: 2026-08-02
* **Автор**: Savage Fantasy Agent

## Контекст (Context)

`HexFaceTopology` has a deterministic, fully validated data representation,
but it was not possible to compare its warped boundaries with the existing
regular logical hexes in the running editor. The diagnostic must not become a
second production geometry path or alter terrain rendering, Height3D, picking,
or gameplay.

## Решение (Decision)

Add a dedicated runtime/debug layer under `src/map/face_topology/`:

1. Regenerate the authoritative resource for the diagnostic `HexFaceTopology`
   path from `MapData` and `WorldSeed` only when a logical-input fingerprint
   changes or an existing map event requests reconsideration. It is not
   authoritative for `TerrainTopology`, terrain rendering, picking, or gameplay.
2. Derive cached regular and warped unique-edge lists plus a shared-vertex list
   from one logical map/topology pair. Each cache contains one segment per
   logical map edge.
3. Draw optional immediate-mode Gizmos only through `EditorPhase::Balance`.
4. Run keyboard handling and drawing only in `GameState::Editing`. Keep the
   overlay disabled by default, with `F7` as the master toggle, `F6` for shared
   vertices, and `F5` for sampled HalfEdge arrows.
5. Clear stale data on failed generation for a changed logical map and retain a
   valid result when the failed inputs are unchanged.
6. Use `F8` (in `GameState::Editing`) to cycle the experimental deformation
   profile among `Subtle`, `Organic`, and `PagoniaLike`; a profile change
   regenerates the diagnostic topology once and is logged.

## Эксперимент (Experimental Deformation Profiles)

The diagnostic path also supports an experimental, visually comparative set of
deterministic deformation profiles. This is diagnostic only and is **not**
validated, tuned, or integrated into production terrain, Height3D, picking, or
gameplay.

- `Subtle` (the default, bit-compatible with the pre-existing warped output):
  a purely local, per-corner fixed-direction displacement.
- `Organic`: blends the local displacement with a low-frequency correlated
  field (~65/35 correlated/local weight, macro-cell span of 5 hexes). Each
  macro vector comes from a stable seeded node hash combined with the immutable
  fixed Q15 direction table. The profile is deliberately kept small enough that
  nearly every face stays a valid convex polygon without backoff.
- `PagoniaLike`: a stronger correlated variant (~75/25 weight) meant to
  approximate a larger-cell, more continuous look reminiscent of that title's
  map aesthetics. This is a **visual experiment only**; it makes no claim to
  reproduce any actual Pagonia data structure, algorithm, or asset.

The deformation field is **independent of the camera**:

- Field geometry does not read the active camera entity or any camera resource.
- Changing camera position, rotation, or zoom cannot affect topology.
- Macro vectors come from stable integer-coordinate hashing of the seeded
  node index and the profile discriminator.
- Direction selection uses the immutable fixed `DISPLACEMENT_DIRECTIONS_Q15`
  direction table at field build time.
- Interpolation uses deterministic fixed-point arithmetic (Q16), not
  floating-point rounding that depends on runtime order.

All three profiles share the same deterministic top-bottom seed, rely on
integer derived fields (no runtime `sin`/`cos`, no `StdRng`, no
`HashMap`-order dependence, no camera resource reads), and undergo the same
simultaneous backoff loop to guarantee valid convex geometry. A canonical
64-bit FNV-1a fingerprint of the geometry and connectivity is used to lock
golden fixtures against commit `158e5f2`. An explicit per-vertex and combined
set of golden displacement vectors locks the correlated field and displacement
to exact `f32::to_bits()` references.

## Контракт приёмки (Acceptance Contract)

Measured final output is checked against centralized criteria in
`src/map/face_topology/acceptance.rs` (`ProfileAcceptanceCriteria`), which are
deliberately distinct from the generator *inputs* (component magnitude ranges,
weights, macro span, and the Q16 absolute cap in the profile config). The
component magnitude fields are an **input range, not a final displacement
guarantee**; acceptance is judged on observed statistics only.

- **Hard cap (generation failure)**: the measured `max_displacement` after the
  backoff loop must not exceed `absolute_displacement_cap_ratio * HEX_SIZE`
  (plus a 1e-3 relative tolerance). Violation returns
  `HexFaceTopologyError::ProfileDisplacementCapExceeded`.
- **Visual targets (warning + canonical-fixture test)**: average displacement
  ratio band, minimum edge length ratio, interior-angle range, minimum aspect
  quality, reduced-vertex ratio, and regular-fallback ratio. A miss emits one
  `WARN` event at regeneration time and fails the canonical 40x40 fixture test,
  but never affects production terrain.
- Per-profile targets: `Subtle` {avg 0.070–0.120, max 0.160, edge ≥0.550,
  angles 80–155°, aspect ≥0.550, reduced ≤0.150, fallback ≤0.150};
  `Organic` {avg 0.050–0.140, max 0.220, edge ≥0.500, aspect ≥0.500};
  `PagoniaLike` {avg 0.050–0.200, max 0.280, edge ≥0.500, aspect ≥0.500,
  reduced ≤0.200}. Ratios are relative to `HEX_SIZE`.

Compatibility is claimed only where it was measured: the five golden fixtures
above were extracted by running the identical fingerprint implementation in a
detached worktree at `158e5f2`, and a seed scan at that commit found **no**
reduction/fallback case, so backoff compatibility is documented as untested
rather than asserted.

## Обоснование (Rationale)

The fingerprint avoids cloning or regenerating the topology every frame while
still detecting in-place map edits. Sorting tile coordinates makes the trigger
independent of HashMap iteration order. Deriving diagnostics from stored
`VertexId` values guarantees that both incident faces use identical shared
border positions. Canonical `SharedCornerKey` pairs apply the same one-edge
rule to regular outlines. Immediate Gizmos avoid persistent ECS entities and
preserve the production renderer. `HexFaceTopology` remains diagnostic and is
not authoritative for production terrain, Height3D, picking, or gameplay.

## Последствия (Consequences)

* **Положительные**: Deterministic topology can be inspected in-place; shared
  borders and vertices are visually auditable; normal visuals remain unchanged
  while debug mode is off.
* **Отрицательные**: The overlay adds a small runtime resource/cache and
  requires native visual checks in addition to unit tests.
* **Нейтральные**: The diagnostic is intentionally hidden after Balance and
  does not project warped vertices onto Height3D.
