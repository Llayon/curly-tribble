use std::path::Path;
use std::fs;
use crate::utils::CodeSniffer;

/// 22. НУЛЕВАЯ ТЕРПИМОСТЬ К UNWRAP: Запрет на .unwrap() и .expect() в продакшн-коде.
/// Это предотвращает панику и краши приложения.
#[test]
fn test_no_unwraps_in_production_code() {
    check_unwraps_recursive(Path::new("src"));
}

fn check_unwraps_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Could not read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            check_unwraps_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            
            // Анализируем только код ВНЕ тестовых блоков
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

            if code_no_tests.contains(".unwrap()") {
                panic!(
                    "Stability Violation in {:?}: Found forbidden '.unwrap()'. Use defensive patterns like 'if let' or 'match'!",
                    path
                );
            }

            if code_no_tests.contains(".expect(") {
                panic!(
                    "Stability Violation in {:?}: Found forbidden '.expect()'. Application must be crash-proof!",
                    path
                );
            }
        }
    }
}
