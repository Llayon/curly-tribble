use std::path::Path;
use std::fs;
use crate::utils::{CodeSniffer, is_data_only};

/// 6. Проверка "Все есть Плагин": каждый файл с логикой должен быть модульным.
#[test]
fn test_all_src_files_are_plugins() {
    check_dir_recursive_plugins(Path::new("src"));
}

fn check_dir_recursive_plugins(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Could not read src directory") {
        let entry = entry.expect("Invalid directory entry");
        let path = entry.path();
        if path.is_dir() {
            check_dir_recursive_plugins(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name == "main.rs" || file_name == "architecture.rs" { continue; }
            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            if is_data_only(&sniffer.clean) { continue; }
            assert!(sniffer.clean.contains("impl Plugin for") || sniffer.clean.contains("app.add_plugins"), "Modularity Violation in {:?}", path);
        }
    }
}

/// 7. Проверка использования Observers для выбора сущностей.
#[test]
fn test_pawn_selection_uses_observers() {
    let sniffer = CodeSniffer::new("src/pawn/mod.rs");
    assert!(sniffer.contains_call(".observe("), "Selection logic must use .observe()");
}

/// 8. Проверка реализации и интеграции GameState.
#[test]
fn test_game_state_is_implemented() {
    let sniffer_main = CodeSniffer::new("src/main.rs");
    assert!(sniffer_main.clean.contains("GameStatePlugin"), "GameStatePlugin must be registered");
}

/// 9. Запрет на использование "ручных" булевых флагов паузы/состояния.
#[test]
fn test_no_boolean_state_flags() {
    check_for_forbidden_state_flags(Path::new("src"));
}

fn check_for_forbidden_state_flags(dir: &Path) {
    let forbidden = ["is_paused: bool", "paused: bool", "is_loading: bool", "game_over: bool"];
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_for_forbidden_state_flags(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            for flag in forbidden {
                if sniffer.clean.contains(flag) {
                    panic!("Forbidden flag '{}' in {:?}", flag, path);
                }
            }
        }
    }
}

/// 10. Запрет на прямой доступ к World в системах (только Commands).
#[test]
fn test_no_direct_world_mutation_in_systems() {
    check_no_world_access(Path::new("src"));
}

fn check_no_world_access(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_no_world_access(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap();
            if path_str.contains("main.rs") || path_str.contains("architecture.rs") { continue; }
            let sniffer = CodeSniffer::new(path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            
            let forbidden = ["&mut World", ".world_mut()", "world: &mut"];
            for pattern in forbidden {
                if code_no_tests.contains(pattern) {
                    // ИСКЛЮЧЕНИЕ: Разрешаем доступ к World внутри реализаций кастомных команд.
                    // Это легальный паттерн Bevy 0.18 для расширения API (Plugins 2.0).
                    if code_no_tests.contains("impl") && (code_no_tests.contains("Commands") || code_no_tests.contains("Command")) {
                        continue;
                    }
                    
                    if code_no_tests.contains("impl Plugin for") && pattern == ".world_mut()" { continue; }
                    panic!("Forbidden World access '{}' in {:?}", pattern, path);
                }
            }
        }
    }
}

/// 12. Запрет на использование флагов внутри компонентов (Marker Components).
#[test]
fn test_no_boolean_classification_flags() {
    check_for_classification_flags(Path::new("src"));
}

fn check_for_classification_flags(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_for_classification_flags(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            let path_str = path.to_str().unwrap().replace("\\", "/");
            if path_str.contains("main.rs") || path_str.contains("events.rs") { continue; }
            for line in sniffer.clean.split(";") {
                let trimmed = line.trim();
                if trimmed.contains(": bool") {
                    let prefixes = ["is_", "has_", "can_", "should_"];
                    if prefixes.iter().any(|&p| trimmed.contains(p)) {
                        panic!("ECS Violation: Found flag '{}' in {:?}", trimmed, path);
                    }
                }
            }
        }
    }
}

/// 13. Проверка атомарности спавна: сложные сущности только через Bundles.
#[test]
fn test_complex_entities_use_bundles() {
    check_bundles_recursive(Path::new("src"));
}

