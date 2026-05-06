# Standard Operating Procedures: Bevy 0.18.1 Patterns

## Reactive UI & Change Detection
- **Rule**: Never update UI frame-by-frame. Use `resource_changed::<R>` or `Changed<T>` filters.
- **Pattern**: For complex updates, use `AnyOf` or `Or` filters in queries.
- **Verification**: Architectural Guard #19 enforces this globally.

## Deterministic Simulation (FixedUpdate)
- **Rule**: All simulation logic (AI, Needs, Atmosphere) MUST live in `FixedUpdate`.
- **Timing**: Use `Time<Fixed>` instead of `Time` for consistent delta calculations.
- **Verification**: Architectural Guard #20 enforces Simulation/Presentation split.

## Automated Setup (Hooks)
- **Rule**: Automate complex entities using `app.world_mut().register_component_hooks::<T>()`.
- **Pattern**: Use `on_add` to attach lights, bundles, or child entities automatically.

## Entity Hierarchy
- **Rule**: Use `.with_children()` for tools and visual attachments.
- **Standard**: Always use named `Bundle` structs for child entities to satisfy Guard #13.

## Global Orchestration (System Sets)
- **Rule**: Every system must belong to a `GameSet` (Update/FixedUpdate) or `StartupSet`.
- **Shield**: Use `GameSet::Logic.run_if(in_state(GameState::Playing))` for centralized state control.
