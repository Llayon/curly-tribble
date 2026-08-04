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

- `Subtle` (the default): preserves the legacy candidate corner-displacement
  function. Its geometry and connectivity are bit-compatible with `158e5f2` for
  the recorded golden fixtures only; universal legacy safety-reduction
  compatibility is **not** claimed (the legacy scan found no reduction case).
  It is a purely local, per-corner fixed-direction displacement.
- `Organic`: blends the local displacement with a low-frequency correlated
  field (~65/35 correlated/local weight, macro-cell span of 5 hexes). Each
  macro vector comes from a stable seeded node hash combined with the immutable
  fixed Q15 direction table. The profile is deliberately kept small enough that
  nearly every face stays a valid convex polygon without backoff.
- `PagoniaLike`: a stronger correlated variant (~75/25 weight) meant to
  approximate a larger-cell, more continuous look reminiscent of that title's
  map aesthetics. This is a **visual experiment only**; it makes no claim to
  reproduce any actual Pagonia data structure, algorithm, or asset.

The two non-Subtle profiles combine their components with a deterministic
**magnitude-preserving blend** (`src/map/face_topology/blend.rs`): the profile
weights decide the *direction* (weighted vector sum normalized in Q24 fixed
point, integer square root — no floating point before the final `Vec2`), while
the *magnitude* is the strongest component length
(`max(|correlated|, |local|)`). Because the magnitude never comes from a vector
*sum*, anti-parallel components cannot cancel the result: the old naive blend
squashed the average (half the samples were anti-aligned, so
`avg(|wc*C + wl*L|)` fell to ≈0.076 for `Organic`, *below* `Subtle`'s ≈0.100).
Closing that gap with input-magnitude/weight tuning alone would push aligned
vertices toward the hard cap, so the blend law was changed instead; parameter
tuning was measured and rejected (Option A), see the rationale below.

### Near-zero direction stabilization (the blend reliability floor)

Because magnitude is immune to anti-parallel cancellation, the weighted *sum*
can still almost cancel even when its direction is real — and when it does the
residual is dominated by integer rounding noise, so the normalized direction
flips arbitrarily between adjacent corners. The 256-seed canonical scan found
adjacent dot products of exactly `-1.0` on `Organic`/`PagoniaLike` (e.g. seed 42
Pago edge 7744 at ratio ≈19193/18967, seed 64 Organic edge 7296 at ~40527/6224)
even though the nearest-stabilizable corners are at ratios as low as 59; those
extreme flips are non-near-zero local noise and cannot be removed without
changing profile weights. The fix (`blend.rs` + `blend_diagnostics.rs`):

- A corner is *unreliable* when its weighted length is below
  `MIN_RELIABLE_DIRECTION_RATIO_Q16` (1/64) of the target magnitude. Only
  those corners change direction; all reliable corners keep the exact previous
  integer arithmetic, bit for bit (so `Subtle`, whose geometry never
  near-cancels, is byte-identical).
- Unreliable corners are projected onto a continuous *reference* component —
  the larger **weighted** magnitude, with near-ties resolved toward the
  coherent correlated field via `CORRELATED_PREFERENCE_MARGIN_Q16` (1/8 band,
  which routes every stabilized corner to `Correlated`; `local_ref = 0` at
  1/64) — extending the weighted vector along the reference until its
  projection reaches the floor, then normalizing as before.
- Measured across 256 canonical seeds: 1/64 stabilizes only 618 (`Organic`) and
  500 (`PagoniaLike`) corners (~0.07%), all to ≥0.995 of the target magnitude,
  and the worst *both-stabilized* adjacent dot is `-0.041`, safely above the
  `-0.1` anti-parallel band. Raising the floor to 1/32 degrades that worst dot
  to `-0.986` and 1/16 to `-1.0` (with 830/1307 Local references), which is why
  1/64 is the smallest sufficient threshold. Both lines above are also flips
  below a 1/16 floor and into measurable stabilization totals (2338/1993 and
  9224/7702 respectively).

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
`src/map/face_topology/acceptance.rs`, deliberately distinct from the generator
*inputs* (component magnitude ranges, weights, macro span, and the Q16 absolute
cap in the profile config). The component magnitude fields are an **input
range, not a final displacement guarantee**; acceptance is judged on observed
statistics only. Acceptance is layered into three documented levels:

- **Level A — hard correctness (generation failure or test-failure)**: all
  measured metrics must be finite (typed `NonFiniteMetric` issues); the
  measured `max_displacement` after the backoff loop must not exceed the
  profile's absolute cap. The cap lives **once** in the profile config
  (`absolute_displacement_cap_ratio`, derived from each profile's Q16 value);
  `validate_profile_displacement_cap` accepts the cap itself plus one
  `DISPLACEMENT_CAP_EPSILON` (1e-3) of slack and rejects anything above — and
  every non-finite value. Violations return
  `HexFaceTopologyError::ProfileDisplacementCapExceeded` /
  `ProfileDisplacementNotFinite`.
- **Level B (canonical visual bands, warning + fixture test)**: average
  displacement ratio band, minimum edge length ratio, interior-angle range,
  minimum aspect quality, reduced-vertex ratio, and regular-fallback ratio. A
  miss emits **one combined `WARN` at regeneration time** (stable issue
  ordering: non-finite metric issues first, then the fixed band order, with the
  profile name and geometry fingerprint for reproduction) and fails the
  canonical 40x40 fixture test, but never affects production terrain.
- **Level B band derivation (measured, not heuristic)**: the `Organic` and
  `PagoniaLike` bands are the **minimum relaxation supported by the full
  256-seed x six-shape matrix** (4,608 topologies), not values chosen to pass
  the 8 fast-seed matrix. Every worst case in that matrix occurs on the
  canonical 40x40 map: `Organic` seed 203 reaches 161.11° max interior angle
  and seed 74 drops to 0.4937 aspect; `PagoniaLike` seed 58 reaches 175.195°
  and seed 169 drops to 0.3783 aspect. The documented bands sit a small margin
  beyond those extrema (162°/0.490 and 176°/0.370 respectively). The old
  pre-tuning limits (Organic 155°/0.500; Pago 80–155°/0.500) fail **hundreds**
  of canonical cases (258 and 340 respectively across the matrix), so they are
  not recoverable without weakening the geometry itself. Locked worst-case
  fixtures per extreme live in `tests_quality.rs`, and the strengthened 4,608
  stress tier runs under the corrected bands. This document does **not** claim
  the profiles are production-ready: the bands intentionally permit the
  measured near-flat (176°) and slender (0.370 aspect) extremes and simply make
  that boundary explicit for human review rather than letting it happen
  silently.
- **Level C (profile separation contract)**: documented minimum average
  displacement gaps between consecutive profiles
  (`ProfileSeparationCriteria`, both gaps 0.015 of `HEX_SIZE`). `Organic` must
  average at least 0.015 above `Subtle`, and `PagoniaLike` at least 0.015 above
  `Organic`. On the canonical 40x40 the contract now **passes for every seed**:
  measured averages (8 fast seeds) are `Subtle` 0.099–0.100, `Organic`
  0.150–0.152, `PagoniaLike` 0.200–0.202 — each gap clears the 0.015 floor by
  ~0.035. The 256-seed canonical separation sweep reports a minimum gap of
  0.04887 across all seeds (`Subtle`→`Organic` 0.04978 at seed 206,
  `Organic`→`PagoniaLike` 0.04887 at seed 17). `Organic` and `PagoniaLike`
  remain experimental; separation is checked per seed in the per-seed canonical
  test plus the full 256-seed sweep.

- Per-profile visual targets: `Subtle` {avg 0.070–0.120, edge ≥0.550,
  angles 80–155°, aspect ≥0.550, reduced ≤0.150, fallback ≤0.150};
  `Organic` {avg 0.110–0.175, edge ≥0.500, angles 80–162°, aspect ≥0.490};
  `PagoniaLike` {avg 0.150–0.235, edge ≥0.500, angles 75–176°, aspect ≥0.370,
  reduced ≤0.200}.
  Hard caps: `Subtle` 0.160, `Organic` 0.220, `PagoniaLike` 0.280.
  Ratios are relative to `HEX_SIZE`.

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

The separation contract was originally missed because the naive blend
(`wc*C + wl*L`, then the vector length) measures a weighted *sum*: with the
correlated and local components roughly orthogonal on average and half the
samples anti-aligned (measured `negDot ~= 0.5`), the average shrinks to
`sqrt(wc^2 * avg|C|^2 + wl^2 * avg|L|^2)`, so `Organic` averaged ~0.076 —
*below* `Subtle` (0.100). Two tuning paths were considered. Option A
(parameters only: raise magnitude ranges or shift weights) was measured but
rejected: scaling magnitudes ~1.5x to reach the 0.115 floor pushes aligned
vertices into the `Organic` cap (0.220), re-clamping the distribution and
hovering near the hard cap; shifting weight toward the larger local component
degenerates into pure corner jitter (explicitly out of scope). Option B
(chosen, `blend.rs`): keep both components and their weights for the
*direction*, but take the *magnitude* from the strongest component —
`max(|C|, |L|)`. The result is a deterministic integer blend whose magnitude
never comes from a vector sum, so it cannot cancel. It preserves the local
high-frequency character (|L| dominates per-corner) while the moderate,
coherent large-scale flow comes from the weighted direction. The magnitude
distribution (and thus the profile averages) is bounded by the component
magnitude ranges, giving every seed healthy margins below the hard caps and
no measured reduction/fallback across the 256-seed stress.

## Последствия (Consequences)

* **Положительные**: Deterministic topology can be inspected in-place; shared
  borders and vertices are visually auditable; normal visuals remain unchanged
  while debug mode is off.
* **Отрицательные**: The overlay adds a small runtime resource/cache and
  requires native visual checks in addition to unit tests.
* **Нейтральные**: The diagnostic is intentionally hidden after Balance and
  does not project warped vertices onto Height3D.
