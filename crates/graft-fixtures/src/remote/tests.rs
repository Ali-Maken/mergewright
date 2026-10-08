//! Every remote, exercised through real `git` (PROMPT.md §7.1 fast tier).

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};

use super::{HttpRemote, HungRemote, LocalRemote, hermetic_command, run_git};
use crate::git::GitError;
use crate::{Scratch, Shape};

const USER: &str = "alice";
const PASSWORD: &str = "s3cret";

fn local() -> LocalRemote {
    LocalRemote::new(Shape::Linear).expect("the Linear shape clones into a local remote")
}

/// An askpass script that answers every prompt with `answer`.
fn askpass(dir: &Path, answer: &str) -> String {
    let path = dir.join(format!("askpass-{answer}.sh"));
    fs::write(&path, format!("#!/bin/sh\necho '{answer}'\n")).expect("write askpass");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod askpass");
    path.to_string_lossy().into_owned()
}

/// `git <args>` in `dir` with extra environment; stdout trimmed.
fn git_env(remote: &LocalRemote, dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = run_git(remote.home(), dir, args, env)
        .unwrap_or_else(|error| panic!("git {args:?} failed: {error}"));
    String::from_utf8_lossy(&out).trim().to_owned()
}

/// Bytes that do not compress, so a pack carrying them stays their size.
fn incompressible(len: usize) -> Vec<u8> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state.to_le_bytes()[0]
        })
        .collect()
}

fn port_is_closed(port: u16) -> bool {
    TcpStream::connect(("127.0.0.1", port)).is_err()
}

#[test]
fn local_remote_clone_has_origin_and_upstream() {
    let remote = local();
    let branch = remote.default_branch();
    assert_eq!(branch, "main");
    let work = remote.work();
    assert_eq!(
        remote.git(work, &["remote", "get-url", "origin"]).unwrap(),
        remote.bare().to_string_lossy()
    );
    assert_eq!(
        remote
            .git(
                work,
                &[
                    "rev-parse",
                    "--abbrev-ref",
                    &format!("{branch}@{{upstream}}")
                ]
            )
            .unwrap(),
        format!("origin/{branch}")
    );
    assert!(remote.url().starts_with("file:///"), "{}", remote.url());
}

#[test]
fn commit_on_remote_leaves_work_behind() {
    let remote = local();
    let branch = remote.default_branch().to_owned();
    let before = remote.git(remote.work(), &["rev-parse", "HEAD"]).unwrap();
    let tip = remote.commit_on_remote("teammate's change").unwrap();

    assert_eq!(
        remote.git(remote.bare(), &["rev-parse", &branch]).unwrap(),
        tip
    );
    assert_eq!(
        remote.git(remote.work(), &["rev-parse", "HEAD"]).unwrap(),
        before
    );
    remote.git(remote.work(), &["fetch", "-q"]).unwrap();
    let range = format!("{branch}..origin/{branch}");
    assert_eq!(
        remote
            .git(remote.work(), &["rev-list", "--count", &range])
            .unwrap(),
        "1"
    );

    // A second call builds on the first, even after `work` pushed.
    remote
        .git(remote.work(), &["pull", "-q", "--ff-only"])
        .unwrap();
    remote.commit_locally("mine").unwrap();
    remote.git(remote.work(), &["push", "-q"]).unwrap();
    let second = remote.commit_on_remote("theirs again").unwrap();
    assert_eq!(
        remote.git(remote.bare(), &["rev-parse", &branch]).unwrap(),
        second
    );
}

#[test]
fn commit_locally_leaves_work_ahead() {
    let remote = local();
    let branch = remote.default_branch().to_owned();
    let remote_tip = remote.git(remote.bare(), &["rev-parse", &branch]).unwrap();
    let tip = remote.commit_locally("my change").unwrap();

    assert_eq!(
        remote.git(remote.work(), &["rev-parse", "HEAD"]).unwrap(),
        tip
    );
    assert_eq!(
        remote.git(remote.bare(), &["rev-parse", &branch]).unwrap(),
        remote_tip
    );
    let range = format!("origin/{branch}..{branch}");
    assert_eq!(
        remote
            .git(remote.work(), &["rev-list", "--count", &range])
            .unwrap(),
        "1"
    );
}

