# git-remote-and-askpass: remote reads, failure and progress parsing, the streaming stderr drain, and the askpass socket

Status: draft
Build with: Opus — taking the planner's recommendation: a second drain mode in graft-git's runner plus a Unix-socket server whose shutdown order matters. Concurrency, not wiring.
Crates: graft-git
Needs: none (graft-fixtures' `LocalRemote` is nice-to-have for one test; if graft-fixtures depends on graft-git, use `graft_git::fixture::Fixture::bare_repo()` instead and say so)

## Read first

- `docs/design-notes/m4-remotes.md` — Interfaces (the `graft-git` block); Hard choices 2, 3, 5; Genuinely risky groups (G2)
- `.scratch/m4-remotes/scout.md` §Git invocation (lines 37–44), §Remote data that already exists (85–91), §Tests (93–101)
- `crates/graft-git/MAP.md` §lib.rs, §repo (`RepoHandle::upstream`, `reference`), §run (whole), §version (`system_git`'s deadline `Stop` pattern), §stash (`list` — the shape of a gix-backed read)
- `docs/design/01-view-mockups.md` §1.14 (lines 273–287) — what the askpass helper is for
- `PROMPT.md` Technology "Writing to the repo" (lines 51–54); §3 "Long operations" (254–258)
- CLAUDE.md "Rules for every agent"

## Scope

In — in this order (each piece a commit):

1. **`src/remote.rs` — reads.** `Remote`, `list(repo)`: gix config `remote.<name>.url` / `pushurl`; sorted by name with `origin` first; a remote with no URL is skipped with the reason in `RemoteError::Config`. `TrackedUpstream`, `upstream_of(repo, branch_full_name)`: `branch.<short>.remote` + `branch.<short>.merge` → `remote_branch` (the `merge` value, a `refs/heads/x` name), `tracking_ref` by applying the remote's fetch refspec (`+refs/heads/*:refs/remotes/<remote>/*` — use gix's refspec matching, do not string-replace; a remote with a non-standard refspec that does not map the branch → `Ok(None)`), `tracking_tip` via `RepoHandle::reference`. Compare with the existing `RepoHandle::upstream` (scout: it exists, returns the remote-tracking ref) — reuse it for `tracking_ref`, do not duplicate; if it is a `git` spawn, keep it and say so.
2. **`src/remote/failure.rs` — `RemoteFailure`, `classify_failure(stderr)`.** Pure, over the full stderr, checked in this order (first match wins): `Cancelled` — never matched from text (the caller sets it from `RunError::Stopped`); `AuthFailed { target }` — `Authentication failed for '<url>'`, `Invalid username or password`, `could not read Username for '<url>'`, `could not read Password for '<url>'`, `Permission denied (publickey`, `Host key verification failed` (target = the quoted URL or the `ssh:` host); `StaleInfo` — `[rejected]` … `(stale info)`; `NonFastForward` — `[rejected]` … `(non-fast-forward)` or `(fetch first)`; `RemoteRejected { reason }` — `[remote rejected]` … `(<reason>)`, `pre-receive hook declined`; `NoSuchRemote { name }` — `'<name>' does not appear to be a git repository`, `No such remote`, `fatal: '<name>' does not appear`; `Unreachable { target }` — `Could not resolve host: <h>`, `Connection refused`, `Connection timed out`, `Network is unreachable`, `ssh: connect to host <h>`, `unable to access '<url>'`; `NotFastForward` — `Not possible to fast-forward, aborting`; `WouldOverwrite { paths }` — `Your local changes to the following files would be overwritten` followed by tab-indented paths up to the blank/`Please` line; else `Other { first_line }` (first non-empty, non-`remote:`-progress line). Table-driven: one `(pattern, constructor)` list, a test per row using stderr captured from real git (the fixtures in piece 5 produce them; paste the exact bytes into the test as constants and name the git version they came from).
3. **`src/remote/progress.rs` — `RemoteProgress`, `parse_progress(segment)`, `brief()`.** Segments are what git writes between `\r`/`\n`: `remote: Enumerating objects: 12, done.`, `remote: Counting objects:  50% (6/12)`, `remote: Compressing objects: 100% (3/3), done.`, `Receiving objects:  45% (450/1000), 1.20 MiB | 2.30 MiB/s`, `Resolving deltas:  6% (3/50)`, `Writing objects: 100% (3/3), 300 bytes | 300.00 KiB/s, done.`, `Unpacking objects: 100% (3/3), 250 bytes | 250.00 KiB/s, done.`, `Total 12 (delta 3), reused 0 (delta 0)`. Strip a leading `remote: `. Phase lower-cased without the colon (`receiving objects`). `None` for anything else (ref update lines, hints, errors). `brief()` = `<phase> <percent>%` when percent, else `<phase> <done>/<total>` when both, else `<phase>`; ≤ 32 chars.
4. **`src/run.rs` — `on_stderr_line(sink)`.** The stderr drain thread already exists (`exec.rs`); add: when a sink is set, split the stream on `\r` and `\n` as bytes arrive (carry a partial segment across reads), call `sink(segment)` for each complete non-empty segment, flush the remainder at EOF. Stderr is still captured whole into `GitOutput.stderr` and still capped. The sink runs on the drain thread — document that. Order preserved. No behaviour change without a sink.
5. **`src/run/askpass.rs` — `AskPass`, `Secret`, `AskPassServer`, `respond`, `GitInvocation::askpass`.** Server: `std::env::temp_dir()/graft-askpass-<pid>-<random>` created with mode `0700` (`std::os::unix::fs::DirBuilderExt`), socket `ask.sock` inside; one listener thread; per connection: read `u32 BE len` + prompt (cap 4 KiB; over → close), `handler.ask(&prompt)` (blocking), reply `u8 status` (0 = answer, 1 = cancelled) + `u32 BE len` + bytes; close. Shutdown: `Drop` sets a flag, connects once to itself to unblock `accept`, joins, unlinks socket and dir; every step's error goes to a `tracing`/`eprintln`-free path — collect into `AskPassServer::close(self) -> io::Result<()>` that `Drop` calls and ignores **only** with a comment saying why (a failed unlink of a 0700 temp dir is not a user-visible failure); prefer callers use `close`. `respond(socket, prompt, out)`: connect, send, read; status 1 → `Err(RespondError::Cancelled)`; status 0 → write bytes + `\n` to `out`. `Secret`: `Drop` zeroes with `std::ptr::write_volatile` per byte (no new dependency); `Debug` → `Secret(…)`; no `Display`, no `Clone`. `GitInvocation::askpass(self, &server)`: `.env("GIT_ASKPASS", helper)`, `.env("GRAFT_ASKPASS_SOCKET", socket_path)`, `.env("SSH_ASKPASS", helper)`, `.env("SSH_ASKPASS_REQUIRE", "force")`; `GIT_TERMINAL_PROMPT=0` stays. The helper path is the caller's (`AskPassServer::spawn(helper, handler)` stores it) — the runner never guesses at a binary.
6. Tests (fast tier): parsers table-driven; `on_stderr_line` — a `git fetch --progress` from a `Fixture::bare_repo()`-backed remote with ≥ 50 objects yields ≥ 1 segment `parse_progress` accepts, and `GitOutput.stderr` equals the concatenation of segments plus separators; a segment split across two pipe reads reassembles (unit test over the splitter with a scripted reader); askpass — server + a client thread calling `respond` round-trips a 3-byte secret and a `None` → `Cancelled`; a real `git ls-remote` against a `file://` remote with `askpass` set still works (no prompt → no connection, `handler` never called); `Secret` bytes are zero after drop (read through a raw pointer captured before drop — `unsafe`, justified in a comment; or skip and say so). Cancel: `handler.ask` blocks; dropping the server while a client waits → client gets `Err`, thread joins within 1 s.
7. MAP.md lines for every public item; `lib.rs` module lines.

Out: any UI; deciding what text the user sees (that is graft-ops' `explain` and the app); the app's askpass mode (G5b); `RunError` changes (choice 3: none).

**Verify in a throwaway repo before coding:** (a) exact stderr of a non-fast-forward push, a stale-info lease push, a wrong-password HTTP fetch (`GIT_TERMINAL_PROMPT=0`, `-c credential.helper=`), a `merge --ff-only` on diverged branches, a `merge --ff-only` over a dirty file — with the installed git version noted; (b) that `GIT_ASKPASS` pointing at a path *with a space* is exec'd correctly by git (it is run through `sh -c` only when it contains shell metacharacters — record what you observe; the env-var mode selection exists so no argument is needed either way).

## Signatures

```rust
// crates/graft-git/src/remote.rs
#[derive(Clone, Debug, PartialEq, Eq)] pub struct Remote { pub name: String, pub fetch_url: String, pub push_url: Option<String> }
pub fn list(repo: &RepoHandle) -> Result<Vec<Remote>, RemoteError>;
#[derive(Clone, Debug, PartialEq, Eq)] pub struct TrackedUpstream { pub remote: String, pub remote_branch: String, pub tracking_ref: String, pub tracking_tip: Option<Oid> }
pub fn upstream_of(repo: &RepoHandle, branch: &str) -> Result<Option<TrackedUpstream>, RemoteError>;
#[derive(Debug)] pub enum RemoteError { Config(String), Refs(RefsError) }   // Display one line
// crates/graft-git/src/remote/failure.rs
#[derive(Clone, Debug, PartialEq, Eq)] pub enum RemoteFailure { AuthFailed { target: String }, NonFastForward, StaleInfo, RemoteRejected { reason: String }, NoSuchRemote { name: String }, Unreachable { target: String }, NotFastForward, WouldOverwrite { paths: Vec<String> }, Cancelled, Other { first_line: String } }
pub fn classify_failure(stderr: &[u8]) -> RemoteFailure;
// crates/graft-git/src/remote/progress.rs
#[derive(Clone, Debug, PartialEq, Eq)] pub struct RemoteProgress { pub phase: String, pub percent: Option<u8>, pub done: Option<u64>, pub total: Option<u64>, pub rate: Option<String> }
pub fn parse_progress(segment: &[u8]) -> Option<RemoteProgress>;
impl RemoteProgress { pub fn brief(&self) -> String; }
// crates/graft-git/src/run.rs (additive)
impl GitInvocation {
    pub fn on_stderr_line(self, sink: Box<dyn FnMut(&[u8]) + Send>) -> Self;
    pub fn askpass(self, server: &AskPassServer) -> Self;
}
// crates/graft-git/src/run/askpass.rs
pub trait AskPass: Send + Sync { fn ask(&self, prompt: &str) -> Option<Secret>; }
pub struct Secret(Vec<u8>);
impl Secret { pub fn new(bytes: Vec<u8>) -> Secret; pub fn expose(&self) -> &[u8]; }
impl Drop for Secret {}  impl fmt::Debug for Secret {}
pub struct AskPassServer { /* dir, socket path, helper, thread, stop flag */ }
impl AskPassServer { pub fn spawn(helper: PathBuf, handler: Arc<dyn AskPass>) -> io::Result<AskPassServer>; pub fn socket_path(&self) -> &Path; pub fn helper(&self) -> &Path; pub fn close(self) -> io::Result<()>; }
impl Drop for AskPassServer {}
#[derive(Debug)] pub enum RespondError { Io(io::Error), Cancelled, Protocol(&'static str) }
pub fn respond(socket: &Path, prompt: &str, out: &mut dyn Write) -> Result<(), RespondError>;
```

## Acceptance

Every tick names its verifying test inline, at tick time (ADR-0035).

- [ ] `list` on a `LocalRemote`/bare-backed clone returns `origin` with the bare's URL; on a repo with no remotes returns `Ok(vec![])` — `remote::tests::list_names_origin_first`, `list_is_empty_without_remotes`
- [ ] `upstream_of("refs/heads/main")` yields `origin`/`refs/heads/main`/`refs/remotes/origin/main`/the tip; a branch without `branch.*.merge` → `Ok(None)` — `upstream_of_reads_config_and_tip`, `upstream_of_is_none_without_tracking`
- [ ] Every `RemoteFailure` variant has a test built from real git stderr bytes naming the git version — `failure::tests::<variant>_from_real_stderr` (10 tests)
- [ ] `parse_progress` accepts the eight documented segment shapes and rejects a ref-update line and a `hint:` line — `progress::tests::*`
- [ ] A real `fetch --progress` through `on_stderr_line` yields progress segments and an unchanged `GitOutput.stderr` — `run::tests::stderr_sink_sees_progress_and_capture_is_whole`
- [ ] Segment splitting survives arbitrary read boundaries — `run::tests::segments_reassemble_across_reads` (proptest over split points, case count from `graft_fixtures::proptest_cases` if available, else 64 fixed)
- [ ] Askpass round trip, cancel, and server drop while a client waits — `askpass::tests::{round_trip, cancelled_answers_status_1, drop_unblocks_waiting_client}`
- [ ] Socket dir is `0700`, removed after `close` — `askpass::tests::socket_dir_is_private_and_removed`
- [ ] `GitInvocation::askpass` sets the four variables and leaves `GIT_TERMINAL_PROMPT=0` — `run::tests::askpass_env_is_set`
- [ ] `Secret` never appears in `{:?}` — `askpass::tests::secret_debug_is_redacted`
- [ ] MAP.md updated for `remote`, `run`, `run/askpass`

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` · `cargo nextest run -p graft-git` · `cargo test --doc -p graft-git`.

## Size

One context. Brownfield in `run.rs`/`exec.rs` — send a scout (haiku/sonnet, ≤ 1 page) for `exec.rs`'s drain threads before touching them; report whether you did. Stop seam: after piece 4 (parsers + sink land without askpass); pieces 5–6 in a handoff note.
