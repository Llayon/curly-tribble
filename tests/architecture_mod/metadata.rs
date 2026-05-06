use std::process::Command;

/// 18. ТЕСТ: Проверка стандарта коммит-сообщений (AI-Friendly Conventional Commits).
#[test]
fn test_commit_message_standard() {
    let output = Command::new("git").args(["log", "-1", "--pretty=%B"]).output();
    let Ok(output) = output else { return; };
    if !output.status.success() { return; }
    let message = String::from_utf8_lossy(&output.stdout);
    let required_blocks = ["What:", "Why:"];
    for block in required_blocks {
        assert!(message.contains(block), "Commit History Violation: The latest commit message is missing the '{}' block.", block);
    }
}
