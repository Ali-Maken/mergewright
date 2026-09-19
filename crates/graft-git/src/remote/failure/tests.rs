//! Every constant below was captured from a real git run on **git 2.54.0
//! (Apple Git-157)**, `LC_ALL=C GIT_TERMINAL_PROMPT=0`, 2026-09-20, unless its
//! comment says otherwise. Local paths were shortened to `/tmp/t`; nothing
//! else was edited.

use super::*;

/// `git push origin main` after another clone pushed, before a fetch.
const PUSH_FETCH_FIRST: &str = "To /tmp/t/bare.git
 ! [rejected]        main -> main (fetch first)
error: failed to push some refs to '/tmp/t/bare.git'
hint: Updates were rejected because the remote contains work that you do not
hint: have locally. This is usually caused by another repository pushing to
hint: the same ref. If you want to integrate the remote changes, use
hint: 'git pull' before pushing again.
hint: See the 'Note about fast-forwards' in 'git push --help' for details.
";

/// The same push after a fetch.
const PUSH_NON_FAST_FORWARD: &str = "To /tmp/t/bare.git
 ! [rejected]        main -> main (non-fast-forward)
error: failed to push some refs to '/tmp/t/bare.git'
hint: Updates were rejected because the tip of your current branch is behind
hint: its remote counterpart. If you want to integrate the remote changes,
hint: use 'git pull' before pushing again.
hint: See the 'Note about fast-forwards' in 'git push --help' for details.
";

/// `git push --force-with-lease=main:<local HEAD> origin main`, remote moved.
const PUSH_STALE_INFO: &str = "To /tmp/t/bare.git
 ! [rejected]        main -> main (stale info)
error: failed to push some refs to '/tmp/t/bare.git'
";

/// A push into a bare repo whose `pre-receive` prints and exits 1. Note the
/// padding git puts after a `remote:` line (escaped: rustfmt strips trailing
/// spaces from a multi-line literal).
const PUSH_HOOK_DECLINED: &str = concat!(
    "remote: no pushes today\x20\x20\x20\x20\x20\x20\x20\x20\n",
    "To /tmp/t/bare.git\n",
    " ! [remote rejected] main -> main (pre-receive hook declined)\n",
    "error: failed to push some refs to '/tmp/t/bare.git'\n",
);

/// `git ls-remote http://127.0.0.1:8765/x.git` against a server answering 401.
const HTTP_NO_USERNAME: &str =
    "fatal: could not read Username for 'http://127.0.0.1:8765': terminal prompts disabled\n";

/// The same with `user@` in the URL.
const HTTP_NO_PASSWORD: &str =
    "fatal: could not read Password for 'http://user@127.0.0.1:8765': terminal prompts disabled\n";

/// The same with a `GIT_ASKPASS` that answers a wrong password.
const HTTP_WRONG_PASSWORD: &str =
    "fatal: Authentication failed for 'http://127.0.0.1:8765/x.git/'\n";

/// `git fetch nope` / `git push nope main`, no such remote.
const NO_SUCH_REMOTE: &str = "fatal: 'nope' does not appear to be a git repository
fatal: Could not read from remote repository.

Please make sure you have the correct access rights
and the repository exists.
";

/// `git remote remove nope`.
const NO_SUCH_REMOTE_REMOVE: &str = "error: No such remote: 'nope'\n";

const UNRESOLVED_HOST: &str = "fatal: unable to access 'http://no-such-host.invalid/x.git/': Could not resolve host: no-such-host.invalid\n";

const REFUSED_HTTP: &str = "fatal: unable to access 'http://127.0.0.1:1/x.git/': Failed to connect to 127.0.0.1 port 1 after 0 ms: Couldn't connect to server\n";

const REFUSED_SSH: &str = "ssh: connect to host 127.0.0.1 port 1: Connection refused
fatal: Could not read from remote repository.

Please make sure you have the correct access rights
and the repository exists.
";

/// `git merge --ff-only origin/main` with both sides ahead.
const FF_ONLY_DIVERGED: &str =
    "hint: Diverging branches can't be fast-forwarded, you need to either:
hint:
hint: \tgit merge --no-ff
hint:
hint: or:
hint:
hint: \tgit rebase
hint:
hint: Disable this message with \"git config set advice.diverging false\"
fatal: Not possible to fast-forward, aborting.
";

/// `git merge --ff-only origin/main` over a modified tracked file.
const FF_ONLY_DIRTY_TRACKED: &str =
    "error: Your local changes to the following files would be overwritten by merge:
\tf
Please commit your changes or stash them before you merge.
Aborting
Updating 865fc1a..6402177
";

/// The same over an untracked file the merge would create.
const FF_ONLY_DIRTY_UNTRACKED: &str =
    "error: The following untracked working tree files would be overwritten by merge:
\tg
Please move or remove them before you merge.
Aborting
Updating 29c4603..53369f9
";

fn classify(stderr: &str) -> RemoteFailure {
    classify_failure(stderr.as_bytes())
}

#[test]
fn auth_failed_from_real_stderr() {
    assert_eq!(
        classify(HTTP_WRONG_PASSWORD),
        RemoteFailure::AuthFailed {
            target: "http://127.0.0.1:8765/x.git/".to_owned()
        }
    );
    assert_eq!(
        classify(HTTP_NO_USERNAME),
        RemoteFailure::AuthFailed {
            target: "http://127.0.0.1:8765".to_owned()
        }
    );
    assert_eq!(
        classify(HTTP_NO_PASSWORD),
        RemoteFailure::AuthFailed {
            target: "http://user@127.0.0.1:8765".to_owned()
        }
    );
}

