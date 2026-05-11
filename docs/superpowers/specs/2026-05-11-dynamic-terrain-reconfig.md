# Spec: Dynamic Terrain Reconfiguration

- **Status**: Draft
- **Date**: 2026-05-11
- **Topic**: UI-Driven Map Regeneration
- **References**: bevy-inspector-egui, bevy_egui, Bevy 0.18

## 1. Vision & Goals
Provide a real-time tuning environment for terrain generation. Allow the user to adjust noise parameters, map size, and seed through a dynamic UI, and regenerate the entire world with a single click.

## 2. Technical Architecture

### TerrainConfig Resource
A centralized configuration for all terrain parameters.
- **Fields**: 
    - `map_width`, `map_height` (Default 120x120)
    - `seed` (u32)
    - `macro_freq`, `macro_height`, `macro_sharpness`
    - `plateau_freq`, `plateau_height`, `plateau_steps`
    - `warp_freq`, `warp_strength`
- **Features**: `Reflect`, `InspectorOptions` with slider limits.

### Event-Driven Regeneration
- **Event**: `GenerateMapEvent`.
- **Trigger**: "Regenerate World" button in a custom `bevy_egui` window.
- **Side Effect**: "Randomize Seed" button updates the `TerrainConfig.seed`.

### Lifecycle & Cleanup
- **Marker**: `MapEntity` component.
- **Cleanup**: Before spawning a new map, all entities with `MapEntity` are recursively despawned.
- **Initialization**: Fire `GenerateMapEvent` once on startup to spawn the initial world.

### UI Styling (Dark Tactical RPG)
- **Colors**: Dark stone background (#1A1A1A), bronze accents (#CD7F32).
- **Layout**: Floating window with a large, prominent "Regenerate World" button.

## 3. Implementation Plan
1. **Update `Cargo.toml`**: Add `bevy-inspector-egui` and `bevy_egui`.
2. **Define `TerrainConfig`**: In `src/map/terrain_gen.rs`.
3. **Marker & Event**: In `src/map/mod.rs`.
4. **Refactor `spawn_map`**: 
    - Add listener for `GenerateMapEvent`.
    - Implement `MapEntity` cleanup.
    - Attach `MapEntity` to all spawned objects.
5. **UI System**: Implement the custom regeneration window.

## 4. Verification
- **Inspector**: `TerrainConfig` window appears and values can be changed.
- **Regeneration**: Clicking the button removes the old map and spawns a new one with updated parameters.
- **Stability**: No entity leaks after multiple regenerations.
