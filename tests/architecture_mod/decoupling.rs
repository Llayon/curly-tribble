use crate::utils::CodeSniffer;
use std::fs;
use std::path::Path;

/// 11. Архитектурная развязка: Логика не должна знать о выводе (UI/Logs).
#[test]
fn test_architectural_decoupling() {
    check_decoupling_recursive(Path::new("src"));
}

fn check_decoupling_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_decoupling_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap().replace("\\", "/");
            let allowed_to_log = [
                "src/main.rs",
                "src/ui",
                "src/events.rs",
                "src/game_state.rs",
                "src/sets.rs",
            ];
            if allowed_to_log.iter().any(|&p| path_str.contains(p)) {
                continue;
            }

            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            let forbidden_macros = ["info!", "warn!", "error!", "println!"];
            for macro_name in forbidden_macros {
                if code_no_tests.contains(macro_name) {
                    panic!(
                        "Decoupling Violation: Core logic file {:?} uses direct logging '{}'.",
                        path, macro_name
                    );
                }
            }
            let uses_ui_dependency =
                code_no_tests.contains("bevy::ui") && !path_str.ends_with("src/camera.rs");
            if uses_ui_dependency || code_no_tests.contains("Interaction") {
                panic!(
                    "Dependency Violation: Core logic file {:?} depends on UI types.",
                    path
                );
            }
        }
    }
}

/// Architecture Guard: Enforces that cliff gizmo visualization relies ONLY on authoritative HexFaceTopology,
/// forbidding HEX_SIZE, EdgeDirection, to_world, .sin(), and .cos() in src/economy/mesh_gen/cliff_gizmos.rs.
#[test]
fn test_cliff_gizmos_authoritative_topology_decoupling() {
    let sniffer = CodeSniffer::new("src/economy/mesh_gen/cliff_gizmos.rs");
    let code = sniffer.clean;

    let forbidden = ["HEX_SIZE", "EdgeDirection", "to_world", ".sin()", ".cos()"];

    for item in forbidden {
        assert!(
            !code.contains(item),
            "Cliff Gizmos Architecture Violation: cliff_gizmos.rs must not contain '{item}'. Geometry must come purely from HexFaceTopology."
        );
    }
}

/// Architecture Guard: Enforces that warped cliff edge picking and editing rely ONLY on authoritative HexFaceTopology,
/// forbidding HEX_SIZE, HexCoord::from_world, .to_world(, angle_deg, angle_rad, .sin(), .cos(), EdgeDirection, and RebuildMeshEvent
/// in src/map/tools/cliff_edit.rs and src/map/tools/landscape_edge_picker.rs.
#[test]
fn test_cliff_edit_authoritative_topology_decoupling() {
    let files = [
        "src/map/tools/cliff_edit.rs",
        "src/map/tools/cliff_picking.rs",
        "src/map/tools/landscape_edge_picker.rs",
    ];

    let forbidden = [
        "HEX_SIZE",
        "from_world",
        ".to_world(",
        "angle_deg",
        "angle_rad",
        ".sin()",
        ".cos()",
        "EdgeDirection",
        "RebuildMeshEvent",
    ];

    for file in files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

        for item in forbidden {
            assert!(
                !code_no_tests.contains(item),
                "Cliff Editing Architecture Violation in {file}: code must not contain '{item}'. Selection and editing must rely on HexFaceTopology edge index without regular hex conversion or mesh rebuild events."
            );
        }
    }
}

/// Architecture Guard: Enforces that SurfaceTopology production modules rely ONLY on authoritative HexFaceTopology XZ/connectivity,
/// forbidding HEX_SIZE, to_world, canonical_vertex_key, angle_deg, angle_rad, .sin(), .cos(), MAX_HEIGHT, compute_vertex_heights, TerrainHeightMode, and .elevation.
#[test]
fn test_surface_topology_authoritative_decoupling() {
    let files = [
        "src/map/surface_topology/types.rs",
        "src/map/surface_topology/generator.rs",
        "src/map/surface_topology/twins.rs",
        "src/map/surface_topology/validation.rs",
        "src/map/surface_topology/provenance_validation.rs",
        "src/map/surface_topology/runtime.rs",
    ];

    let forbidden = [
        "HEX_SIZE",
        "to_world(",
        "canonical_vertex_key",
        "angle_deg",
        "angle_rad",
        ".sin()",
        ".cos()",
        "MAX_HEIGHT",
        "compute_vertex_heights",
        "TerrainHeightMode",
        ".elevation",
    ];

    for file in files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

        for item in forbidden {
            assert!(
                !code_no_tests.contains(item),
                "SurfaceTopology Architecture Violation in {file}: production code must not contain '{item}'."
            );
        }
    }
}

