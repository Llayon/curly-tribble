use std::path::Path;
use std::fs;
use crate::utils::CodeSniffer;

/// 27. ЗАПРЕТ АНОНИМНЫХ ОЧЕРЕДЕЙ: Запрет на использование анонимных замыканий в .queue().
/// Требует создания именованных структур Command (Plugins 2.0 Optimal).
#[test]
fn test_no_anonymous_command_queues() {
    check_queues_recursive(Path::new("src"));
}

fn check_queues_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Could not read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            check_queues_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let sniffer = CodeSniffer::new(path.to_str().unwrap());
            let code = sniffer.clean;

            // Ищем паттерны анонимных замыканий в очереди команд
            // Например: commands.queue(|world| ...)
            if code.contains(".queue(|") || code.contains(".queue(move |") {
                panic!(
                    "Plugins 2.0 Violation in {:?}: Found anonymous closure in .queue(). \
                    World-Class Rule: Use named 'Command' structs for better debugging, profiling, and reflection!",
                    path
                );
            }
        }
    }
}
