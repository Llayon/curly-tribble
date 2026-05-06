use std::path::Path;
use std::fs;
use crate::utils::CodeSniffer;

/// 26. ГРАФОВАЯ АРХИТЕКТУРА (Relations Guard): Запрет на хранение Entity в компонентах.
/// Требует использования системы отношений Bevy 0.18.1.
#[test]
fn test_no_raw_entity_references_in_components() {
    check_entity_refs_recursive(Path::new("src"));
}

fn check_entity_refs_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Could not read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            check_entity_refs_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap();
            
            // Исключения: Файлы определений отношений и мета-файлы
            if path_str.contains("relations.rs") || path_str.contains("main.rs") { continue; }

            let sniffer = CodeSniffer::new(path_str);
            let code = sniffer.clean;

            // Расширенный список запрещенных паттернов прямого хранения Entity
            let forbidden_patterns = [
                ": Entity",       // Поля структур
                "(Entity)",       // Кортежные структуры
                "Vec<Entity>",    // Списки (самый опасный источник dangling pointers)
                "Option<Entity>", // Опциональные ссылки
            ];

            if code.contains("struct ") {
                for pattern in forbidden_patterns {
                    if code.contains(pattern) {
                        panic!(
                            "Graph Architecture Violation in {:?}: Found forbidden raw '{}'. \
                            World-Class Rule: Use Bevy 0.18.1 Entity Relations (Targeting, OwnedBy, etc.) instead of raw IDs. \
                            This ensures automatic cleanup and graph performance!",
                            path, pattern
                        );
                    }
                }
            }
        }
    }
}
