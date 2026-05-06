use std::path::Path;
use std::fs;
use crate::utils::CodeSniffer;

/// 23. ЯВНЫЕ ЗАВИСИМОСТИ (Explicit DI): Запрет на использование глобальных переменных.
/// Все состояние должно быть внутри ECS (Resources/Components).
#[test]
fn test_no_hidden_global_states() {
    check_globals_recursive(Path::new("src"));
}

fn check_globals_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Could not read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            check_globals_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            let code = sniffer.clean;

            // Современный список запрещенных паттернов скрытого состояния (Singletons)
            let forbidden_patterns = [
                "static mut ",      // Изменяемое глобальное состояние
                "lazy_static!",     // Устаревший макрос для ленивой инициализации
                "thread_local!",    // Локальное состояние потока (скрытое для ECS)
                "OnceLock::new",    // Современный способ создания синглтонов (Rust 1.70+)
                "LazyLock::new",    // Современный способ (Rust 1.80+)
                "once_cell",        // Популярная библиотека для скрытых синглтонов
            ];
            
            for pattern in forbidden_patterns {
                if code.contains(pattern) {
                    panic!(
                        "Explicit DI Violation in {:?}: Found hidden state via '{}'. \
                        World-Class Rule: All game state MUST be stored in Bevy Resources or Components. \
                        Global singletons break parallel execution and test isolation!",
                        path, pattern
                    );
                }
            }
        }
    }
}
