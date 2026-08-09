# SOP Pattern: Surface Height Solver Operations

## Overview
This pattern documents the operation and maintainer guidelines for Milestone M5 — `SurfaceHeightLayer` and `SurfaceHeightSolver`.

## Pipeline Structure

```
MapData + SurfaceTopology + HeightConstraintGraph
                     │
                     ▼
         derive_legacy_height_guide()
                     │
                     ▼
          compile_height_targets()
                     │
                     ▼
        compile_hard_constraints()
                     │
                     ▼
          solve_surface_heights()
                     │
                     ▼
      validate_surface_height_layer()
                     │
                     ▼
       Published SurfaceHeightLayer
```

## Key Components

- `SurfaceHeightLayer`: Stores `heights: Vec<f32>` (indexed by `HeightNodeId.index()`) and `stats: SurfaceHeightStats`.
- `HeightSolverConfig`: Resource holding weights (`guide_weight`, `region_weight`, `smoothness_weight`), feature biases, cliff drop threshold (`cliff_min_drop`), relaxation parameter, max iterations, and convergence epsilon.
- `LegacyHeightGuide`: Compatibility bridge converting legacy `TileData.elevation` into node-level soft target guides with ocean hard pins (`0.0`).
- `HeightTargetField`: Explicit struct containing `{ target, weight }` per node for Jacobi solver.
- `CompiledHeightHardConstraints`: DAG cliff constraint edges and interval bounds `[lower_bounds, upper_bounds]`.

## Architectural Constraints

1. **Hardcoded Domain**: All heights strictly in `[0.0, 1.0]`.
2. **Side-by-Side**: Production rendering is untouched until M5.1.
3. **No Retries on Failures**: Inputs recorded before solving; failure clears derived resources.
