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

/// Architecture Guard: Enforces that production terrain mesh rebuild in systems.rs routes ONLY through SurfaceTopology,
/// forbidding direct calls to derive_terrain_topology(&map_data).
#[test]
fn test_production_terrain_routes_through_surface_topology() {
    let sniffer = CodeSniffer::new("src/map/systems.rs");
    let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

    assert!(
        code_no_tests.contains("derive_terrain_topology_from_surface"),
        "Production Terrain Routing Violation in systems.rs: handle_rebuild_mesh must call derive_terrain_topology_from_surface."
    );

    assert!(
        !code_no_tests.contains("derive_terrain_topology(&map_data"),
        "Production Terrain Bypass Violation in systems.rs: handle_rebuild_mesh must not call legacy derive_terrain_topology directly."
    );
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
        "src/map/height_graph/validation.rs",
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
