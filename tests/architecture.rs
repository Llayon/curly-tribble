use std::fs;
use std::path::Path;

/// Вспомогательная структура для анализа "чистого" кода без шума (комментариев и лишних пробелов).
/// Это сердце нашей системы архитектурного контроля мирового уровня.
struct CodeSniffer {
    raw: String,
    clean: String,
}

impl CodeSniffer {
    fn new(path: &str) -> Self {
        let raw = fs::read_to_string(path).expect(&format!("Critical Error: Could not read file at {}", path));
        
        // Удаляем однострочные комментарии и превращаем файл в одну строку для удобства поиска паттернов.
        // Это предотвращает ложные срабатывания на закомментированный код.
        let clean = raw.lines()
            .map(|line| line.split("//").next().unwrap_or("").trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
            
        Self { raw, clean }
    }

    fn contains_call(&self, pattern: &str) -> bool {
        self.clean.contains(pattern)
    }

    fn count_occurrences(&self, pattern: &str) -> usize {
        self.clean.split(pattern).count() - 1
    }
}

// ============================================================================
// БЛОК 1: ГВАРДЕЙЦЫ MAIN.RS (ВХОДНАЯ ТОЧКА)
// Эти тесты следят за тем, чтобы вход в игру оставался чистым и профессиональным.
// ============================================================================

/// 1. Проверка чистоты main.rs: разрешена только функция main.
#[test]
fn test_main_rs_is_clean() {
    let sniffer = CodeSniffer::new("src/main.rs");
    
    // Считаем все определения функций в чистом коде.
    let fn_count = sniffer.count_occurrences("fn ") + sniffer.count_occurrences("pub fn ");
    
    assert!(sniffer.clean.contains("fn main"), "Architectural Violation: main.rs must contain a 'main' function.");
    assert_eq!(fn_count, 1, "Architectural Violation: main.rs should ONLY contain 'fn main'. Found {} functions. All business logic must be moved to dedicated Plugins!", fn_count);
}

/// 2. Проверка конфигурации DefaultPlugins.
#[test]
fn test_default_plugins_are_configured() {
    let sniffer = CodeSniffer::new("src/main.rs");
    
    let required_configs = [
        ".set(WindowPlugin",
        ".set(bevy::log::LogPlugin",
        ".set(AssetPlugin",
    ];

    for config in required_configs {
        assert!(
            sniffer.contains_call(config), 
            "Architectural Violation: Missing mandatory DefaultPlugins configuration: {}. You must configure Window, Logs, and Assets via .set() as per World-Class standards.", 
            config
        );
    }
}

/// 3. Проверка наличия и качества Panic Hook.
#[test]
fn test_panic_hook_is_robust() {
    let sniffer = CodeSniffer::new("src/main.rs");
    
    assert!(
        sniffer.contains_call("std::panic::set_hook"),
        "Stability Violation: Panic hook is not set in main.rs. This is required to prevent silent crashes and ensure errors are visible in the CLI."
    );
    assert!(
        sniffer.clean.contains("error!(") || sniffer.clean.contains("println!("),
        "Stability Violation: Panic hook exists but doesn't seem to log anything. Use error!(...) macro to ensure the panic info reaches the Bevy logging system."
    );
}

/// 4. Проверка защиты инструментов отладки (cfg guards).
#[test]
fn test_debug_tools_are_conditional() {
    let sniffer = CodeSniffer::new("src/main.rs");
    
    assert!(
        sniffer.contains_call("#[cfg(debug_assertions)]"),
        "Engineering Violation: main.rs missing active #[cfg(debug_assertions)] attribute. Development tools (like inspectors) must never leak into release builds."
    );

    assert!(
        sniffer.clean.contains("#[cfg(debug_assertions)] {") || 
        sniffer.clean.contains("#[cfg(debug_assertions)] app"),
        "Engineering Violation: #[cfg(debug_assertions)] should guard a block of code or a plugin registration. Ensure it is not just a loose string in a comment."
    );
}

/// 5. Проверка эстетики регистрации плагинов (кортежи).
#[test]
fn test_plugins_are_grouped() {
    let sniffer = CodeSniffer::new("src/main.rs");
    
    assert!(
        sniffer.clean.contains(".add_plugins(("),
        "Style Violation: Plugins in main.rs should be grouped in tuples for better readability: .add_plugins((PluginA, PluginB, ...)). Avoid single .add_plugins() calls for custom logic."
    );
}

// ============================================================================
// БЛОК 2: ГВАРДЕЙЦЫ МОДУЛЬНОСТИ И ECS (ЛОГИКА)
// Эти тесты обеспечивают высокую производительность и правильную структуру ECS.
// ============================================================================

/// 6. Проверка "Все есть Плагин": каждый файл должен быть модульным.
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
            
