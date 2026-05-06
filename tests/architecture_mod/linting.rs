use std::fs;
use std::process::Command;

/// 28. ГВАРДЕЙЦЫ КАЧЕСТВА: Проверка линтов в Cargo.toml.
#[test]
fn test_clippy_guards_presence() {
    let content = fs::read_to_string("Cargo.toml").expect("Could not read Cargo.toml");

    assert!(
        content.contains("all = ") && content.contains("pedantic = "),
        "Quality Violation: Cargo.toml MUST contain global clippy guards: all and pedantic!"
    );
}

/// 29. ГВАРДЕЙЦЫ СТИЛЯ: Проверка форматирования через cargo fmt.
#[test]
fn test_code_formatting() {
    let output = Command::new("cargo")
        .args(["fmt", "--", "--check"])
        .output();

    if let Ok(output) = output {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!(
                "Formatting Violation: Code is not formatted! Run 'cargo fmt' to fix this.\nDetails: {}",
                stderr
            );
        }
    }
}
