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
