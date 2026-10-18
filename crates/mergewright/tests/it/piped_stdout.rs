//! Piped stdout is plain text, never escape soup (ticket m1-the-graph/19;
//! acceptance criterion 4). Detection never probes a terminal that is not a
//! tty, so a fixture opened with stdout piped away must render on the ASCII
//! tier and print without a single escape byte — `graft <fixture> | cat`
//! must be safe to grep.

use std::process::Command;

#[test]
fn piped_stdout_never_contains_an_escape_byte() {
    let dir = mergewright_fixtures::borrow(mergewright_fixtures::Shape::Linear);

    let output = Command::new(env!("CARGO_BIN_EXE_mwr"))
        .arg(&dir)
        .output()
        .expect("graft runs");

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = output.stdout;
    assert!(
        !stdout.contains(&0x1b),
        "piped stdout contained an escape byte: {:?}",
        String::from_utf8_lossy(&stdout)
    );
    assert!(!stdout.is_empty(), "expected a plain-text frame");

    // `headless_settled` must wait for `history.is_some()` before printing:
    // counting `None` as settled (M1 review, M4, KNOWN_GAPS.md:181) prints an
    // "opening…" frame with no rows the instant the loop starts, before the
    // walk has said anything. The tip commit's subject can only be on screen
    // once the walk has actually laid out a row for it.
    let text = String::from_utf8_lossy(&stdout);
    assert!(
        text.contains("Commit 200"),
        "expected the tip commit's row, got: {text}"
    );
}

/// A repository that fails to open must still print its error and exit,
/// never hang waiting for a `history` that will never arrive (M1 review,
/// M4). `run_headless`'s open failure sets `Quit::Fatal` independently of
/// `headless_settled`, so this is a regression guard on that path staying
/// wired up, not a timing race: `Command::output` blocks until the child
/// exits on its own, so a hang here would hang the test, not just Mergewright.
#[test]
fn a_repository_that_fails_to_open_exits_with_its_error_never_hangs() {
    let dir = std::env::temp_dir().join(format!("graft-piped-not-a-repo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");

    let output = Command::new(env!("CARGO_BIN_EXE_mwr"))
        .arg(&dir)
        .env(
            "GIT_CEILING_DIRECTORIES",
            dir.parent().expect("temp has a parent"),
        )
        .output()
        .expect("graft runs and exits");
    std::fs::remove_dir(&dir).expect("the scratch directory is empty");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("mwr: ") && stderr.contains("not inside a git repository"),
        "{stderr}"
    );
    assert!(output.stdout.is_empty(), "expected no frame at all");
}
