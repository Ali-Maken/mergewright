//! Piped stdout is plain text, never escape soup (ticket m1-the-graph/19;
//! acceptance criterion 4). Detection never probes a terminal that is not a
//! tty, so a fixture opened with stdout piped away must render on the ASCII
//! tier and print without a single escape byte — `graft <fixture> | cat`
//! must be safe to grep.

use std::process::Command;

#[test]
fn piped_stdout_never_contains_an_escape_byte() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Linear);

    let output = Command::new(env!("CARGO_BIN_EXE_graft"))
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
}