/// Architecture Guard: Enforces that SurfaceTerrainAdapter relies ONLY on SurfaceTopology,
/// forbidding face_topology, MapData, HEX_SIZE, to_world, canonical_vertex_key, SurfaceVertexSource, MAX_HEIGHT, .elevation, compute_vertex_heights, TerrainHeightMode, .sin(), .cos().
#[test]
fn test_surface_terrain_adapter_decoupling() {
    let sniffer = CodeSniffer::new("src/map/surface_topology/terrain_adapter.rs");
    let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

    let forbidden = [
        "HexFaceTopology",
        "HexFace",
        "map::face_topology",
        "MapData",
        "HEX_SIZE",
        "to_world(",
        "canonical_vertex_key",
        "SurfaceVertexSource",
        "MAX_HEIGHT",
        ".elevation",
        "compute_vertex_heights",
        "TerrainHeightMode",
        ".sin()",
        ".cos()",
    ];

    for item in forbidden {
        assert!(
            !code_no_tests.contains(item),
            "SurfaceTerrainAdapter Architecture Violation in terrain_adapter.rs: code must not contain '{item}'."
        );
    }
}

/// Architecture Guard: Enforces that production terrain mesh rebuild in systems.rs routes
/// ONLY through the authoritative M5.1 `SurfaceTerrainBake` (via
/// `derive_terrain_topology_from_bake`), gates on `TerrainBakeGenerationOutcome::Success`,
/// and never falls back to the legacy SurfaceTopology adapter or legacy height computation.
#[test]
fn test_production_terrain_routes_through_height_bake() {
    let sniffer = CodeSniffer::new("src/map/systems.rs");
    let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

    assert!(
        code_no_tests.contains("derive_terrain_topology_from_bake"),
        "Production Terrain Routing Violation in systems.rs: handle_rebuild_mesh must call derive_terrain_topology_from_bake."
    );
    assert!(
        code_no_tests.contains("TerrainBakeGenerationOutcome::Success"),
        "Production Terrain Routing Violation in systems.rs: handle_rebuild_mesh must gate on TerrainBakeGenerationOutcome::Success."
    );
    assert!(
        !code_no_tests.contains("derive_terrain_topology_from_surface"),
        "Production Terrain Bypass Violation in systems.rs: handle_rebuild_mesh must not call derive_terrain_topology_from_surface."
    );
    assert!(
        !code_no_tests.contains("compute_vertex_heights"),
        "Production Terrain Bypass Violation in systems.rs: handle_rebuild_mesh must not call compute_vertex_heights."
    );
    assert!(
        !code_no_tests.contains("create_global_map_meshes("),
        "Production Terrain Bypass Violation in systems.rs: handle_rebuild_mesh must not call the legacy mesh generator."
    );
}

/// Architecture Guard: Enforces that the production `SpawnGlobalTerrainCommand` is bake-only —
/// the bake field is mandatory and the legacy `create_global_map_meshes` fallback is removed.
#[test]
fn test_production_terrain_command_is_bake_only() {
    let raw =
        std::fs::read_to_string("src/economy/mesh_gen/mod.rs").expect("mesh_gen/mod.rs must exist");
    let code_no_tests = raw
        .lines()
        .filter(|line| !line.contains("#[cfg(test)]"))
        .map(|line| line.split("//").next().unwrap_or("").trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        code_no_tests.contains("create_global_map_meshes_from_bake("),
        "Production Command Violation in mesh_gen/mod.rs: SpawnGlobalTerrainCommand must call create_global_map_meshes_from_bake."
    );
    assert!(
        !code_no_tests.contains("create_global_map_meshes("),
        "Production Command Violation in mesh_gen/mod.rs: SpawnGlobalTerrainCommand must not call the legacy create_global_map_meshes fallback."
    );
    assert!(
        !code_no_tests.contains("bake: Option<"),
        "Production Command Violation in mesh_gen/mod.rs: SpawnGlobalTerrainCommand.bake must be mandatory, not Option."
    );
}

