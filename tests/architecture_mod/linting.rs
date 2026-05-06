use crate::utils::CodeSniffer;
use std::process::Command;

/// 28. ГВАРДЕЙЦЫ КАЧЕСТВА: Проверка наличия clippy-аттрибутов в main.rs.
#[test]
fn test_clippy_guards_presence() {
    let sniffer = CodeSniffer::new("src/main.rs");
    let code = sniffer.clean;

    assert!(
        code.contains("#![deny(clippy::all)]") && code.contains("#![deny(clippy::pedantic)]"),
        "Quality Violation: main.rs MUST contain global clippy guards: #![deny(clippy::all)] and #![deny(clippy::pedantic)]!"
    );
}

/// 29. ГВАРДЕЙЦЫ СТИЛЯ: Проверка форматирования через cargo fmt.
#[test]
fn test_code_formatting() {
    // Пропускаем, если мы не в git-репозитории или нет cargo (для CI)
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