            // Исключения для файлов, которые не обязаны быть плагинами
            if file_name == "main.rs" || file_name == "architecture.rs" {
                continue;
            }

            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            
            // Игнорируем пустые файлы и файлы, содержащие только декларации модулей (mod X;).
            if sniffer.clean.is_empty() || is_mod_only(&sniffer.clean) {
                continue;
            }

            assert!(
                sniffer.clean.contains("impl Plugin for") || sniffer.clean.contains("app.add_plugins"),
                "Modularity Violation: File {:?} does not seem to implement a Plugin or register others. Every logic file must follow the 'Everything is a Plugin' rule!", 
                path
            );
        }
    }
}

/// 7. Проверка использования Observers для выбора сущностей.
#[test]
fn test_pawn_selection_uses_observers() {
    let sniffer = CodeSniffer::new("src/pawn/mod.rs");
    
    assert!(
        sniffer.contains_call(".observe("),
        "Performance Violation: Pawn/Settler selection logic must use the .observe() mechanism for reactive behavior instead of polling in the Update loop."
    );

    assert!(
        sniffer.clean.contains("On<Add, Selected>"),
        "Reactivity Violation: Missing Observer trigger for adding the 'Selected' component. Use On<Add, T> for efficiency."
    );

    assert!(
        sniffer.clean.contains("With<Settler>"),
        "Theme/Query Violation: Selection observers should specifically filter for the 'Settler' component to maintain type safety."
    );

    assert!(
        !sniffer.clean.contains("add_systems(Update, update_selection_visuals"),
        "Performance Violation: Legacy 'update_selection_visuals' system found in Update loop. Polling for selection changes is forbidden in World-Class Bevy code!"
    );
}

/// 8. Проверка реализации и интеграции GameState.
#[test]
fn test_game_state_is_implemented() {
    // Проверка физического наличия модуля
    assert!(Path::new("src/game_state.rs").exists(), "Architecture Violation: Missing 'src/game_state.rs' module. State management is mandatory.");

    // Проверка регистрации в главном файле
    let sniffer_main = CodeSniffer::new("src/main.rs");
    assert!(sniffer_main.clean.contains("GameStatePlugin"), "Architecture Violation: GameStatePlugin must be registered in main.rs to enable state-based logic.");

    // Проверка того, что системы реально используют состояния для защиты
    let sniffer_needs = CodeSniffer::new("src/pawn/needs.rs");
    assert!(
        sniffer_needs.clean.contains("run_if(in_state(GameState::Playing))"),
        "Logic Violation: Vital game systems (like hunger update) must be guarded by GameState::Playing condition to prevent logic leaks during Loading or Pause."
    );
}

/// 9. Запрет на использование "ручных" булевых флагов паузы/состояния.
#[test]
fn test_no_boolean_state_flags() {
    check_for_forbidden_state_flags(Path::new("src"));
}

fn check_for_forbidden_state_flags(dir: &Path) {
    // Список анти-паттернов флагов, которые должны быть заменены на GameState
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
                    panic!(
                        "Architectural Violation: Forbidden boolean flag '{}' found in {:?}. You must use the global GameState system instead of manual bool flags!", 
                        flag, path
                    );
                }
            }
        }
    }
}

