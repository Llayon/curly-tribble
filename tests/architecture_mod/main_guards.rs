use crate::utils::CodeSniffer;

/// 1. Проверка чистоты main.rs: разрешена только функция main.
#[test]
fn test_main_rs_is_clean() {
    let sniffer = CodeSniffer::new("src/main.rs");
    let fn_count = sniffer.count_occurrences("fn ") + sniffer.count_occurrences("pub fn ");
    assert!(
        sniffer.clean.contains("fn main"),
        "Architectural Violation: main.rs must contain a 'main' function."
    );
    assert_eq!(
        fn_count, 1,
        "Architectural Violation: main.rs should ONLY contain 'fn main'. Found {} functions.",
        fn_count
    );
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
            "Architectural Violation: Missing mandatory DefaultPlugins configuration: {}.",
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
        "Stability Violation: Panic hook is not set in main.rs."
    );
}

/// 4. Проверка защиты инструментов отладки (cfg guards).
#[test]
fn test_debug_tools_are_conditional() {
    let sniffer = CodeSniffer::new("src/main.rs");
    assert!(
        sniffer.contains_call("#[cfg(debug_assertions)]"),
        "Engineering Violation: main.rs missing active #[cfg(debug_assertions)] attribute."
    );
}

/// 5. Проверка эстетики регистрации плагинов (кортежи).
#[test]
fn test_plugins_are_grouped() {
    let sniffer = CodeSniffer::new("src/main.rs");
    assert!(
        sniffer.clean.contains(".add_plugins(("),
        "Style Violation: Plugins in main.rs should be grouped in tuples."
    );
}
