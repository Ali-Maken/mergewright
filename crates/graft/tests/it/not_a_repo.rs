//! Starting outside a repository: one plain line, no screen.

use std::process::Command;

/// The repository is opened before the terminal is touched, so a path that
/// is not a repository never enters the alternate screen — not even for the
/// length of the capability probe.
#[test]
fn a_path_that_is_not_a_repository_never_enters_the_alternate_screen() {
    let dir = std::env::temp_dir().join(format!("graft-not-a-repo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");

    let output = Command::new(env!("CARGO_BIN_EXE_graft"))
        .arg(&dir)
        // A git repository above the temp dir must not count.
        .env(
            "GIT_CEILING_DIRECTORIES",
            dir.parent().expect("temp has a parent"),
        )
        .output()
        .expect("graft runs");
    std::fs::remove_dir(&dir).expect("the scratch directory is empty");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("graft: ") && stderr.contains("not inside a git repository"),
        "{stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\u{1b}[?1049h"), "{stdout:?}");
}
