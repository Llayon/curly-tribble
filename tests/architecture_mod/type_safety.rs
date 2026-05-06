use std::path::Path;
use std::fs;
use crate::utils::CodeSniffer;

/// 24. СТРОГАЯ ТИПИЗАЦИЯХарактеристик: Запрет на использование f32 в игровых параметрах.
/// Требует использования инкапсулированных Newtypes.
#[test]
fn test_strict_type_driven_stats() {
    check_stat_types_recursive(Path::new("src"));
}

fn check_stat_types_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_stat_types_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            let code = sniffer.clean;
            
            // Паттерн: публичная структура-кортеж с публичным примитивом
            // Например: pub struct Hunger(pub f32);
            let primitives = ["f32", "f64", "u32", "i32", "u64", "i64", "usize", "isize"];
            
            for primitive in primitives {
                let pattern = format!("(pub {})", primitive);
                if code.contains("pub struct") && code.contains(&pattern) {
                    panic!(
                        "Type-Driven Design Violation in {:?}: Found public primitive field '{}'. \
                        World-Class Rule: All game stats and domain-specific numbers MUST be Opaque Newtypes (private fields) with encapsulated logic!",
                        path, pattern
                    );
                }
            }
        }
    }
}