/// **Not captured**: the fast tier has no SSH server or hosted forge. These
/// are the lines as GitHub and OpenSSH document them; treat them as
/// unverified against a live server.
#[test]
fn auth_failed_from_documented_ssh_and_forge_stderr() {
    assert_eq!(
        classify(
            "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.\n"
        ),
        RemoteFailure::AuthFailed {
            target: "git@github.com".to_owned()
        }
    );
    assert_eq!(
        classify("Host key verification failed.\nfatal: Could not read from remote repository.\n"),
        RemoteFailure::AuthFailed {
            target: String::new()
        }
    );
    assert_eq!(
        classify(
            "remote: Invalid username or password.\nfatal: Authentication failed for 'https://github.com/o/r.git/'\n"
        ),
        RemoteFailure::AuthFailed {
            target: "https://github.com/o/r.git/".to_owned()
        }
    );
}

#[test]
fn non_fast_forward_from_real_stderr() {
    assert_eq!(classify(PUSH_FETCH_FIRST), RemoteFailure::NonFastForward);
    assert_eq!(
        classify(PUSH_NON_FAST_FORWARD),
        RemoteFailure::NonFastForward
    );
}

#[test]
fn stale_info_from_real_stderr() {
    assert_eq!(classify(PUSH_STALE_INFO), RemoteFailure::StaleInfo);
}

#[test]
fn remote_rejected_from_real_stderr() {
    assert_eq!(
        classify(PUSH_HOOK_DECLINED),
        RemoteFailure::RemoteRejected {
            reason: "pre-receive hook declined".to_owned()
        }
    );
}

#[test]
fn no_such_remote_from_real_stderr() {
    let nope = RemoteFailure::NoSuchRemote {
        name: "nope".to_owned(),
    };
    assert_eq!(classify(NO_SUCH_REMOTE), nope);
    assert_eq!(classify(NO_SUCH_REMOTE_REMOVE), nope);
}

#[test]
fn unreachable_from_real_stderr() {
    assert_eq!(
        classify(UNRESOLVED_HOST),
        RemoteFailure::Unreachable {
            target: "no-such-host.invalid".to_owned()
        }
    );
    assert_eq!(
        classify(REFUSED_HTTP),
        RemoteFailure::Unreachable {
            target: "127.0.0.1".to_owned()
        }
    );
    assert_eq!(
        classify(REFUSED_SSH),
        RemoteFailure::Unreachable {
            target: "127.0.0.1".to_owned()
        }
    );
}

#[test]
fn not_fast_forward_from_real_stderr() {
    assert_eq!(classify(FF_ONLY_DIVERGED), RemoteFailure::NotFastForward);
}

#[test]
fn would_overwrite_from_real_stderr() {
    assert_eq!(
        classify(FF_ONLY_DIRTY_TRACKED),
        RemoteFailure::WouldOverwrite {
            paths: vec!["f".to_owned()]
        }
    );
    assert_eq!(
        classify(FF_ONLY_DIRTY_UNTRACKED),
        RemoteFailure::WouldOverwrite {
            paths: vec!["g".to_owned()]
        }
    );
}

/// `Cancelled` comes from the runner's `Stopped`, never from text: even the
/// stderr of a fetch killed mid-transfer (progress, then nothing) is not read
/// as a cancel. The progress lines are the real shape from git 2.54.0.
#[test]
fn cancelled_from_real_stderr() {
    let killed = "remote: Enumerating objects: 90, done.        \nremote: Counting objects:  14% (13/90)        \r";
    assert_eq!(
        classify(killed),
        RemoteFailure::Other {
            first_line: String::new()
        }
    );
    assert_ne!(classify(HTTP_NO_PASSWORD), RemoteFailure::Cancelled);
}

/// `git ls-remote http://127.0.0.1:8766/x.git` against a server answering 404.
const HTTP_NOT_FOUND: &str = "fatal: repository 'http://127.0.0.1:8766/x.git/' not found\n";

#[test]
fn no_such_remote_from_a_real_http_404() {
    assert_eq!(
        classify(HTTP_NOT_FOUND),
        RemoteFailure::NoSuchRemote {
            name: "http://127.0.0.1:8766/x.git/".to_owned()
        }
    );
}

/// Anything unmatched surfaces verbatim: the first `fatal:`/`error:` line.
#[test]
fn other_from_real_stderr() {
    // `git checkout -- g` naming a file git does not track.
    let unmatched = "error: pathspec 'g' did not match any file(s) known to git\n";
    assert_eq!(
        classify(unmatched),
        RemoteFailure::Other {
            first_line: "error: pathspec 'g' did not match any file(s) known to git".to_owned()
        }
    );
    // Synthetic: a prefix line ahead of the error, as a push's `To <url>` is.
    assert_eq!(
        classify("To /tmp/t/bare.git\nfatal: the remote end hung up unexpectedly\n"),
        RemoteFailure::Other {
            first_line: "fatal: the remote end hung up unexpectedly".to_owned()
        }
    );
    // Synthetic: no `fatal:`/`error:` at all, so the first line stands.
    assert_eq!(
        classify("something new\n"),
        RemoteFailure::Other {
            first_line: "something new".to_owned()
        }
    );
}
