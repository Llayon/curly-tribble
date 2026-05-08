# AGENTS.md — Savage Fantasy

## First steps
- Read `llms.txt` for project map; `GEMINI.md` for constitution.
- Key SOPs: `docs/superpowers/patterns/bevy_018_standards.md` (Bevy patterns), `docs/superpowers/patterns/zenith_navigator.md` (pathfinding).
- All architectural rules are enforced by 29 tests in `tests/architecture.rs`.
- Package name is `savage_fantasy` (from `Cargo.toml`), not the directory name.

## Essential commands
- `cargo fmt` — required before every commit
- `cargo clippy` — must pass clean (clippy is deny-all with specific Bevy exceptions)
- `cargo test` — runs both unit tests and all 29 architecture guard tests
- `cargo test -q` — quieter output for iterative dev
- There is no build/dev server command; it's a Bevy app launched via `cargo run`.

## Architecture (non-obvious)

### Bevy 0.18.1 specifics
- Events are now **Messages** (`#[derive(Message)]`, `add_message::<T>()`, `MessageReader`/`MessageWriter`).
- Entity relationships use the `#[relationship]` / `#[relationship_target]` attributes (see `src/pawn/relations.rs`). No raw `Entity` references in components.
- Component hooks: `app.world_mut().register_component_hooks::<T>().on_add(...)`.

### System ordering
- Three **GameSet** stages in both `Update` and `FixedUpdate`: `Input → Logic → Visuals` (chained).
- Two **StartupSet** stages: `LoadAssets → SpawnEntities` (chained).
- `GameSet::Logic` is gated on `GameState::Playing` (set centrally in `src/sets.rs`).
- **Simulation** (AI, Needs, Atmosphere) goes in `FixedUpdate` with `Time<Fixed>`. **Rendering/UI** goes in `Update`.

### Mandatory conventions
- **No `.unwrap()` or `.expect()`** in `src/` — use `if let`, `match`, or `.ok()`. Enforced by Guard #22.
- **Never use `bool` flags** for state/classification. Use ZST marker components or `State` enums. Guards #9, #11.
- **Always use named Bundles** for spawning complex entities, never anonymous component tuples. Guard #13.
- **Every `.rs` file in `src/`** (except `main.rs` and `sets.rs`) must contain `impl Plugin for`. Guard #6.
- **Every system** must be in a `GameSet` or `StartupSet` via `.in_set()`. Guard #14.
- **Every mutable `Query<&mut T>`** must have a filter (`With`, `Without`, `Changed`, etc.). Guard #16.
- **File size**: max 300 lines. Guard #21.
- **Custom Commands** (Plugins 2.0): use named `struct FooCommand` implementing `Command` + a fluent trait extension. Never anonymous closures in `.queue()`. Guard #27.

### Module boundaries
- `src/main.rs` — only `fn main()`, plugin registration, and `#[cfg(debug_assertions)]` block. Guard #1.
- `src/sets.rs` — all `SystemSet` definitions and scheduling configuration.
- `src/economy/` — assets, global resources, custom economy commands.
- `src/map/` — terrain, zoning, construction, navigation (A* with `NavObstacle`).
- `src/pawn/` — settler AI: needs, brain, behaviors (ZST markers), relations.
- `src/ui/` — HUD panels (resources, settler details, game log).
- `src/camera.rs` — orbit camera (WASD move, Q/E rotate, mouse wheel zoom).
- `src/events.rs` — game log messages with severity levels.
- `src/game_state.rs` — `States` enum: Loading → Playing.

### Commit style
- Follow Guard #26: messages must include `What:` and `Why:` blocks.
- See `commit_msg.txt` for template.

### Loop prevention (GEMINI.md)
- If any operation fails 3 times: read the target file, announce "LOOP DETECTED", propose alternative.
