# Editor Stabilization Plan

## Goal

Stabilize the nine-phase world editor at 120x120 tiles and prepare it for 100 active agents. The work prioritizes reliable editor workflows over new colony gameplay.

## Decisions

- Keep a single `Camera3d` as the default UI camera.
- Use a versioned JSON project format.
- Replace textual architectural checks with semantic AST-backed checks.
- Treat generated visuals separately from generated world content.

## Delivery Order

1. Restore a green baseline: clean formatting, Clippy, tests, and diagnose the WGPU panic with compatible and cinematic render profiles.
2. Repair ECS boundaries: typed generation modes, state enums instead of behavioural booleans, real Bevy relationships, and semantic architectural guards.
3. Scale editor rendering and navigation: coalesced dirty updates, chunked 16x16 meshes, immutable navigation snapshots, and stale-path cancellation.
4. Add `ProjectFileV1` JSON persistence with stable domain IDs, validation-before-apply, atomic writes, and editor Save/Load controls.
5. Restore ignored functional tests, add Windows CI, benchmark the 40x40 and 120x120 profiles, and remove duplicate source assets.

## Progress

- [x] Separate map visuals from map content with `MapVisualEntity`.
- [x] Make map generation intent explicit with `GenerationMode::{Reset, Preserve}`.
- [x] Coalesce rebuild messages and protect non-visual map entities with a regression test.
- [x] Mark the sole 3D camera as the default UI camera.
- [x] Omit empty water and roof meshes, preventing invalid GPU buffers for absent overlays.
- [x] Reduce the Clippy backlog from 62 to 21 diagnostics with behavior-preserving fixes.
- [ ] Complete the clean baseline: resolve the existing Clippy backlog and WGPU validation panic.
- [ ] Continue with semantic guard checks, chunked rendering, navigation snapshots, and persistence.

## Acceptance Criteria

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and the full test suite pass without ignored functional tests.
- Thirty regeneration cycles leak no mesh assets and produce no WGPU validation panic.
- A brush rebuild never despawns POIs, camps, deposits, treasures, or artifacts.
- Save/load produces an equivalent project and rejects malformed files without changing the active world.
- Continuous editing of a 120x120 map rebuilds only dirty visual regions; 100 agents never clone the full map per path request.

## Constraints

- Existing user changes to `panic_log.txt` and deleted office files remain outside implementation commits.
- No backwards project-file migration is required before `ProjectFileV1` exists.
- Each implementation stage receives its own `What:` / `Why:` commit after verification.