#[test]
fn hermetic_env_blocks_user_config() {
    let remote = local();
    let real_home = std::env::var("HOME").expect("the test process has a HOME");
    let listing = remote
        .git(remote.work(), &["config", "--show-origin", "--list"])
        .unwrap();
    for line in listing.lines() {
        assert!(
            !line.contains(&real_home),
            "config from the user's home leaked in: {line}"
        );
        assert!(line.starts_with("file:"), "unexpected origin: {line}");
    }
    assert!(listing.contains("remote.origin.url"), "{listing}");
}

#[test]
fn http_wrong_password_is_401_and_git_says_authentication_failed() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();

    // No credentials at all, no way to ask for them.
    let missing = run_git(
        remote.home(),
        remote.work(),
        &[
            "-c",
            "credential.helper=",
            "ls-remote",
            &http.url_anonymous(),
        ],
        &[("GIT_TERMINAL_PROMPT", "0")],
    );
    let Err(GitError::Failed { status, stderr, .. }) = missing else {
        panic!("ls-remote without credentials should fail: {missing:?}");
    };
    assert_eq!(status.code(), Some(128), "{stderr}");
    assert!(
        stderr.contains("could not read Username") || stderr.contains("Authentication failed"),
        "{stderr}"
    );
    assert_eq!(http.requests().0, 0);
    let rejected_so_far = http.requests().1;
    assert!(rejected_so_far >= 1);

    // The wrong password, given.
    let scripts = Scratch::empty("askpass").unwrap();
    let wrong = askpass(scripts.path(), "not-the-password");
    let refused = run_git(
        remote.home(),
        remote.work(),
        &["-c", "credential.helper=", "ls-remote", &http.url()],
        &[("GIT_TERMINAL_PROMPT", "0"), ("GIT_ASKPASS", &wrong)],
    );
    let Err(GitError::Failed { stderr, .. }) = refused else {
        panic!("ls-remote with the wrong password should fail: {refused:?}");
    };
    assert!(stderr.contains("Authentication failed"), "{stderr}");
    assert_eq!(http.requests().0, 0);
    assert!(http.requests().1 > rejected_so_far);
}

#[test]
fn http_right_password_clones_and_pushes() {
    let remote = local();
    let branch = remote.default_branch().to_owned();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let scratch = Scratch::empty("http-clone").unwrap();
    let right = askpass(scratch.path(), PASSWORD);
    let env = [
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GIT_ASKPASS", right.as_str()),
    ];
    let clone = scratch.path().join("clone");

    git_env(
        &remote,
        scratch.path(),
        &["clone", "-q", &http.url(), &clone.to_string_lossy()],
        &env,
    );
    assert_eq!(
        git_env(&remote, &clone, &["rev-parse", "HEAD"], &[]),
        remote.git(remote.bare(), &["rev-parse", &branch]).unwrap()
    );

    // Fetch: a teammate's commit arrives over HTTP.
    let theirs = remote.commit_on_remote("over http").unwrap();
    git_env(&remote, &clone, &["fetch", "-q"], &env);
    assert_eq!(
        git_env(
            &remote,
            &clone,
            &["rev-parse", &format!("origin/{branch}")],
            &[]
        ),
        theirs
    );
    git_env(
        &remote,
        &clone,
        &["merge", "-q", "--ff-only", "FETCH_HEAD"],
        &[],
    );

    // Push 2 MiB that will not compress: past `http.postBuffer` (1 MiB), so
    // git sends the pack chunked.
    fs::write(clone.join("big.bin"), incompressible(2 * 1024 * 1024)).unwrap();
    git_env(&remote, &clone, &["add", "big.bin"], &[]);
    git_env(&remote, &clone, &["commit", "-q", "-m", "big"], &[]);
    let pushed = git_env(&remote, &clone, &["rev-parse", "HEAD"], &[]);
    git_env(&remote, &clone, &["push", "-q", "origin", "HEAD"], &env);

    assert_eq!(
        remote.git(remote.bare(), &["rev-parse", &branch]).unwrap(),
        pushed
    );
    assert!(http.requests().0 >= 2, "{:?}", http.requests());
    assert!(
        http.chunked_requests() >= 1,
        "the 2 MiB push should have arrived chunked"
    );
}