// ============================================================================
// БЛОК 3: ГВАРДЕЙЦЫ КОНКУРЕНТНОСТИ И РАЗВЯЗКИ (DECOUPLING)
// Эти тесты следят за тем, чтобы модули были независимы и работали параллельно.
// ============================================================================

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
            
            // Исключения: Файлы, где доступ к World необходим для настройки или тестов.
            if path_str.contains("main.rs") || path_str.contains("architecture.rs") {
                continue;
            }

            let sniffer = CodeSniffer::new(path_str);
            // Проверяем только код вне тестовых блоков
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

            // Паттерны прямого (эксклюзивного) доступа к миру
            let forbidden = ["&mut World", ".world_mut()", "world: &mut"];
            
            for pattern in forbidden {
                if code_no_tests.contains(pattern) {
                    // Исключение: Разрешаем .world_mut() только внутри метода build плагина
                    if code_no_tests.contains("impl Plugin for") && pattern == ".world_mut()" {
                        continue;
                    }
                    
                    panic!(
                        "Concurrency Violation: Forbidden direct World access '{}' found in {:?}. Systems must use 'Commands' to allow Bevy to run them in parallel!", 
                        pattern, path
                    );
                }
            }
        }
    }
}

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
            
            // "Белый список" слоев вывода, которым разрешено прямое общение с пользователем.
            let allowed_to_log = ["src/main.rs", "src/ui.rs", "src/events.rs", "src/game_state.rs", "src/sets.rs"];
            if allowed_to_log.iter().any(|&p| path_str.contains(p)) {
                continue;
            }

            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            
            // Запрещенные макросы прямого вывода в ядре логики
            let forbidden_macros = ["info!", "warn!", "error!", "println!"];
            for macro_name in forbidden_macros {
                if code_no_tests.contains(macro_name) {
                    panic!(
                        "Architectural Violation: Core logic file {:?} uses direct logging '{}'. You must send a 'Message' event and let the UI layer handle the output!", 
                        path, macro_name
                    );
                }
            }

            // Проверка зависимости от UI типов (ядро не должно знать про интерфейс)
            if code_no_tests.contains("bevy::ui") || code_no_tests.contains("Interaction") {
                 panic!(
                    "Dependency Violation: Core logic file {:?} depends on UI types. Keep the simulation decoupled from the interface using the Observer pattern!", 
                    path
                );
            }
        }
    }
}

// ============================================================================
// БЛОК 4: ГВАРДЕЙЦЫ ПАТТЕРНОВ МИРОВОГО УРОВНЯ
// Эти тесты принуждают к использованию самых современных и производительных техник Bevy.
// ============================================================================

