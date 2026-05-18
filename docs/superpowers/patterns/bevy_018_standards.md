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

## Command Application (Race Conditions)
- **Rule**: Bevy `Commands` are not applied immediately within chained systems. When a custom `Command`'s `apply` method runs, any further `world.commands()` queued inside it will suffer a 1-tick delay.
- **Pattern**: When implementing `Command::apply(self, world: &mut World)`, use `world.get_entity_mut()` or `world.get_resource_mut()` to insert components or mutate resources *immediately*. This prevents race conditions where subsequent chained systems fail their query filters (e.g. `Without<ComputingPath>`).

## Bevy Egui 0.39 Integration (Multi-Pass UI)
- **Rule**: All `egui::Window` systems MUST be added to the `EguiPrimaryContextPass` schedule instead of standard `Update`.
- **Why**: This ensures that font initialization is complete and input events are synchronized, preventing `No fonts available` panics and "stuck" non-interactive windows.
- **Pattern**: 
  ```rust
  app.add_systems(EguiPrimaryContextPass, my_ui_system);
  ```
- **Interactivity**: Always use a stable `egui::Id` for windows and ensure `title_bar(true)` is set for draggability. Use `horizontal_wrapped` inside `ScrollArea` to prevent "shrink-wrap" layout locks.

## Procedural Mesh Management (VRAM Leaks)
- **Rule**: All procedural meshes created at runtime MUST be tracked and manually removed from `Assets<Mesh>` when replaced.
- **Why**: Bevy's `Assets` collection does not automatically garbage collect GPU memory when handles are dropped if those handles are stored in `Mesh3d` components or other resources. Frequent world regeneration without manual `remove()` will lead to VRAM exhaustion and renderer panics.
- **Pattern**: Use a dedicated `Resource` (e.g., `GeneratedMapAssets`) to store `Option<Handle<Mesh>>` and call `meshes.remove(&h)` before spawning a new map.

## Phased Map Generation
- **Rule**: Separate 2D shape probability (Shape Phase) from 3D elevation and micro-details (Detail Phases).
- **Optimization**: Phase 1 MUST use a lightweight 2D pass (e.g., `get_shape_value`) to determine land/ocean boundaries. Full 3D calculations (terracing, plateaus, warping) should only trigger once the user leaves the shaping phase.
- **Persistence**: User-painted data (like `is_ocean`) MUST be preserved across phase transitions and only reset upon explicit world regeneration (new seed).
