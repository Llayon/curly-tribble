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
changing profile weights. The fix (`blend_policy.rs` + `blend_diagnostics.rs` +
`blend.rs`):

- The **law** lives in `src/map/face_topology/blend_policy.rs`, a dependency
  leaf that imports nothing from the blend implementation: the activation mode
  (`WeightedLength` or `ReferenceProjection`), the threshold ratio, and the
  correlated-preference margin. The boundary comparison is cross-multiplied in
  `i128`, so no intermediate division can round it: exactly `floor`,
  `floor+1`, and `floor+2` keep the raw law, and only `floor-1`/`floor-2`
  correct (permanent assertions in `tests_blend_boundary.rs`).
- A corner is *unreliable* when its measured quantity (the raw weighted length,
  or for the projection mode its projection onto the reference) is below
  `MIN_RELIABLE_DIRECTION_RATIO_Q16` (1/64) of the target magnitude. Only
  those corners change direction; all reliable corners keep the exact previous
  integer arithmetic, bit for bit (so `Subtle`, whose geometry never
  near-cancels, is byte-identical).
- Unreliable corners (raw weighted length below `minimum_reliable_length_q16`)
  are stabilized via sign-aware ceiling radial length scaling (`div_away_from_zero`),
  scaling the raw weighted vector to reach or exceed the reliability floor while
  strictly preserving the raw vector's hemisphere orientation. Reference selection
  (`BlendReference`) determines the activation condition (`is_below_floor`) and
  exact-zero fallback vector.
- The production baseline matches the hardened radial stabilization contract (commit
  `6454046`, two-level baseline contract in `tests_blend_candidate_geometry.rs`):
  the public entry points equal the explicit production-policy pipeline, and the
  literal geometry fingerprints for `Organic`/`PagoniaLike` at seeds 42 and 194 match
  `6454046` (historical pre-fix baseline `9ad12ae` migrated to eliminate reference-projection
  vector addition direction flips).

**Candidate matrix** — measured honestly: every candidate is generated through
its own complete pipeline, so no row is a re-classification of another's
topology (all rows valid and backoff-free; non-raw candidates change real
geometry, verified by fingerprints). Authoritative scan:
`full_blend_reliability_candidate_geometry_256_seeds` (256 canonical seeds ×
3,360 corners = 860,160 samples per row; adjacency extremes from
`full_candidate_adjacency_256_seeds`):

| policy | stab. Organic | stab. Pago | worst both-stab. dot (Org/Pago) | min stabilized ratio (Org/Pago) |
|---|---|---|---|---|
| raw (no floor) | 0 | 0 | — | — |
| **1/64 length (production)** | **618 (0.07%)** | **500 (0.06%)** | **−0.041 / +0.738** | **995 / 1004** |
| 1/32 length | 2,338 | 1,993 | +0.447 / +0.456 | 2024 / 2032 |
| 1/16 length | 9,224 | 7,702 | −1.0 / −1.0 | 4073 / 4076 |
| 1/64 projection | 11,588 | 5,804 | −1.0 / −0.99980 | 995 / 1004 |
| 1/32 projection | 16,710 | 9,179 | −1.0 / −0.99980 | 2024 / 2032 |
| 1/16 projection | 31,095 | 20,744 | −1.0 / −1.0 | 4073 / 4076 |

The earlier documented 1/32 dot of `−0.986` was an artifact of re-classifying
corners on the production topology; honest per-candidate geometry at 1/32
length has **no** both-stabilized anti-parallel pair (`+0.45`). The `−1.0`
values at 1/16 length and across the projection mode are real: when both
endpoints of an edge stabilize, they project onto the same reference, and at
those floors the projected directions become anti-parallel. The tolerance band
for this is documented as `NEAR_ANTIPARALLEL_DOT_THRESHOLD = −0.9995`, with an
exact `-1.0` separately detectable via `to_bits()`.