/// `printf '0014command=ls-refs\n0000' | gzip -n -c`: a protocol-v2
/// `ls-refs` request, gzipped the way git gzips a large upload-pack request.
const LS_REFS_GZIPPED: [u8; 42] = [
    0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x33, 0x30, 0x30, 0x34, 0x49, 0xce,
    0xcf, 0xcd, 0x4d, 0xcc, 0x4b, 0xb1, 0xcd, 0x29, 0xd6, 0x2d, 0x4a, 0x4d, 0x2b, 0xe6, 0x32, 0x00,
    0x02, 0x00, 0xf7, 0x1d, 0x61, 0x5d, 0x18, 0x00, 0x00, 0x00,
];

/// Git gzips an upload-pack request only past a size the small corpus
/// never reaches, so this sends one by hand: the backend must be told the
/// body is gzipped (`HTTP_CONTENT_ENCODING`) or it reads garbage.
#[test]
fn http_gzipped_request_is_inflated_by_the_backend() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", http.port())).unwrap();
    let head = format!(
        "POST /repo.git/git-upload-pack HTTP/1.1\r\n\
         Host: 127.0.0.1\r\n\
         Authorization: Basic {}\r\n\
         Content-Type: application/x-git-upload-pack-request\r\n\
         Content-Encoding: gzip\r\n\
         Git-Protocol: version=2\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        super::http::base64(format!("{USER}:{PASSWORD}").as_bytes()),
        LS_REFS_GZIPPED.len()
    );
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(&LS_REFS_GZIPPED).unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    let answer = String::from_utf8_lossy(&answer);

    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.contains("refs/heads/main"), "{answer}");
    assert_eq!(http.gzipped_requests(), 1);
}

#[test]
fn http_remote_frees_its_port_on_drop() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let port = http.port();
    assert!(!port_is_closed(port));
    drop(http);
    assert!(port_is_closed(port), "port {port} still accepts after drop");
}

/// Whether a process with this pid still exists (a zombie counts).
fn process_exists(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .expect("kill(1) runs")
        .success()
}

