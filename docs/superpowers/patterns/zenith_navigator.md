# SOP: Zenith Navigator (Navigation Patterns)

## Core Principles
- **Async Execution**: Never block the main thread with pathfinding. Use `NavigationCommandsExt::move_to`.
- **Reactive Updates**: Use `NavObstacle` component to automatically update the `NavigationMap` via Observers.
- **Integer Scaling**: Costs are `u8`. `COST_BLOCKER` (0) means impassable. Lower non-zero values are preferred.
- **Path Invalidation**: Always handle `PathBlockEvent` to cancel paths that are no longer valid.

## Implementation Standard
- **Coordinate Conversion**: Use `world_to_grid` and `grid_to_world` helpers.
- **Pure Logic**: Core pathfinding MUST be implemented in pure functions (e.g., `compute_astar_path`) decoupled from Bevy's `World` for unit testing.
- **Interaction Radius**: Use `NavigationCommandsExt::interact_with` to stop at a distance from targets.
- **Finite Search**: Always implement a `search_limit` to prevent infinite loops in unreachable areas.

## Testing
- **ASCII Maps**: Use the `parse_ascii_map` helper for readable navigation tests.
- **Regression Suite**: Every new navigation feature must include a corresponding test in `navigation_tests.rs`.

## Performance
- **Query Filtering**: Systems iterating over paths MUST use `With<Path>` or `With<ComputingPath>` filters (Guard #16).
- **Task Polling**: Use `future::poll_once` in `poll_path_tasks` to check for completion without blocking.
