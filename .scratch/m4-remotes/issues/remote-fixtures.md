# remote-fixtures: real remotes for tests — file://, smart HTTP with Basic auth, and one that never answers

Status: draft
Build with: Opus — greenfield, but it is a hermetic HTTP server driving `git http-backend` as CGI plus two background-thread servers with clean shutdown; protocol and concurrency work, not wiring.
Crates: graft-fixtures
Needs: none

## Read first

- `docs/design-notes/m4-remotes.md` — Promise; Interfaces (the `graft-fixtures/src/remote.rs` block); Hard choice 2 (what the askpass helper is, so `HttpRemote` tests can drive it later); Genuinely risky groups
- `.scratch/m4-remotes/scout.md` §Tests (lines 93–101)
- `crates/graft-fixtures/MAP.md` (whole — `Shape`, `copy_into_temp`, `Scratch`, `git` mod)
- `crates/graft-git/MAP.md` §run (`GitInvocation`, `Stop`, `RunError`) — only to know what the consumers will call; this group does not depend on graft-git
- `PROMPT.md` §7.1 (laptop-safe tests) — grep `laptop`; ADR-0018 (heavy tests gated by feature alone), first 30 lines
- CLAUDE.md "Rules for every agent" (quiet gates, never swallow an error, tests laptop-safe)

## Scope

In — pure test infrastructure, in this order (each piece a commit; past 60 turns say what made it long):

