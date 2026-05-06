use std::path::Path;
use std::fs;

/// 21. ПРОВЕРКА ИНКАПСУЛЯЦИИ: Ограничение размера файлов (макс. 300 строк).
/// Это гарантирует, что логика остается дробной и "LLM-friendly".
#[test]
fn test_file_line_count_limit() {
    check_line_count_recursive(Path::new("src"));
}

fn check_line_count_recursive(dir: &Path) {
    let line_limit = 300;

    for entry in fs::read_dir(dir).expect("Could not read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            check_line_count_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(&path).expect("Could not read file");
            let line_count = content.lines().count();

            assert!(
                line_count <= line_limit,
                "Encapsulation Violation in {:?}: File has {} lines, which exceeds the limit of {}. Break this file into smaller sub-modules!",
                path, line_count, line_limit
            );
        }
    }
}
