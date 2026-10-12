//! The binary's askpass mode (m4-remotes/06): with `TRIB_ASKPASS_SOCKET`
//! set and one argument, `graft` asks the running Graft over the socket,
//! prints the answer for git and exits — 0 answered, 1 cancelled, 2 anything
//! else — without touching the terminal.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};

use graft_git::run::askpass::{AskPass, AskPassServer, Secret};

/// Answers every prompt with `answer` (or cancels, when `None`) and records
/// the prompts it was asked.
struct Scripted {
    answer: Option<&'static str>,
    asked: Mutex<Vec<String>>,
}

impl AskPass for Scripted {
    fn ask(&self, prompt: &str) -> Option<Secret> {
        self.asked
            .lock()
            .expect("no test thread panicked holding the lock")
            .push(prompt.to_owned());
        self.answer
            .map(|answer| Secret::new(answer.as_bytes().to_vec()))
    }
}

fn graft() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mwr"))
}

/// Runs the binary the way git would: the prompt as its one argument, the
/// socket in the environment, no terminal on any of its streams.
fn run_helper(socket: &std::path::Path, prompt: &str) -> Output {
    Command::new(graft())
        .arg(prompt)
        .env(graft::cli::ASKPASS_SOCKET_VAR, socket)
        .env_remove("TERM")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("the graft binary runs")
}

fn serve(answer: Option<&'static str>) -> (AskPassServer, Arc<Scripted>) {
    let handler = Arc::new(Scripted {
        answer,
        asked: Mutex::new(Vec::new()),
    });
    let server = AskPassServer::spawn(graft(), handler.clone()).expect("the askpass server starts");
    (server, handler)
}

#[test]
fn binary_answers_over_socket() {
    let (server, handler) = serve(Some("hunter2"));

    let out = run_helper(server.socket_path(), "Password for 'http://u@127.0.0.1:1':");

    assert_eq!(out.status.code(), Some(0), "answered is exit 0");
    assert_eq!(
        out.stdout, b"hunter2\n",
        "stdout is the answer and a newline"
    );
    assert!(out.stderr.is_empty(), "nothing on stderr: {:?}", out.stderr);
    assert_eq!(
        *handler.asked.lock().expect("lock"),
        vec!["Password for 'http://u@127.0.0.1:1':".to_owned()],
        "the running Graft saw git's prompt verbatim"
    );
    server.close().expect("the server closes");
}

#[test]
fn binary_exits_1_with_nothing_on_stdout_when_cancelled() {
    let (server, _handler) = serve(None);

    let out = run_helper(server.socket_path(), "Password for 'http://u@h':");

    assert_eq!(out.status.code(), Some(1), "cancelled is exit 1");
    assert!(out.stdout.is_empty(), "no answer printed: {:?}", out.stdout);
    server.close().expect("the server closes");
}

#[test]
fn binary_exits_2_naming_the_error_when_graft_is_gone() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let socket = dir.path().join("ask.sock");

    let out = run_helper(&socket, "Password for 'http://u@h':");

    assert_eq!(out.status.code(), Some(2), "any other failure is exit 2");
    assert!(out.stdout.is_empty(), "no answer printed: {:?}", out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(stderr.lines().count(), 1, "one line naming it: {stderr:?}");
    assert!(stderr.starts_with("mwr: askpass:"), "{stderr:?}");
    // No terminal escape: the helper never took the terminal over.
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
}
