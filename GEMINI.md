# Bevy 0.18.1 Development Rules

## Documentation & API Discovery
- **Primary Reference**: Always refer to the Bevy 0.18.1 documentation.
- **LLM Index**: Use `https://bevyengine.org/llms.txt` as a starting point to find relevant documentation modules.
- **Just-in-Time Research**: Before implementing new features or refactoring systems (especially Rendering, UI, or ECS), use `web_fetch` to retrieve the latest raw markdown guides from the Bevy website.

## Stability & Environment (Windows)
- **Graphics Backend**: Use `WGPU_BACKEND="dx12"` when running the application to avoid "swap chain" errors and CLI crashes on Windows.
- **Power Management**: Be aware that laptop GPUs (like the RTX 3050) may require explicit backend selection for stability.

## Architectural Patterns (0.18.1 specific)
- **Everything is a Plugin**: Every new functional block must be encapsulated in a `pub struct XPlugin; impl Plugin for XPlugin { ... }`.
- **Main.rs Restriction**: `main.rs` should only contain `App` initialization and `.add_plugins(...)` calls. No business logic or complex system definitions are allowed in `main.rs`.
- **UI & Text**: `LineHeight` is a separate component; do not look for it in `TextFont`.
- **Observers**: Prefer using `Observer` for UI interactions and text-picking where applicable.
- **Camera**: Check for built-in `FlyCamera` or `PanCamera` controllers before implementing custom movement boilerplate.
- **Safety**: Avoid `unwrap()` on queries; use `get_single_mut()` or `iter_mut().next()` to handle potential multi-camera scenarios gracefully.

## Context Efficiency
- Do not read the entire documentation. Use the index to target specific files (e.g., `migration-guides/0.17-to-0.18.md`) relevant to the current task.

## Development Standards (World-Class)
- **Atomic Commits**: Each commit should focus on a single logical change or feature.
- **AI-Friendly Commit Messages**: Use the Conventional Commits format with a structured body:
  - `What`: Technical changes summary.
  - `Why`: Rationale or architectural intent.
  - `Impact`: Observable effect on the system/user.
  - `Risk`: Potential regressions or edge cases.
- **Surgical Edits**: Prefer the `replace` tool over `write_file` for existing files to maintain context and logic integrity.
- **Guard-Driven Development**: Every architectural rule MUST have a corresponding test in `tests/architecture.rs`.
