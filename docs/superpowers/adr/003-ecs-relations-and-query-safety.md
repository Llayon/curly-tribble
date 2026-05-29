# ADR: ECS Entity Relations and Query Safety (Bevy 0.18.1)

## Context
During the implementation of Phase 7: Treasures, we needed a way to link a source treasure to a target treasure (a "Treasure Map" mechanic). Initially, we stored the target `Entity` ID directly inside the `TreasureItem` enum within the `TreasureDeposit` component.

Additionally, our UI and tool systems used broad, unfiltered queries, leading to runtime panics (e.g., Error B0001) and violation of internal architectural guards.

## Decisions

### 1. No Raw Entity References in Components
To comply with our "Semantic Graph" architecture (Guard #18) and ensure Bevy handles component lifecycles correctly, we strictly forbid storing `Entity` IDs inside structs/enums.
Instead, relations are modeled using Bevy's spatial hierarchy (ChildOf/Parent) and dedicated targeting components.
- **Implementation**: A treasure link is created by spawning a child entity with a `MapToTarget` marker and a `Targeting { target: Entity }` component. 
- **Benefit**: If the parent treasure is despawned, Bevy automatically cleans up the child relation entity.

### 2. Strict Query Isolation (Preventing B0001)
In Bevy, querying the same component mutably and immutably in the same system causes a runtime panic (Error B0001). 
- **Implementation**: When iterating over entities to find a position (e.g., raycasting a mouse click), we do NOT fetch the full component data if it's not needed. We use `Query<(Entity, &GlobalTransform), With<TreasureDeposit>>` instead of `Query<(Entity, &GlobalTransform, &TreasureDeposit)>`. 
- **Benefit**: This allows a separate mutable query `Query<&mut TreasureDeposit>` to exist in the same system without triggering the borrow checker panic.

### 3. Resource Initialization Safety
Bevy 0.18.1 is strict about resource existence when requested via `Res<T>` or `ResMut<T>`.
- **Implementation**: All global state structures (`MapData`, `WorldSeed`, `NavigationMap`, `LinkToolState`) MUST be explicitly initialized using `.init_resource::<T>()` or `.insert_resource(T)` inside their respective Plugin's `build` function. Failure to do so results in a startup panic in `system_param.rs`.

## Status
Accepted and implemented during Phase 7 Editor cleanup.