1. `crates/graft-fixtures/src/remote.rs` + `remote/local.rs`: `LocalRemote::new(shape)` — `git clone --bare <borrow(shape)> <scratch>/remote.git`, then `git clone <bare> <scratch>/work` (real `origin`, `branch.<default>.remote/merge` set by clone). Both under one `Scratch`. `commit_on_remote(subject)`: a third clone under the same scratch, one commit on the bare's default branch (a file `remote-<n>.txt`, fixed identity/clock from `fixtures::git`), `git push origin HEAD`; returns the new tip. `commit_locally(subject)`: same in `work`, no push. `default_branch()` reads the bare's `HEAD`. Every git call through `graft_fixtures::git` (fixed identity, `GIT_CONFIG_NOSYSTEM=1`, `HOME` pointed at the scratch so the user's `credential.helper`, `push.default`, `pull.rebase` can never leak into a test).
2. `remote/http.rs`: `HttpRemote::serve(bare, user, password)`. Add `tiny_http` as a **dev-dependency**? No — this is library code the `graft` crate's tests call, so it is a normal dependency of `graft-fixtures` (a dev-only crate). One listener thread on `127.0.0.1:0`. Per request: no/wrong `Authorization: Basic` → `401` with `WWW-Authenticate: Basic realm="graft"`; right → spawn `git http-backend` with CGI env (`GIT_PROJECT_ROOT=<parent of bare>`, `GIT_HTTP_EXPORT_ALL=1`, `PATH_INFO=/repo.git/<rest>`, `QUERY_STRING`, `REQUEST_METHOD`, `CONTENT_TYPE`, `CONTENT_LENGTH` (de-chunk first if the client sent `Transfer-Encoding: chunked` — tiny_http hands a de-chunked body reader, verify), `HTTP_CONTENT_ENCODING` (git gzips upload-pack requests; http-backend inflates when told), `REMOTE_USER`), body piped to stdin, CGI headers parsed (`Status:` → code, `Content-Type`), body streamed back. Set `http.receivepack=true` on the bare (`git config http.receivepack true`) so push works over HTTP. Counters: authorised / rejected requests. `Drop`: `server.unblock()`, join with a 2 s bound, error to stderr if it did not stop (never a hang in a test's teardown).
3. `remote/hung.rs`: `HungRemote::listen()` — `TcpListener` on `127.0.0.1:0`, a thread that `accept`s and parks each connection (holds the stream, never reads or writes) until dropped; `connections()`. `Drop` closes every held stream and the listener.
4. Tests (`remote/tests.rs`, fast tier, each < 2 s, all through real `git` in a `Scratch`):
   - `local_remote_clone_has_origin_and_upstream`: `git -C work remote get-url origin` = bare path; `git -C work rev-parse --abbrev-ref main@{upstream}` = `origin/main` (or the shape's default).
   - `commit_on_remote_leaves_work_behind`: after it, `git -C work fetch` then `rev-list --count main..origin/main` = 1.
   - `http_wrong_password_is_401_and_git_says_authentication_failed`: `git -c credential.helper= ls-remote <url_anonymous>` with `GIT_TERMINAL_PROMPT=0` → exit 128, stderr contains `Authentication failed` or `could not read Username`; `requests()` = `(0, ≥1)`.
   - `http_right_password_clones_and_pushes`: `GIT_ASKPASS=<a shell script written by the test that echoes the password>` (tests may use a script — the real helper is the app's, G5b) → `git clone <url>` succeeds, then a commit + `git push` succeeds, bare's tip moves; `requests().0 ≥ 2`. Also a push large enough to be chunked (a 2 MiB random file) — this is the CGI body-handling test.
   - `hung_remote_blocks_then_dies_on_sigterm`: `git fetch <hung url>` as a `std::process::Child` with `GIT_TERMINAL_PROMPT=0`; after 300 ms it is still running (`try_wait` = None), `connections()` = 1; `kill` (SIGTERM via `libc`? — use `Child::kill`, which is SIGKILL; the SIGTERM behaviour is graft-git's `run`, tested there) → exits.
5. `crates/graft-fixtures/MAP.md`: one line per public item; `Cargo.toml` dependency line commented with why (`tiny_http`: smart HTTP for remote tests; no async).

Out: anything in graft-git/graft-ops/graft (they consume this); SSH; TLS (plain HTTP is enough to exercise Basic auth — say so in the module doc); a heavy-tier large remote.

**Verify in a throwaway directory before coding** (each a one-liner, results in the ticket NOTES): (a) `git http-backend` needs `GIT_HTTP_EXPORT_ALL` or `git-daemon-export-ok`; (b) whether git's push over HTTP sends `Transfer-Encoding: chunked` for a 2 MiB pack (it does past `http.postBuffer`, 1 MiB) and whether tiny_http de-chunks it; (c) http-backend with `HTTP_CONTENT_ENCODING=gzip` inflates itself; (d) `git config http.receivepack true` on the bare is required for anonymous-user push through http-backend even when authenticated (it is — `REMOTE_USER` set enables it; test both).

## Signatures

```rust
// crates/graft-fixtures/src/remote.rs  (pub mod remote; re-exports the three types)
pub struct LocalRemote { /* scratch, bare, work, default_branch */ }
impl LocalRemote {
    pub fn new(shape: Shape) -> Result<LocalRemote, Unavailable>;
    pub fn bare(&self) -> &Path;
    pub fn work(&self) -> &Path;
    pub fn url(&self) -> String;                                   // file:// URL of `bare`
    pub fn default_branch(&self) -> &str;                          // short name
    pub fn commit_on_remote(&self, subject: &str) -> io::Result<Oid>;
    pub fn commit_locally(&self, subject: &str) -> io::Result<Oid>;
    pub fn git(&self, dir: &Path, args: &[&str]) -> io::Result<String>;   // hermetic git, stdout trimmed; Err carries stderr
}
pub struct HttpRemote { /* thread, port, counters */ }
impl HttpRemote {
    pub fn serve(bare: &Path, user: &str, password: &str) -> io::Result<HttpRemote>;
    pub fn url(&self) -> String;              // http://<user>@127.0.0.1:<port>/repo.git
    pub fn url_anonymous(&self) -> String;    // http://127.0.0.1:<port>/repo.git
    pub fn port(&self) -> u16;
    pub fn requests(&self) -> (usize, usize); // (authorised, rejected)
}
impl Drop for HttpRemote {}
pub struct HungRemote { /* listener thread, held streams */ }
impl HungRemote {
    pub fn listen() -> io::Result<HungRemote>;
    pub fn url(&self) -> String;              // http://127.0.0.1:<port>/repo.git
    pub fn connections(&self) -> usize;
}
impl Drop for HungRemote {}
```

`Oid` is `graft_core::Oid` if graft-fixtures already depends on graft-core; otherwise a `String` of 40 hex and the builder says so in NOTES (do not add a graft-core dependency just for this).

## Acceptance

Every tick names its verifying test inline, at tick time (ADR-0035).

- [ ] `LocalRemote::new(Shape::<any>)` yields a work clone whose `origin` is the bare and whose default branch tracks `origin/<default>` — `local_remote_clone_has_origin_and_upstream`
- [ ] `commit_on_remote` advances the bare, not `work`; `commit_locally` the reverse — `commit_on_remote_leaves_work_behind`, `commit_locally_leaves_work_ahead`
- [ ] Wrong or missing password → 401, git reports authentication failure, counter shows the rejection — `http_wrong_password_is_401_and_git_says_authentication_failed`
- [ ] Right password → clone, fetch and a > 1 MiB push all succeed over HTTP — `http_right_password_clones_and_pushes`
- [ ] `HungRemote` holds a `git fetch` open indefinitely and counts the connection — `hung_remote_blocks_then_dies_on_sigterm`
- [ ] No test leaves a thread or a listening port behind: `cargo nextest run -p graft-fixtures` twice in a row passes; `lsof -i :<port>` after a test binary exits shows nothing (the builder runs it once, records the result)
- [ ] No test reads the user's `~/.gitconfig` or credential helpers: `HOME` and `GIT_CONFIG_NOSYSTEM` set on every child — `hermetic_env_blocks_user_config` (asserts `git config --show-origin --list` in `work` names no path under the real `$HOME`)
- [ ] MAP.md updated; module docs say what each remote is for and what it does not cover (TLS, SSH)

## Gates

`cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo clippy --all-targets --features heavy-tests --message-format=short -- -D warnings` · `cargo nextest run -p graft-fixtures` · `cargo test --doc -p graft-fixtures`. Quiet output; only the failing part on failure.

## Size

One context. Greenfield: expect ~150–250 turns. Stop seam if long: after piece 2 (HTTP) with pieces 3–4 in a handoff note. Report whether a scout was used and if not why (ADR-0036).
