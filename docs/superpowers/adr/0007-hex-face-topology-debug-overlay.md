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
  field (~65/35 correlated/local weight, macro-cell span of 5 hexes) whose
  gradient arises from the fixed camera rotation applied to a stable, seeded
  node vector. Target kept small enough that nearly every face stays a valid
  convex polygon without backoff.
- `PagoniaLike`: a stronger correlated variant (~75/25 weight) meant to
  approximate a larger-cell, more continuous look reminiscent of that title's
  map aesthetics. This is a **visual experiment only**; it makes no claim to
  reproduce any actual Pagonia data structure, algorithm, or asset.

All three profiles share the same deterministic top-bottom seed, rely on
integer derived fields (no runtime `sin`/`cos`, no `StdRng`, no
`HashMap`-order dependence), and undergo the same simultaneous backoff loop to
guarantee valid convex geometry. An explicit per-vertex and combined set of
golden displacement vectors locks the correlated field and displacement to
exact `f32::to_bits()` references.

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