/// Waits up to `bound` for `done`; whether it came true.
fn eventually(bound: Duration, mut done: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while !done() {
        if started.elapsed() > bound {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

/// The wedged listener the old teardown could only warn about: a push whose
/// `pre-receive` hook never returns holds the listener inside
/// `http-backend` → `receive-pack` → hook. Drop kills the backend's whole
/// process group, joins the listener, and frees the port — or panics, which
/// fails this test.
#[test]
fn http_remote_drop_ends_a_wedged_backend() {
    let remote = local();
    let scratch = Scratch::empty("http-wedged").unwrap();
    let pid_file = scratch.path().join("hook.pid");
    let hook = remote.bare().join("hooks").join("pre-receive");
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    // Written then renamed, so the test never reads a half-written pid. The
    // `sleep` is the wedge: a child of the hook, not `exec`ed, so killing the
    // backend alone would leave it holding the pipes.
    fs::write(
        &hook,
        format!(
            "#!/bin/sh\necho $$ > '{0}.tmp'\nmv '{0}.tmp' '{0}'\nsleep 3600\n",
            pid_file.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    remote.commit_locally("wedges the hook").unwrap();

    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let port = http.port();
    let right = askpass(scratch.path(), PASSWORD);
    let mut push = hermetic_command(
        remote.home(),
        remote.work(),
        &["push", "-q", &http.url(), "HEAD"],
    )
    .env("GIT_TERMINAL_PROMPT", "0")
    .env("GIT_ASKPASS", &right)
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .unwrap();

    assert!(
        eventually(Duration::from_secs(10), || pid_file.exists()),
        "the pre-receive hook never ran"
    );
    let hook_pid = fs::read_to_string(&pid_file).unwrap().trim().to_owned();
    assert!(process_exists(&hook_pid), "the hook is running");

    let started = Instant::now();
    drop(http);
    let dropped_in = started.elapsed();

    assert!(
        dropped_in < Duration::from_millis(500),
        "drop took {dropped_in:?}"
    );
    assert!(port_is_closed(port), "port {port} still accepts after drop");
    // Killed with its group; gone once its new parent reaps it.
    assert!(
        eventually(Duration::from_secs(5), || !process_exists(&hook_pid)),
        "the hook {hook_pid} outlived the fixture"
    );
    // The push lost its server; it fails rather than hangs. Killed only if
    // it somehow still runs, so the test cannot leave it behind.
    if !eventually(Duration::from_secs(5), || {
        push.try_wait().unwrap().is_some()
    }) {
        push.kill().unwrap();
    }
    assert!(
        !push.wait().unwrap().success(),
        "the push cannot have landed"
    );
}

/// The cause of 13i, pinned: `drop` must not return before the thread that
/// owns the listening socket has dropped it. The seam holds the listener
/// 300 ms past its loop, so a `drop` that does not join that thread returns
/// early and the port still accepts — every time, not only under load.
#[test]
fn http_remote_drop_waits_for_the_listener_to_close() {
    let remote = local();
    let delay = Duration::from_millis(300);
    let http = HttpRemote::serve_with_exit_delay(remote.bare(), USER, PASSWORD, delay).unwrap();
    let port = http.port();
    assert!(!port_is_closed(port));

    let started = Instant::now();
    drop(http);
    let dropped_in = started.elapsed();

    assert!(
        dropped_in >= delay,
        "drop returned in {dropped_in:?}, before the listener thread let go of the socket"
    );
    assert!(port_is_closed(port), "port {port} still accepts after drop");
}

/// `printf '0014command=ls-refs\n0000'`: a protocol-v2 `ls-refs` request.
const LS_REFS: &[u8] = b"0014command=ls-refs\n0000";

/// The head of an authorised `git-upload-pack` POST, up to and including the
/// blank line; `extra` is more header lines, each ending `\r\n`.
fn upload_pack_head(extra: &str) -> String {
    format!(
        "POST /repo.git/git-upload-pack HTTP/1.1\r\n\
         Host: 127.0.0.1\r\n\
         Authorization: Basic {}\r\n\
         Content-Type: application/x-git-upload-pack-request\r\n\
         Git-Protocol: version=2\r\n\
         {extra}\r\n",
        super::http::base64(format!("{USER}:{PASSWORD}").as_bytes()),
    )
}

/// A chunked body is reassembled before `http-backend` sees it: the backend
/// answers `ls-refs` with the branch only if it got the exact 24 bytes.
#[test]
fn http_remote_reads_a_chunked_body() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", http.port())).unwrap();
    stream
        .write_all(upload_pack_head("Transfer-Encoding: chunked\r\n").as_bytes())
        .unwrap();
    // Split mid-pkt-line, with a chunk extension, then a trailer.
    let (first, second) = LS_REFS.split_at(10);
    write!(stream, "{:x};ext=1\r\n", first.len()).unwrap();
    stream.write_all(first).unwrap();
    write!(stream, "\r\n{:X}\r\n", second.len()).unwrap();
    stream.write_all(second).unwrap();
    stream.write_all(b"\r\n0\r\nX-Trailer: t\r\n\r\n").unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    let answer = String::from_utf8_lossy(&answer);

    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.contains("refs/heads/main"), "{answer}");
    assert_eq!(http.chunked_requests(), 1);
    assert_eq!(http.requests(), (1, 0));
}

/// `Expect: 100-continue`: the server says `100 Continue` before the client
/// sends a byte of the body, then answers the request.
#[test]
fn http_remote_answers_expect_100_continue() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", http.port())).unwrap();
    // A server that never says 100 fails the read rather than hang the test.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let extra = format!(
        "Expect: 100-continue\r\nContent-Length: {}\r\n",
        LS_REFS.len()
    );
    stream
        .write_all(upload_pack_head(&extra).as_bytes())
        .unwrap();

    let mut interim = Vec::new();
    while !interim.ends_with(b"\r\n\r\n") {
        let mut byte = [0u8; 1];
        let read = stream.read(&mut byte).expect("the 100 Continue arrives");
        assert_eq!(read, 1, "closed before 100 Continue: {interim:?}");
        interim.push(byte[0]);
    }
    assert_eq!(interim, b"HTTP/1.1 100 Continue\r\n\r\n");

    stream.write_all(LS_REFS).unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    let answer = String::from_utf8_lossy(&answer);
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.contains("refs/heads/main"), "{answer}");
}

