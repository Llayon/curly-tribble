# SOP: Zenith Navigator (Navigation Patterns)

## Core Principles
- **Async Execution**: Never block the main thread with pathfinding. Use `NavigationCommandsExt::move_to`.
- **Reactive Updates**: Use `NavObstacle` component to automatically update the `NavigationMap` via Observers.
- **Integer Scaling**: Costs are `u8`. `COST_BLOCKER` (0) means impassable. Lower non-zero values are preferred.
- **Path Invalidation**: Always handle `PathBlockEvent` to cancel paths that are no longer valid.

## Implementation Standard
- **Coordinate Conversion**: Use `world_to_grid` and `grid_to_world` helpers.
- **Component Lifecycle**: `NavObstacle` requires `Transform`.
- **State Management**: Characters waiting for a path will have the `ComputingPath` component. When the path is ready, it is replaced by `Path`.

## Performance
- **Query Filtering**: Systems iterating over paths MUST use `With<Path>` or `With<ComputingPath>` filters (Guard #16).
- **Task Polling**: Use `future::poll_once` in `poll_path_tasks` to check for completion without blocking.
