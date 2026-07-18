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