**Decision (report-only, no production change)**: production keeps the 1/64
`WeightedLength` law. It is the smallest floor whose worst both-stabilized
adjacent dot stays far above the `−0.1` anti-parallel band (`−0.041`), corrects
only ~0.07% of corners, and changes nothing measurable outside those corners
(acceptance metrics identical to raw). 1/32 length also avoids anti-parallel
pairs but quadruples the stabilized population with no measured benefit; 1/16
length and every projection-mode floor create both-stabilized anti-parallel
pairs and are rejected. The matrix is retained as an ignored scan for future
tuning decisions.

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

## Radial Blend Acceptance Contract Freeze (Commit 3)

The production blend stabilization law `SIGN_AWARE_CEIL_RADIAL` is formally **CLOSED and FROZEN**.
All verification obligations, arithmetic totality proofs, matrix audits, and Stage 2 shape contracts are closed and hardened.

### Evidence Hierarchy

1. **MATHEMATICAL GUARANTEES**:
   - **Sign Preservation**: For every non-zero component $wx$, $scale\_radial\_component\_q16(wx, L, W)$ preserves $sign(wx)$, keeping the stabilized vector strictly in the raw hemisphere quadrant.
   - **Total Bounded Arithmetic**: `scale_radial_component_q16` operates on `u128` intermediates ($abs\_comp \times target\_floor$), mathematically eliminating overflow panics or truncation for all production bounds ($|wx| \le 15_729$, $L \le 245$, $W \ge 1$). No `.unwrap()`, `.expect()`, or silent fallbacks exist in production code.
   - **Weight-Sum Invariant**: $correlated\_weight\_q16 + local\_weight\_q16 == Q16 == 65\_536$ is verified for all production profiles (`Subtle`, `Organic`, `PagoniaLike`).
   - **Conservative Radial Floor Proof**: Because $S = wx^2 + wy^2 \ge W^2$ under floor integer square root $W = \lfloor \sqrt{S} \rfloor$, sign-away-from-zero scaling yields $sx^2 + sy^2 \ge L^2 \frac{S}{W^2} \ge L^2 \implies \lfloor \sqrt{sx^2 + sy^2} \rfloor \ge L$. Therefore, signed floor metrics satisfy:
     $$floor\_deficit\_q16 = requested - stabilized \le 0$$
     $$floor\_excess\_q16 = stabilized - requested \ge 0$$
     with 0 positive floor deficits.

2. **EXHAUSTIVE TESTED CONTRACTS**:
   - **Canonical 256-Seed Matrix**: Verified across $1,024$ topology generation runs ($512$ raw, $512$ production). 256-seed total corrected corners equals historical expected count ($1,118$).
   - **12-Way Perturbation Matrix**: Verified across all corrected corners ($13,416$ theoretical max cases) with 100% perturbation reconciliation equality. All incident-edge dots and near-antiparallel transitions are tracked and verified safe.
   - **Full Stage 2 6-Shape Matrix**: Verified $3,072$ topologies ($2 \text{ profiles} \times 6 \text{ grid shapes} \times 256 \text{ seeds}$) with 12 distinct per-profile/per-shape report rows. All interior angle and aspect quality criteria satisfied (`Organic`: angle $\le 162^\circ$, aspect $\ge 0.490$; `PagoniaLike`: angle $\le 176^\circ$, aspect $\ge 0.370$).
   - **Determinism**: 100% bit-identical geometry and connectivity fingerprints under repeated generation and insertion-order variations.

3. **OBSERVED EXTREMA**:
   - Worst corrected-endpoint angular rotation: $< 0.4^\circ$.
   - Maximum floor excess: $346 - 245 = 101$ Q16 units.
   - Weakest corner quantization fixtures: locked for `Organic seed 64` (`[(14,7),(15,6),(15,7)]`) and `PagoniaLike seed 194` (`[(6,7),(6,8),(7,7)]`).

### Next Phase Transition
Further work on radial blend algorithm design is frozen. Development moves directly to `HexFaceTopology -> TerrainTopology` integration.
