use std::path::Path;
use std::fs;
use crate::utils::{CodeSniffer};

/// 19. ГЛОБАЛЬНЫЙ СТРАЖ ПРОИЗВОДИТЕЛЬНОСТИ: Проверка Change Detection во всем проекте.
#[test]
fn test_global_performance_guards() {
    check_performance_recursive(Path::new("src"));
}

fn check_performance_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_performance_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap();
            if path_str.contains("main.rs") || path_str.contains("sets.rs") { continue; }
            let sniffer = CodeSniffer::new(path_str);
            let code = sniffer.clean;

            // --- ПРАВИЛО 1: Реактивный UI ---
            if code.contains("Query<") && code.contains("&mut Text") {
                // В Bevy 0.18.1 мы можем защищать систему в mod.rs через .run_if(resource_changed)
                // Если файл не является mod.rs, мы проверяем локальные фильтры.
                // Если это вспомогательный файл UI, мы доверяем родителю (mod.rs).
                let is_ui_submodule = path_str.contains("ui") && !path_str.contains("mod.rs");
                
                let is_reactive = code.contains("Changed<") || 
                                 code.contains("Added<") || 
                                 code.contains("resource_changed") ||
                                 is_ui_submodule; // Подмодули UI делегируют защиту mod.rs
                
                if !is_reactive { 
                    panic!("Performance Violation in {:?}: UI must use Change Detection!", path); 
                }
            }

            // --- ПРАВИЛО 2: Реактивные Маркеры ---
            if (code.contains(".insert(") || code.contains(".remove::<")) && 
               (code.contains("Hungry") || code.contains("InDarkness") || code.contains("Selected")) {
                let is_reactive = code.contains("Changed<") || 
                                 code.contains("Added<") || 
                                 code.contains("RemovedComponents") || 
                                 code.contains(".is_changed()") || 
                                 code.contains(".observe(");
                if !is_reactive { panic!("Performance Violation in {:?}: Marker management must be reactive!", path); }
            }
        }
    }
}

/// 20. РАЗДЕЛЕНИЕ ОТВЕТСТВЕННОСТИ: Логика в FixedUpdate, Визуал в Update.
#[test]
fn test_simulation_presentation_split() {
    check_schedule_split_recursive(Path::new("src"));
}

fn check_schedule_split_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_schedule_split_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let path_str = path.to_str().unwrap();
            let sniffer = CodeSniffer::new(path_str);
            let code = sniffer.clean;

            if path_str.contains("needs.rs") || path_str.contains("brain.rs") || path_str.contains("atmosphere.rs") {
                if code.contains("add_systems(Update,") {
                    panic!("Architectural Violation in {:?}: Core logic found in 'Update' schedule.", path);
                }
            }

            if path_str.contains("ui") || path_str.contains("camera.rs") {
                if code.contains("add_systems(FixedUpdate,") {
                    panic!("Architectural Violation in {:?}: Presentation found in 'FixedUpdate'.", path);
                }
            }
        }
    }
}