/// Architecture Guard: Enforces that terrain_bake core modules rely ONLY on height-domain
/// data (`SurfaceTopology`, `HeightConstraintGraph`, `SurfaceHeightLayer`), forbidding
/// MapData, legacy topology, and render-side geometry (MAX_HEIGHT, heights, meshes, phases).
#[test]
fn test_terrain_bake_core_decoupling() {
    let files = [
        "src/map/terrain_bake/types.rs",
        "src/map/terrain_bake/builder.rs",
        "src/map/terrain_bake/walls.rs",
        "src/map/terrain_bake/runtime.rs",
        "src/map/terrain_bake/validation.rs",
    ];

    let forbidden = [
        "MapData",
        "TileData",
        "TerrainConfig",
        "TerrainTopology",
        "HexFaceTopology",
        "MAX_HEIGHT",
        "compute_vertex_heights",
        "TerrainHeightMode",
        ".elevation",
        "create_global_map_meshes",
        "SpawnGlobalTerrainCommand",
        "EditorPhase",
    ];

    for file in files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

        for item in forbidden {
            assert!(
                !code_no_tests.contains(item),
                "TerrainBake Core Architecture Violation in {file}: production code must not contain '{item}'."
            );
        }
    }
}

/// Architecture Guard: Enforces that HeightConstraint compilation modules rely ONLY on MapData intent and SurfaceTopology identity/connectivity,
/// forbidding TerrainTopology, derive_terrain_topology, terrain_adapter, topology_adapter, compute_vertex_heights, MAX_HEIGHT, .elevation, HEX_SIZE, from_world, to_world, Vec2, .position, .sin(), .cos(), face_topology, HexFaceTopology, MapVertex, BoundCliffEdges, SurfaceVertexSource, RebuildMeshEvent.
#[test]
fn test_height_constraints_decoupling() {
    let files = [
        "src/map/height_constraints/types.rs",
        "src/map/height_constraints/compiler.rs",
        "src/map/height_constraints/validation.rs",
        "src/map/height_constraints/runtime.rs",
    ];

    let forbidden = [
        "TerrainTopology",
        "derive_terrain_topology",
        "terrain_adapter",
        "topology_adapter",
        "compute_vertex_heights",
        "MAX_HEIGHT",
        ".elevation",
        "HEX_SIZE",
        "from_world",
        "to_world(",
        "Vec2",
        ".position",
        ".sin()",
        ".cos()",
        "HexFace",
        "HexFaceTopology",
        "map::face_topology",
        "MapVertex",
        "BoundCliffEdges",
        "SurfaceVertexSource",
        "RebuildMeshEvent",
    ];

    for file in files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

        for item in forbidden {
            assert!(
                !code_no_tests.contains(item),
                "HeightConstraint Architecture Violation in {file}: code must not contain '{item}'."
            );
        }
    }
}

/// Architecture Guard: Enforces that MapPlugin registers HeightConstraintsPlugin in production.
#[test]
fn test_production_map_plugin_registers_height_constraints() {
    let sniffer = CodeSniffer::new("src/map/mod.rs");
    let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

    assert!(
        code_no_tests.contains("height_constraints::HeightConstraintsPlugin"),
        "Production Plugin Registration Violation in src/map/mod.rs: MapPlugin must register HeightConstraintsPlugin."
    );
}

/// Architecture Guard: Enforces that MapPlugin registers HeightGraphPlugin in production.
#[test]
fn test_production_map_plugin_registers_height_graph() {
    let sniffer = CodeSniffer::new("src/map/mod.rs");
    let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

    assert!(
        code_no_tests.contains("height_graph::HeightGraphPlugin"),
        "Production Plugin Registration Violation in src/map/mod.rs: MapPlugin must register HeightGraphPlugin."
    );
}

