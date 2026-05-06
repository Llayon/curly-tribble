use crate::utils::CodeSniffer;
use std::fs;
use std::path::Path;

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
            // (Берем всё до первого упоминания #[cfg(test)])
            let code_no_tests = sniffer.clean.split("#[cfg(test)]").next().unwrap_or("");

            // Список запрещенных "бомб замедленного действия"
            let forbidden_patterns = [
                (
                    ".unwrap()",
                    "Immediate panic if None/Err. Use 'if let' or 'match'.",
                ),
                (
                    ".expect(",
                    "Immediate panic with message. Use proper error handling.",
                ),
                (
                    ".unwrap_err()",
                    "Immediate panic if Ok. Reserved for tests only.",
                ),
                (
                    "panic!",
                    "Manual crash detected. All errors must be handled gracefully.",
                ),
                (
                    "todo!",
                    "Placeholder found! Logic must be fully implemented for production.",
                ),
                (
                    "unimplemented!",
                    "Logic missing! Features must be complete.",
                ),
                (
                    "unreachable!",
                    "Dangerous assumption! Handle the 'impossible' case safely.",
                ),
            ];

            for (pattern, recommendation) in forbidden_patterns {
                if code_no_tests.contains(pattern) {
                    panic!(
                        "Stability Violation in {:?}: Found forbidden '{}'. \
                        Recommendation: {} \
                        World-Class Rule: Application must be crash-proof and feature-complete!",
                        path, pattern, recommendation
                    );
                }
            }
        }
    }
}
