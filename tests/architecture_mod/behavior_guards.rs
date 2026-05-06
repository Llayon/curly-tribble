use std::path::Path;
use std::fs;
use crate::utils::CodeSniffer;

/// 25. ИСКЛЮЧИТЕЛЬНОСТЬ ПОВЕДЕНИЯ (Exclusivity Guard): Запрет на ручной .insert() маркеров поведения.
/// Все переключения состояний должны идти через .switch_behavior::<T>().
#[test]
fn test_safe_behavior_transitions() {
    check_behavior_usage_recursive(Path::new("src"));
}

fn check_behavior_usage_recursive(dir: &Path) {
    // 1. ДИНАМИЧЕСКОЕ ОТКРЫТИЕ: Выясняем, какие поведения существуют в проекте
    let behaviors_file = "src/pawn/behaviors.rs";
    let sniffer_def = CodeSniffer::new(behaviors_file);
    let mut behaviors = Vec::new();
    
    // Ищем все 'pub struct Name' в behaviors.rs
    for part in sniffer_def.clean.split("struct ") {
        let name = part.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("");
        if !name.is_empty() && name.chars().next().unwrap().is_uppercase() {
            if name != "AllBehaviors" { // Исключаем вспомогательный алиас
                behaviors.push(name.to_string());
            }
        }
    }

    // 2. РЕКУРСИВНАЯ ПРОВЕРКА
    check_dir_recursive(dir, &behaviors);
}

fn check_dir_recursive(dir: &Path, behaviors: &[String]) {
    for entry in fs::read_dir(dir).expect("Could not read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            check_dir_recursive(&path, behaviors);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap();
            
            // Игнорируем файлы определений и тесты
            if path_str.contains("behaviors.rs") || path_str.contains("architecture") { continue; }

            let sniffer = CodeSniffer::new(path_str);
            let code = sniffer.clean;

            for behavior in behaviors {
                // Запрещаем ручную вставку (через insert или в кортеже spawn)
                let forbidden_insert = format!("insert({})", behavior);
                let forbidden_spawn = format!("spawn(({}", behavior);
                
                // Запрещаем ручное удаление (оно должно быть только в switch_behavior)
                let forbidden_remove = format!("remove::<{}", behavior);
                
                if code.contains(&forbidden_insert) || code.contains(&forbidden_spawn) || code.contains(&forbidden_remove) {
                    panic!(
                        "Exclusivity Violation in {:?}: Found manual manipulation of Behavior Marker '{}'. \
                        World-Class Rule: You MUST use 'commands.entity(e).switch_behavior::<T>()' for all task transitions!",
                        path, behavior
                    );
                }
            }
        }
    }
}