/// Architecture Guard: Enforces strict decoupling for src/map/height_graph/ modules.
#[test]
fn test_height_graph_decoupling() {
    let files = [
        "src/map/height_graph/types.rs",
        "src/map/height_graph/builder.rs",
        "src/map/height_graph/builder_dsu.rs",
        "src/map/height_graph/builder_diagnostics.rs",
        "src/map/height_graph/validation.rs",
        "src/map/height_graph/validation_completeness.rs",
        "src/map/height_graph/diagnostics.rs",
        "src/map/height_graph/runtime.rs",
        "src/map/height_graph/mod.rs",
    ];

    let forbidden = [
        "MapData",
        "TileData",
        "LandscapeFeature",
        "EdgeType",
        "HexFaceTopology",
        "BoundCliffEdges",
        "TerrainTopology",
        "terrain_adapter",
        "topology_adapter",
        "WorldSeed",
        "TerrainConfig",
        "f32",
        "f64",
    ];

    for file in files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

        for item in forbidden {
            assert!(
                !code_no_tests.contains(item),
                "HeightGraph Architecture Violation in {file}: code must not contain '{item}'."
            );
        }
    }
}

/// Architecture Guard: Enforces strict 3-tier decoupling for src/map/surface_height/ modules.
#[test]
fn test_surface_height_decoupling() {
    let core_files = [
        "src/map/surface_height/types.rs",
        "src/map/surface_height/targets.rs",
        "src/map/surface_height/hard_constraints.rs",
        "src/map/surface_height/solver.rs",
        "src/map/surface_height/validation.rs",
    ];

    let core_forbidden = [
        "MapData",
        "TileData",
        "SurfaceTopology",
        "SurfaceFace",
        "HexFaceTopology",
        "TerrainTopology",
        "MAX_HEIGHT",
        "HEX_SIZE",
        "Vec2",
        "Vec3",
        "RebuildMeshEvent",
    ];

    for file in core_files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
        for item in core_forbidden {
            assert!(
                !code_no_tests.contains(item),
                "SurfaceHeight Core Architecture Violation in {file}: code must not contain '{item}'."
            );
        }
    }

    let guide_forbidden = [
        "TerrainTopology",
        "compute_vertex_heights",
        "Vec2",
        "Vec3",
        "RebuildMeshEvent",
    ];
    let guide_sniffer = CodeSniffer::new("src/map/surface_height/guide.rs");
    let guide_code = guide_sniffer
        .clean
        .split("#[cfg(test)]")
        .next()
        .unwrap_or("");
    for item in guide_forbidden {
        assert!(
            !guide_code.contains(item),
            "SurfaceHeight Guide Architecture Violation in guide.rs: code must not contain '{item}'."
        );
    }

    let runtime_forbidden = [
        "TerrainTopology",
        "compute_vertex_heights",
        "SpawnGlobalTerrainCommand",
        "RebuildMeshEvent",
    ];
    let runtime_sniffer = CodeSniffer::new("src/map/surface_height/runtime.rs");
    let runtime_code = runtime_sniffer
        .clean
        .split("#[cfg(test)]")
        .next()
        .unwrap_or("");
    for item in runtime_forbidden {
        assert!(
            !runtime_code.contains(item),
            "SurfaceHeight Runtime Architecture Violation in runtime.rs: code must not contain '{item}'."
        );
    }
}

/// Architecture Guard (M6): navigation production modules must route through
/// the authoritative `SurfaceGameplayMap` and the dynamic-only overlay grid,
/// never legacy `MapData` elevation or legacy topology.
#[test]
fn test_navigation_decoupling() {
    let files = [
        "src/map/navigation/types.rs",
        "src/map/navigation/algo.rs",
        "src/map/navigation/commands.rs",
        "src/map/navigation/systems.rs",
        "src/map/navigation/mod.rs",
    ];

    let forbidden = [
        "MapData",
        "TileData",
        "TerrainType",
        "OceanState",
        "MAX_HEIGHT",
        "TerrainTopology",
        "compute_vertex_heights",
        ".elevation",
        "create_global_map_meshes",
        "SpawnGlobalTerrainCommand",
        "RebuildMeshEvent",
    ];

    for file in files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

        for item in forbidden {
            assert!(
                !code_no_tests.contains(item),
                "Navigation Architecture Violation in {file}: production code must not contain '{item}'. Pathfinding must route through SurfaceGameplayMap."
            );
        }
    }
}

