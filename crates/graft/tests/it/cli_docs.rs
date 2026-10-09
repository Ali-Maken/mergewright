//! The binary prints what the tables say (m8-polish-and-ship/23): `--help`
//! is `usage(FLAGS)`, and `--generate-man` is `man_page(…)`, byte for byte.
#![allow(
    clippy::expect_used,
    reason = "a binary that cannot be run has nothing to test; the panic names why"
)]

use std::process::Command;

use graft::cli::{CONFIG_KEYS, FLAGS, man_page, usage};

fn stdout_of(flag: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_graft"))
        .arg(flag)
        .output()
        .expect("graft runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

#[test]
fn help_prints_usage_of_flags() {
    assert_eq!(stdout_of("--help"), format!("{}\n", usage(FLAGS)));
    assert_eq!(stdout_of("-h"), format!("{}\n", usage(FLAGS)));
}

#[test]
fn generate_man_prints_the_man_page_and_nothing_else() {
    let page = man_page(FLAGS, graft_ui::keymap::defaults(), CONFIG_KEYS);
    assert_eq!(stdout_of("--generate-man"), page);
}

/// `graft --generate-man | head -0`: the reader is gone before the first
/// write. Rust ignores SIGPIPE, so the write fails with `BrokenPipe`; that is
/// a clean exit 0, never a panic (review m8-hygiene-release m6).
fn into_closed_pipe(flag: &str) -> std::process::Output {
    let (reader, writer) = std::io::pipe().expect("pipe");
    drop(reader);
    Command::new(env!("CARGO_BIN_EXE_graft"))
        .arg(flag)
        .stdout(writer)
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("graft runs")
}

#[test]
fn printing_into_a_closed_pipe_exits_clean() {
    for flag in ["--generate-man", "--help", "--version"] {
        let output = into_closed_pipe(flag);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "{flag}: stderr: {stderr}");
        assert!(!stderr.contains("panicked"), "{flag}: {stderr}");
    }
}
