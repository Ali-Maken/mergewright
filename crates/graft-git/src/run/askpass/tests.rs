use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use super::*;

/// Answers every prompt with a fixed reply, and remembers the prompts.
struct Fixed {
    reply: Option<&'static [u8]>,
    prompts: Mutex<Vec<String>>,
}

impl Fixed {
    fn new(reply: Option<&'static [u8]>) -> Arc<Self> {
        Arc::new(Self {
            reply,
            prompts: Mutex::new(Vec::new()),
        })
    }
}

impl AskPass for Fixed {
    fn ask(&self, prompt: &str) -> Option<Secret> {
        self.prompts
            .lock()
            .expect("test lock")
            .push(prompt.to_owned());
        self.reply.map(|bytes| Secret::new(bytes.to_vec()))
    }
}

fn server(handler: Arc<dyn AskPass>) -> AskPassServer {
    AskPassServer::spawn(PathBuf::from("/nonexistent/graft"), handler).expect("server starts")
}

#[test]
fn round_trip() {
    let handler = Fixed::new(Some(b"s3c"));
    let server = server(handler.clone());
    let socket = server.socket_path().to_owned();

    let client = thread::spawn(move || {
        let mut out = Vec::new();
        respond(&socket, "Password for 'http://u@h':", &mut out).map(|()| out)
    });
    let out = client
        .join()
        .expect("client thread")
        .expect("the helper got an answer");

    assert_eq!(out, b"s3c\n", "the answer and a newline, nothing else");
    assert_eq!(
        *handler.prompts.lock().expect("test lock"),
        vec!["Password for 'http://u@h':".to_owned()]
    );
    server.close().expect("closes cleanly");
}

#[test]
fn cancelled_answers_status_1() {
    let server = server(Fixed::new(None));

    // Through the helper: a cancel is its own error, not an empty answer.
    let mut out = Vec::new();
    let result = respond(server.socket_path(), "Username:", &mut out);
    assert!(matches!(result, Err(RespondError::Cancelled)), "{result:?}");
    assert!(out.is_empty(), "nothing written for git to read");

    // On the wire: status byte 1, zero length.
    let mut raw = UnixStream::connect(server.socket_path()).expect("connects");
    raw.write_all(&3u32.to_be_bytes()).expect("writes");
    raw.write_all(b"Us:").expect("writes");
    let mut reply = Vec::new();
    raw.read_to_end(&mut reply).expect("reads");
    assert_eq!(reply, [1, 0, 0, 0, 0]);
    server.close().expect("closes cleanly");
}

/// Blocks in `ask` until the test lets it go; says when it has been asked.
struct Blocking {
    entered: Mutex<Sender<()>>,
    release: Mutex<Receiver<()>>,
}

impl AskPass for Blocking {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        self.entered
            .lock()
            .expect("test lock")
            .send(())
            .expect("the test is waiting");
        // Released: a late answer. The test gone (its sender dropped): the
        // user cancelled, as far as this handler can tell.
        self.release
            .lock()
            .expect("test lock")
            .recv()
            .ok()
            .map(|()| Secret::new(b"late".to_vec()))
    }
}

#[test]
fn drop_unblocks_waiting_client() {
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let server = server(Arc::new(Blocking {
        entered: Mutex::new(entered_tx),
        release: Mutex::new(release_rx),
    }));
    let socket = server.socket_path().to_owned();
    let client = thread::spawn(move || {
        let mut out = Vec::new();
        let result = respond(&socket, "Password:", &mut out);
        (result, out)
    });
    entered
        .recv_timeout(Duration::from_secs(5))
        .expect("the handler was asked");

    let started = Instant::now();
    drop(server);
    let dropped_in = started.elapsed();

    let (result, out) = client.join().expect("client thread");
    assert!(
        matches!(result, Err(RespondError::Io(_))),
        "the waiting helper gets an error, not a hang: {result:?}"
    );
    assert!(out.is_empty(), "no answer reached git");
    assert!(
        dropped_in < Duration::from_secs(1),
        "drop joined the listener in {dropped_in:?}"
    );
    // Let the detached handler thread finish; its late answer goes nowhere.
    release.send(()).expect("the handler is still waiting");
}

#[test]
fn socket_dir_is_private_and_removed() {
    let server = server(Fixed::new(None));
    let dir = server
        .socket_path()
        .parent()
        .expect("the socket is in a directory")
        .to_owned();
    let mode = std::fs::metadata(&dir)
        .expect("the directory exists")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700, "mode {mode:o}");
    assert!(
        dir.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("graft-askpass-")),
        "{dir:?}"
    );

    server.close().expect("closes cleanly");
    assert!(!dir.exists(), "{dir:?} removed");
}

#[test]
fn dropping_without_close_removes_the_dir_too() {
    let server = server(Fixed::new(None));
    let dir = server
        .socket_path()
        .parent()
        .expect("a directory")
        .to_owned();
    drop(server);
    assert!(!dir.exists(), "{dir:?} removed");
}

#[test]
fn an_oversized_prompt_is_refused_and_the_server_carries_on() {
    let server = server(Fixed::new(Some(b"ok")));

    let mut raw = UnixStream::connect(server.socket_path()).expect("connects");
    raw.write_all(&(PROMPT_MAX + 1).to_be_bytes())
        .expect("writes");
    let mut reply = Vec::new();
    raw.read_to_end(&mut reply).expect("reads to the close");
    assert!(reply.is_empty(), "closed without an answer");

    let long = "x".repeat(PROMPT_MAX as usize + 1);
    let refused = respond(server.socket_path(), &long, &mut Vec::new());
    assert!(
        matches!(refused, Err(RespondError::Protocol(_))),
        "{refused:?}"
    );

    let mut out = Vec::new();
    respond(server.socket_path(), "again:", &mut out).expect("still answering");
    assert_eq!(out, b"ok\n");
    server.close().expect("closes cleanly");
}

#[test]
fn secret_debug_is_redacted() {
    let secret = Secret::new(b"hunter2".to_vec());
    assert_eq!(format!("{secret:?}"), "Secret(…)");
    assert_eq!(format!("{:?}", Some(&secret)), "Some(Secret(…))");
    assert!(!format!("{secret:#?}").contains("hunter2"));
    assert_eq!(secret.expose(), b"hunter2");
}

/// `Secret`'s `Drop` is `wipe`. Reading the freed buffer after the drop
/// would need `unsafe`, which this crate forbids, so the wipe is tested on a
/// live buffer instead: every byte of the allocation, spare capacity too.
#[test]
fn wipe_zeroes_the_whole_allocation() {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(b"hunter2");
    let before = bytes.as_ptr();
    wipe(&mut bytes);
    assert_eq!(bytes.as_ptr(), before, "no reallocation: the same memory");
    assert_eq!(bytes.len(), bytes.capacity());
    assert!(bytes.iter().all(|&b| b == 0));
}
