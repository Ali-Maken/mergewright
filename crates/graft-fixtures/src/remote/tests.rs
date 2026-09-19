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