/// A client that connects and says nothing holds a handler blocked in its
/// first read. Drop still ends it promptly and frees the port. The bound is
/// under the old 1 s `READ_TIMEOUT` (m8/21b-3), so the read's timeout cannot
/// pass it alone: this pins `Drop`'s shutdown of the stream.
#[test]
fn http_remote_drop_ends_an_idle_client() {
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let port = http.port();
    let idle = TcpStream::connect(("127.0.0.1", port)).unwrap();
    assert!(
        eventually(Duration::from_secs(5), || http.open_connections() == 1),
        "the idle connection reached a handler"
    );

    let started = Instant::now();
    drop(http);
    let dropped_in = started.elapsed();

    assert!(
        dropped_in < Duration::from_millis(500),
        "drop took {dropped_in:?}"
    );
    assert!(port_is_closed(port), "port {port} still accepts after drop");
    let mut buf = [0u8; 1];
    let read = (&idle).read(&mut buf);
    assert!(
        matches!(read, Ok(0) | Err(_)),
        "the idle stream was closed: {read:?}"
    );
}

/// Drop wakes the listener while it is still open, joins it, and closes the
/// held streams: it returns promptly, and the listener is gone with the port.
/// (A failed wake panics in drop; loopback gives no way to make it fail here.)
#[test]
fn hung_remote_drop_joins_its_listener_promptly() {
    let hung = HungRemote::listen().unwrap();
    let port = hung.port();
    let held = TcpStream::connect(("127.0.0.1", port)).unwrap();
    assert!(
        eventually(Duration::from_secs(5), || hung.connections() == 1),
        "the connection was held"
    );

    let started = Instant::now();
    drop(hung);
    let dropped_in = started.elapsed();

    assert!(
        dropped_in < Duration::from_secs(1),
        "drop took {dropped_in:?}"
    );
    assert!(port_is_closed(port), "port {port} still accepts after drop");
    let mut buf = [0u8; 1];
    let read = (&held).read(&mut buf);
    assert!(
        matches!(read, Ok(0) | Err(_)),
        "the held stream was closed: {read:?}"
    );
}

#[test]
fn hung_remote_blocks_then_dies_on_sigterm() {
    let hung = HungRemote::listen().unwrap();
    let scratch = Scratch::empty("hung").unwrap();
    let home = scratch.path().join("home");
    let repo = scratch.path().join("repo");
    fs::create_dir(&home).unwrap();
    fs::create_dir(&repo).unwrap();
    run_git(&home, &repo, &["init", "-q"], &[]).unwrap();

    let mut fetch = hermetic_command(&home, &repo, &["fetch", &hung.url()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let started = Instant::now();
    while hung.connections() == 0 && started.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(300));
    assert!(
        fetch.try_wait().unwrap().is_none(),
        "git fetch returned against a server that never answers"
    );
    assert_eq!(hung.connections(), 1);

    // `Child::kill` is SIGKILL; graft-git's `run` owns the SIGTERM path and
    // tests it there.
    fetch.kill().unwrap();
    let status = fetch.wait().unwrap();
    assert!(!status.success());

    let port = hung
        .url()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches("/repo.git")
        .parse::<u16>()
        .unwrap();
    drop(hung);
    assert!(port_is_closed(port), "port {port} still accepts after drop");
}