/// 12. Запрет на использование флагов внутри компонентов (используйте Marker Components).
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
            
            // Пропускаем мета-файлы
            if path_str.contains("main.rs") || path_str.contains("events.rs") {
                continue;
            }

            for line in sniffer.clean.split(";") {
                let trimmed = line.trim();
                if trimmed.contains(": bool") {
                    let classification_prefixes = ["is_", "has_", "can_", "should_"];
                    for prefix in classification_prefixes {
                        if trimmed.contains(prefix) {
                            panic!(
                                "ECS Violation: Found classification flag '{}' in {:?}. World-Class Rule: Use a Marker Component (empty struct) instead for O(1) query performance!", 
                                trimmed, path
                            );
                        }
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
            
            // Слой вывода может спавнить простые объекты (UI, свет) без бандлов
            let output_layer = ["src/main.rs", "src/ui.rs", "src/events.rs", "src/game_state.rs"];
            if output_layer.iter().any(|&p| path_str.contains(p)) {
                continue;
            }

            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            
            // Ищем попытки спавна сущностей через анонимные кортежи (CompA, CompB)
            let forbidden_patterns = ["spawn((", "insert(("];
            for pattern in forbidden_patterns {
                if code_no_tests.contains(pattern) {
                     panic!(
                        "Integrity Violation in {:?}: Found anonymous component tuple in '{}'. You must use named Bundle structs to ensure entity atomicity and data integrity!", 
                        path, pattern
                    );
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
            if path_str.contains("main.rs") || path_str.contains("sets.rs") || path_str.contains("architecture.rs") {
                continue;
            }

            let sniffer = CodeSniffer::new(path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

            // Проверяем все основные расписания Bevy
            let schedules = ["Update", "PreUpdate", "PostUpdate", "FixedUpdate"];
            
            for schedule in schedules {
                let pattern = format!("add_systems({},", schedule);
                if code_no_tests.contains(&pattern) {
                    let registrations = code_no_tests.split(&pattern).count() - 1;
                    let sets_count = code_no_tests.split(".in_set(").count() - 1;

                    if registrations > sets_count {
                        panic!(
                            "Orchestration Violation in {:?}: Found {} system registrations in '{}', but only {} are assigned to a GameSet. Every system must have a designated place in the pipeline!", 
                            path, registrations, schedule, sets_count
                        );
                    }
                }
            }
        }
    }
}

/// 15. Проверка фильтрации запросов: запрет на "широкие" мутабельные Query.
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
            
            // Исключения для систем, где широкие запросы оправданы
            if path_str.contains("main.rs") || path_str.contains("ui.rs") || path_str.contains("camera.rs") || path_str.contains("architecture.rs") {
                continue;
            }

            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            let mut search_str = code_no_tests;
            
            // Ищем Query<...> и анализируем вложенность скобок для поиска фильтров
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
                
                // Если запрос запрашивает мутабельный доступ, он ОБЯЗАН иметь фильтр (With/Without)
                if inside_query.contains("&mut") {
                    let mut comma_found = false;
                    let (mut b_depth, mut p_depth) = (0, 0);
                    let mut filter_part = "";

                    for (i, b) in inside_query.as_bytes().iter().enumerate() {
                        match *b {
                            b'<' => b_depth += 1, b'>' => b_depth -= 1,
                            b'(' => p_depth += 1, b')' => p_depth -= 1,
                            b',' if b_depth == 0 && p_depth == 0 => {
                                comma_found = true;
                                filter_part = &inside_query[i+1..];
                                break;
                            }
                            _ => {}
                        }
                    }

                    if !comma_found {
                        panic!(
                            "Performance Violation in {:?}: Found broad mutable query 'Query<{}>'. You must use With<Marker> or Without<Marker> to filter entities at the ECS level!", 
                            path, inside_query
                        );
                    }

                    let valid_filters = ["With", "Without", "Added", "Changed", "Or", "And", "AnyOf"];
                    if !valid_filters.iter().any(|f| filter_part.contains(f)) {
                        panic!(
                            "Performance Violation in {:?}: Query 'Query<{}>' has a second parameter, but it doesn't look like a valid Bevy Filter. Use With, Without, etc.", 
                            path, inside_query
                        );
                    }
                }
                
                search_str = &search_str[end_idx..];
            }
        }
    }
}

/// 16. Запрет на создание ассетов в логике игры (Asset Handling).
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
            
            // Ассеты можно создавать только в модуле ресурсов и в тестах.
            if path_str.contains("economy.rs") || path_str.contains("architecture.rs") {
                continue;
            }

            let sniffer = CodeSniffer::new(&path_str);
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");
            
            // Ищем прямой вызов .add() для коллекций ассетов (meshes/materials).
            if code_no_tests.contains(".add(") && (code_no_tests.contains("meshes") || code_no_tests.contains("materials")) {
                panic!(
                    "Asset Violation in {:?}: Found direct asset creation using '.add(...)'. World-Class Rule: Pre-load all assets into 'GameAssets' resource during startup and clone Handles!", 
                    path
                );
            }
        }
    }
}

fn is_mod_only(clean_code: &str) -> bool {
    let parts: Vec<&str> = clean_code.split(' ').collect();
    parts.iter().all(|&p| p == "mod" || p == "pub" || p == "use" || p.ends_with(';') || p.contains("::"))
}