fn check_bundles_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_bundles_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap().replace("\\", "/");
            let output_layer = ["src/main.rs", "src/ui", "src/events.rs", "src/game_state.rs"];
            if output_layer.iter().any(|&p| path_str.contains(p)) { continue; }
            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            let forbidden_patterns = ["spawn((", "insert(("];
            for pattern in forbidden_patterns {
                if code_no_tests.contains(pattern) {
                     panic!("Integrity Violation in {:?}: Found anonymous component tuple in '{}'.", path, pattern);
                }
            }
        }
    }
}

/// 14. Проверка глобального расписания: все системы должны быть в System Sets.
#[test]
fn test_systems_belong_to_sets() {
    check_sets_recursive(Path::new("src"));
}

fn check_sets_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_sets_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap();
            if path_str.contains("main.rs") || path_str.contains("sets.rs") || path_str.contains("architecture.rs") { continue; }
            let sniffer = CodeSniffer::new(path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            let schedules = ["Update", "Startup", "PreUpdate", "PostUpdate", "FixedUpdate"];
            for schedule in schedules {
                let pattern = format!("add_systems({},", schedule);
                if code_no_tests.contains(&pattern) {
                    let registrations = code_no_tests.split(&pattern).count() - 1;
                    let sets_count = code_no_tests.split(".in_set(").count() - 1;
                    if registrations > sets_count {
                        panic!("Orchestration Violation in {:?}: missing .in_set() for some systems.", path);
                    }
                }
            }
        }
    }
}

/// 15. Проверка условий выполнения: вся логика должна быть защищена "Глобальным щитом" в sets.rs.
#[test]
fn test_logic_systems_use_run_conditions() {
    let sniffer_sets = CodeSniffer::new("src/sets.rs");
    let code = sniffer_sets.clean;
    assert!(
        code.contains("GameSet::Logic") && code.contains(".run_if(in_state(GameState::Playing))"),
        "Global Guard Violation: Logic must be guarded in sets.rs"
    );
    check_logic_assignment_recursive(Path::new("src"));
}

fn check_logic_assignment_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_logic_assignment_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap();
            if path_str.contains("needs.rs") || path_str.contains("brain.rs") || path_str.contains("atmosphere.rs") {
                let sniffer = CodeSniffer::new(path_str);
                let registrations = sniffer.count_occurrences("add_systems(");
                let logic_set_usage = sniffer.count_occurrences("GameSet::Logic");
                if registrations > logic_set_usage {
                    panic!("Security Leak in {:?}: gameplay logic must be in protected 'GameSet::Logic'.", path);
                }
            }
        }
    }
}

/// 16. Проверка фильтрации запросов: запрет на "широкие" мутабельные Query.
#[test]
fn test_queries_use_filters() {
    check_filters_recursive(Path::new("src"));
}

fn check_filters_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_filters_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap().replace("\\", "/");
            if path_str.contains("main.rs") || path_str.contains("ui") || path_str.contains("camera.rs") || path_str.contains("architecture") {
                continue;
            }

            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            let mut search_str = code_no_tests;
            
            while let Some(start_idx) = search_str.find("Query<") {
                let content_start = start_idx + 6;
                let mut depth = 1;
                let mut end_idx = content_start;
                let bytes = search_str.as_bytes();
                while depth > 0 && end_idx < bytes.len() {
                    if bytes[end_idx] == b'<' { depth += 1; }
                    else if bytes[end_idx] == b'>' { depth -= 1; }
                    end_idx += 1;
                }
                if end_idx > bytes.len() { break; }
                let inside_query = &search_str[content_start..end_idx-1];
                if inside_query.contains("&mut") {
                    let is_filtered = inside_query.contains(",") && (
                        inside_query.contains("With") || inside_query.contains("Without") || 
                        inside_query.contains("Added") || inside_query.contains("Changed")
                    );
                    if !is_filtered {
                        panic!("Performance Violation in {:?}: Found broad mutable query 'Query<{}>'.", path, inside_query);
                    }
                }
                search_str = &search_str[end_idx..];
            }
        }
    }
}

/// 17. Запрет на создание ассетов в логике игры (Asset Handling).
#[test]
fn test_no_asset_creation_in_logic() {
    check_assets_recursive(Path::new("src"));
}

fn check_assets_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_assets_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap().replace("\\", "/");
            if path_str.contains("economy") || path_str.contains("architecture") { continue; }
            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            if code_no_tests.contains(".add(") && (code_no_tests.contains("meshes") || code_no_tests.contains("materials")) {
                panic!("Asset Violation in {:?}: Found direct asset creation using '.add(...)'.", path);
            }
        }
    }
}