/// Architecture Guard (M6): surface_gameplay core (types/metrics/edges) must
/// stay pure-geometry; only compiler/runtime may read MapData classification
/// fields, and only world.rs may convert to world space (MAX_HEIGHT).
#[test]
fn test_surface_gameplay_decoupling() {
    let core_files = [
        "src/map/surface_gameplay/types.rs",
        "src/map/surface_gameplay/metrics.rs",
        "src/map/surface_gameplay/edges.rs",
    ];
    let core_forbidden = [
        "MapData",
        "TileData",
        "TerrainType",
        "OceanState",
        "MAX_HEIGHT",
        "HEX_SIZE",
        "Mesh",
        "TerrainTopology",
        "compute_vertex_heights",
        "TerrainHeightMode",
        ".elevation",
        "NavigationMap",
        "compute_astar_path",
    ];
    for file in core_files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
        for item in core_forbidden {
            assert!(
                !code_no_tests.contains(item),
                "SurfaceGameplay Core Architecture Violation in {file}: production code must not contain '{item}'."
            );
        }
    }

    let classification_files = [
        "src/map/surface_gameplay/compiler.rs",
        "src/map/surface_gameplay/runtime.rs",
        "src/map/surface_gameplay/validation.rs",
        "src/map/surface_gameplay/mod.rs",
        "src/map/surface_gameplay/config.rs",
    ];
    let classification_forbidden = [
        "MAX_HEIGHT",
        "HEX_SIZE",
        "bevy::mesh",
        "Mesh::new",
        "TerrainTopology",
        "compute_vertex_heights",
        "TerrainHeightMode",
        ".elevation",
        "NavigationMap",
        "compute_astar_path",
    ];
    for file in classification_files {
        let sniffer = CodeSniffer::new(file);
        let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
        for item in classification_forbidden {
            assert!(
                !code_no_tests.contains(item),
                "SurfaceGameplay Architecture Violation in {file}: production code must not contain '{item}'."
            );
        }
    }

    let world_sniffer = CodeSniffer::new("src/map/surface_gameplay/world.rs");
    let world_code = world_sniffer
        .clean
        .split("#[cfg(test)]")
        .next()
        .unwrap_or("");
    for item in [
        "MapData",
        "TileData",
        "TerrainType",
        "OceanState",
        "Mesh",
        "compute_vertex_heights",
        ".elevation",
        "NavigationMap",
    ] {
        assert!(
            !world_code.contains(item),
            "SurfaceGameplay World Architecture Violation in world.rs: code must not contain '{item}'."
        );
    }
}

/// Architecture Guard (M6): map generation must not build or touch the
/// navigation layer — static NavigationMap is retired; pathfinding is
/// gameplay-driven.
#[test]
fn test_generation_terrain_decoupling() {
    let sniffer = CodeSniffer::new("src/map/generation/terrain.rs");
    let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

    let forbidden = [
        "NavigationMap",
        "NavObstacle",
        "compute_astar_path",
        "world_to_grid",
        "AGENT_HEIGHT",
        "COST_BASE",
        "COST_BLOCKER",
    ];
    for item in forbidden {
        assert!(
            !code_no_tests.contains(item),
            "Generation Architecture Violation in terrain.rs: code must not contain '{item}'."
        );
    }
}

/// Architecture Guard (M6): production mesh rebuild routes through the solved
/// gameplay layer — gated on gameplay Success and feeding it to the command;
/// buildability colors come from SurfaceGameplayMap cells.
#[test]
fn test_production_terrain_routes_through_gameplay() {
    let sniffer = CodeSniffer::new("src/map/systems.rs");
    let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

    assert!(
        code_no_tests.contains("SurfaceGameplayGenerationOutcome::Success"),
        "Production Terrain Routing Violation in systems.rs: handle_rebuild_mesh must gate on SurfaceGameplayGenerationOutcome::Success."
    );
    assert!(
        code_no_tests.contains("gameplay: (*gameplay).clone()"),
        "Production Terrain Routing Violation in systems.rs: SpawnGlobalTerrainCommand must receive the SurfaceGameplayMap."
    );
    assert!(
        !code_no_tests.contains("compute_astar_path"),
        "Production Terrain Routing Violation in systems.rs: systems.rs must not run pathfinding directly."
    );

    let bake_sniffer = CodeSniffer::new("src/economy/mesh_gen/bake.rs");
    let bake_code = bake_sniffer
        .clean
        .split("#[cfg(test)]")
        .next()
        .unwrap_or("");
    assert!(
        bake_code.contains("MissingGameplayCell"),
        "Production Mesh Violation in bake.rs: buildability lookup must fail closed with MissingGameplayCell."
    );
    assert!(
        bake_code.contains("gameplay"),
        "Production Mesh Violation in bake.rs: bake coloring must receive the SurfaceGameplayMap."
    );
}
