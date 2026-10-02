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

/// Runs `close` on its own thread, so a close that hangs fails this test
/// after `bound` instead of hanging the test binary.
fn close_within(server: AskPassServer, bound: Duration) -> io::Result<()> {
    let (done, closed) = mpsc::channel();
    thread::spawn(move || done.send(server.close()));
    closed
        .recv_timeout(bound)
        .expect("close returned within the bound, not stuck on its listener")
}

/// The wake-up connect fails with the socket file still there (its directory
/// made unsearchable). `close` says so — it does not claim the listener
/// ended — and leaves the socket in place, so the listener can still be
/// woken. A helper connecting after that, while the server is stopping, is
/// closed unanswered (review 01's race: a connection accepted during
/// shutdown), and its connection is what ends the listener.
#[test]
fn a_failed_wake_is_reported_and_a_late_helper_is_never_answered() {
    let handler = Fixed::new(Some(b"must-not-be-sent"));
    let server = server(handler.clone());
    let socket = server.socket_path().to_owned();
    let dir = socket.parent().expect("a directory").to_owned();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000))
        .expect("the directory is made unsearchable");

    let closed = close_within(server, Duration::from_secs(5));

    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .expect("the directory is searchable again");
    let error = closed.expect_err("a failed wake is an error");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied, "{error}");
    assert!(
        error.to_string().contains("still running"),
        "the error says the listener was not stopped: {error}"
    );
    assert!(
        socket.exists(),
        "the socket is left so it can still be woken"
    );
    assert_eq!(
        Arc::strong_count(&handler),
        2,
        "the listener is still running, as the error said"
    );

    let mut out = Vec::new();
    let late = respond(&socket, "Password:", &mut out);
    assert!(
        matches!(late, Err(RespondError::Io(_))),
        "a helper reaching a stopping server gets EOF: {late:?}"
    );
    assert!(out.is_empty(), "no answer reached git");
    assert!(
        handler.prompts.lock().expect("test lock").is_empty(),
        "the user was never asked"
    );
    let started = Instant::now();
    while Arc::strong_count(&handler) > 1 {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the listener did not end on the late connection"
        );
        thread::sleep(Duration::from_millis(10));
    }
    std::fs::remove_file(&socket).expect("the test removes the socket");
    std::fs::remove_dir(&dir).expect("the test removes the directory");
}

/// Answers every prompt with far more than a socket buffer holds, and says
/// when it has been asked.
struct Huge {
    asked: Mutex<Sender<()>>,
}

impl AskPass for Huge {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        self.asked
            .lock()
            .expect("test lock")
            .send(())
            .expect("the test is waiting");
        Some(Secret::new(vec![b'x'; 8 * 1024 * 1024]))
    }
}

/// A helper that asks and then never reads: the reply write fills the socket
/// buffer and blocks. Once the handler has answered, `serve` writes whatever
/// the stop flag says, so the listener is in (or about to be in) that write
/// when `close` runs. The write timeout ends it; `close` returns and the
/// listener ends.
#[test]
fn a_helper_that_stops_reading_cannot_hang_close() {
    let (asked_tx, asked) = mpsc::channel();
    let handler: Arc<dyn AskPass> = Arc::new(Huge {
        asked: Mutex::new(asked_tx),
    });
    let server = server(Arc::clone(&handler));
    let mut raw = UnixStream::connect(server.socket_path()).expect("connects");
    raw.write_all(&3u32.to_be_bytes()).expect("writes");
    raw.write_all(b"Pw:").expect("writes");
    asked
        .recv_timeout(Duration::from_secs(5))
        .expect("the handler was asked");

    let closed = close_within(server, Duration::from_secs(5));

    // `close` returning at all is the proof: it joins the listener. (The
    // handler's count is no witness here — the detached ask thread holds a
    // clone of its own until it has returned.)
    closed.expect("closes cleanly");
    // Held open until here, so the write could never finish on its own.
    drop(raw);
}

/// Panics on the first prompt, answers the rest.
struct PanicsOnce {
    asked: AtomicU32,
}

impl AskPass for PanicsOnce {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        assert!(
            self.asked.fetch_add(1, Ordering::SeqCst) > 0,
            "the first prompt panics (on purpose)"
        );
        Some(Secret::new(b"second".to_vec()))
    }
}

/// The wait on the handler is one blocking `recv`, not a poll: a handler
/// that unwinds must still end it (through `Reply`'s drop), or the listener
/// would wait forever. The helper gets an error; the next prompt is answered.
#[test]
fn a_panicking_handler_fails_its_helper_and_the_server_carries_on() {
    let server = server(Arc::new(PanicsOnce {
        asked: AtomicU32::new(0),
    }));

    let mut out = Vec::new();
    let first = respond(server.socket_path(), "Password:", &mut out);
    assert!(
        matches!(first, Err(RespondError::Io(_))),
        "the helper whose handler panicked gets an error: {first:?}"
    );
    assert!(out.is_empty(), "no answer reached git");

    let mut out = Vec::new();
    respond(server.socket_path(), "Password:", &mut out).expect("the next prompt is answered");
    assert_eq!(out, b"second\n");
    server.close().expect("closes cleanly");
}

// --- Peer check (m8-polish-and-ship/21) -------------------------------------

#[test]
fn admit_takes_only_our_own_uid() {
    assert!(admit(Ok(501), 501).is_ok(), "same uid: served");
    assert_eq!(
        admit(Ok(0), 501).map_err(|refusal| refusal.to_string()),
        Err("askpass: refused a connection from uid 0 (this server serves uid 501)".to_owned()),
        "another uid, root included, is refused"
    );
    assert_eq!(
        admit(Err(io::Error::other("no creds")), 501).map_err(|refusal| refusal.to_string()),
        Err("askpass: refused a connection whose uid could not be read: no creds".to_owned()),
        "a peer whose uid cannot be read is refused, never assumed to be us"
    );
}

#[test]
fn peer_uid_of_a_real_connection_is_ours() {
    let (ours, _theirs) = UnixStream::pair().expect("socket pair");
    assert_eq!(
        peer_uid(&ours).expect("peer uid readable"),
        nix::unistd::geteuid().as_raw()
    );
}

/// The server is told its owner is a uid this process is not, so the test
/// process is "another user" to it: the different-uid path without root.
#[test]
fn a_peer_with_another_uid_is_refused_unanswered_and_never_asked() {
    let handler = Fixed::new(Some(b"s3c"));
    let stranger = nix::unistd::geteuid().as_raw().wrapping_add(1);
    let server = AskPassServer::spawn_owned_by(
        PathBuf::from("/nonexistent/graft"),
        handler.clone(),
        stranger,
    )
    .expect("server starts");

    let mut out = Vec::new();
    let result = respond(server.socket_path(), "Password for 'http://u@h':", &mut out);

    assert!(
        matches!(result, Err(RespondError::Io(_))),
        "the refused helper sees the connection end, got {result:?}"
    );
    assert!(out.is_empty(), "no secret reached the refused peer");
    assert!(
        handler.prompts.lock().expect("test lock").is_empty(),
        "the handler was never asked for a refused peer"
    );
    server.close().expect("closes cleanly after a refusal");
}
