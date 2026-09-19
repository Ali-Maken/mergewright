# M4 design pass: remotes (write) (2026-09-19)

Read: `.scratch/m4-remotes/scout.md` (whole — facts about current code); CLAUDE.md; PROMPT.md
principles 2/3/5/6, Technology "Writing to the repo", §3 "Long operations"/"Feedback", §4
Remotes, §8 M4; docs/design 01 §1.1 top bar, §1.10, §1.14; 02 §2.3 `f p P`, §2.14; 06 §6.4 Push
row; 08 §8.1–8.3; ADR-0033/0034/0035/0036; every `crates/*/MAP.md`; M3 review 06 (C1, M4, M5);
KNOWN_GAPS M4 rows. No source.

**Promise:** *you never leave Graft to fetch, pull or push — not for a password, a rejection or a
hang.* Measured by "Times I left" (ADR-0033). The acceptance walk proves it against real
remotes: `file://`, smart HTTP with Basic auth (wrong password first), and one that never answers.

**Scope in:** `f` fetch all + prune, `p` fast-forward pull (undoable), `P` push incl. set-upstream,
the lease-guarded force push offered after a non-fast-forward rejection, in-app askpass, `Esc`
cancels any running write, live progress, plain-English failures, real-remote test harness.
**Out (Proposed changes, user decides):** clone → M8 (needs the first-run screen); add/rename/
remove remotes → M7 (rare; refusal names the shell command); pull with merge/rebase → M5 (M4
refuses a diverged pull honestly, naming the command); `notify` watcher (KNOWN_GAPS "M4") → M7.

**Sizing from M3 (ADR-0035):** greenfield ~200 turns, brownfield 430–940 because specs were wrong
about existing code. Here every brownfield group is one pane or one interaction, sized against the
scout, and every git claim is a "verify in a throwaway repo first" item.

## Interfaces and data shapes

Seam rule holds: `app` builds an `Operation` and a `Command::Operate`; `ops` decides plan, check,
journal; `git` runs it. New: remote invocations stream stderr (progress) and carry an askpass hook.

```rust
// graft-fixtures/src/remote.rs                                   (G1, greenfield)
pub struct LocalRemote;   // bare `git clone --bare <shape>` + a work clone of it (real origin, real upstreams)
impl LocalRemote { pub fn new(shape: Shape) -> Result<Self, Unavailable>; pub fn bare(&self) -> &Path; pub fn work(&self) -> &Path;
    pub fn commit_on_remote(&self, subject: &str) -> io::Result<Oid>;   // via a second clone: work falls behind
    pub fn commit_locally(&self, subject: &str) -> io::Result<Oid>; }   // work goes ahead
pub struct HttpRemote;    // smart HTTP: tiny_http (dev-dep) → `git http-backend` CGI; Basic auth; 127.0.0.1 ephemeral port
impl HttpRemote { pub fn serve(bare: &Path, user: &str, password: &str) -> io::Result<Self>;
    pub fn url(&self) -> String /* http://user@127.0.0.1:p/repo.git */; pub fn url_anonymous(&self) -> String;
    pub fn requests(&self) -> (usize /*authorised*/, usize /*rejected 401*/); }
pub struct HungRemote;    // accepts TCP, never answers
impl HungRemote { pub fn listen() -> io::Result<Self>; pub fn url(&self) -> String; pub fn connections(&self) -> usize; }

// graft-git/src/remote.rs                                        (G2)
pub struct Remote { pub name: String, pub fetch_url: String, pub push_url: Option<String> }
pub fn list(repo: &RepoHandle) -> Result<Vec<Remote>, RemoteError>;             // gix config, no spawn; `origin` first
pub struct TrackedUpstream { pub remote: String, pub remote_branch: String /* refs/heads/x */, pub tracking_ref: String /* refs/remotes/o/x */, pub tracking_tip: Option<Oid> }
pub fn upstream_of(repo: &RepoHandle, branch: &str /* full */) -> Result<Option<TrackedUpstream>, RemoteError>;
pub enum RemoteFailure { AuthFailed { target: String }, NonFastForward, StaleInfo, RemoteRejected { reason: String },
    NoSuchRemote { name: String }, Unreachable { target: String }, NotFastForward, WouldOverwrite { paths: Vec<String> },
    Cancelled, Other { first_line: String } }
pub fn classify_failure(stderr: &[u8]) -> RemoteFailure;                          // pure; LC_ALL=C English, phrases listed in the spec
pub struct RemoteProgress { pub phase: String, pub percent: Option<u8>, pub done: Option<u64>, pub total: Option<u64>, pub rate: Option<String> }
pub fn parse_progress(segment: &[u8]) -> Option<RemoteProgress>;  impl RemoteProgress { pub fn brief(&self) -> String /* `receiving objects 45%` */ }
// graft-git/src/run.rs (additive)
impl GitInvocation { pub fn on_stderr_line(self, sink: Box<dyn FnMut(&[u8]) + Send>) -> Self;   // per `\r`/`\n` segment, drain thread; stderr still captured
                     pub fn askpass(self, server: &AskPassServer) -> Self; }                       // GIT_ASKPASS, GRAFT_ASKPASS_SOCKET, SSH_ASKPASS, SSH_ASKPASS_REQUIRE=force
// graft-git/src/run/askpass.rs
pub trait AskPass: Send + Sync { fn ask(&self, prompt: &str) -> Option<Secret>; }   // blocks on the listener thread; None = cancelled
pub struct Secret(Vec<u8>);  // Drop zeroes; Debug prints `Secret(…)`; `expose(&self) -> &[u8]`
pub struct AskPassServer;  impl AskPassServer { pub fn spawn(helper: PathBuf, handler: Arc<dyn AskPass>) -> io::Result<Self>; pub fn socket_path(&self) -> &Path; }  // Drop: join, unlink, rmdir
pub fn respond(socket: &Path, prompt: &str, out: &mut dyn Write) -> Result<(), RespondError>;  // the helper half: exit 1 on Cancelled

// graft-core/src/journal.rs: OperationKind += Fetch, Pull, Push                (G3)
// graft-ops/src/operation.rs
pub enum Operation { …, Fetch(FetchParams), Pull(PullParams), Push(PushParams) }
pub struct FetchParams { pub remote: Option<String> /* None = --all */, pub prune: bool }
pub struct PullParams { pub branch: String /* full */, pub upstream: TrackedUpstream }
pub struct PushParams { pub branch: String, pub remote: String, pub remote_branch: String, pub set_upstream: bool, pub lease: Option<Lease> }
pub struct Lease { pub expected: Oid }        // --force-with-lease=<remote_branch>:<expected> (2026-09-20: --force-if-includes dropped, a no-op beside an explicit expect; Graft checks the fetched tip itself — review m4-04)
impl Operation { pub fn is_remote(&self) -> bool; pub fn preview(&self) -> Option<Vec<String>>; }  // command lines needing no facts (remote ops only): the push box
pub struct Invocation { …, pub remote: bool }  pub struct RepoFacts { …, pub remotes: Vec<Remote>, pub upstream: Option<TrackedUpstream> }
pub enum Refusal { …, DetachedHead, NoRemote, NoSuchRemote { name: String }, NoUpstream { branch: String } }
// graft-ops/src/journal.rs
pub fn undoable_kind(kind: OperationKind) -> bool;   // false for Fetch, Push; next_undo/next_redo honour it (signatures unchanged)
// graft-ops/src/apply.rs
pub struct RemoteHooks { pub progress: Box<dyn Fn(RemoteProgress) + Send + Sync>, pub askpass: Arc<dyn AskPass>, pub helper: PathBuf }
pub struct Context { …, pub remote: Option<RemoteHooks> }   // None = headless: no prompt, no progress (tests)
pub enum ApplyError { …, Remote { entry: Box<JournalEntry>, failure: RemoteFailure, stderr_tail: Vec<String> } }  // explain() words each failure

// graft (app)                                                     (G5a, G5b)
pub struct RemoteFacts { pub remotes: Vec<Remote>, pub head_branch: Option<String>, pub head_upstream: Option<TrackedUpstream> }
// Command::ReadRemotes(JobId) → Product::Remotes(Result<Arc<RemoteFacts>, String>); AppState::remotes: Loading<Arc<RemoteFacts>>
// update/remote.rs: fetch_key, pull_key, push_key (the box), cancel_operating (Esc), remote_progress, remote_failed
// Progress::RemoteProgress { brief: String }; Progress::CredentialNeeded { prompt: String }; Command::AnswerCredential(JobId, Secret)
// update/credential.rs: CredentialPrompt { title, prompt, field: TextField, masked, job }; in AppState::text_field_open()
// cli::Invocation::Askpass { socket, prompt } — chosen when env GRAFT_ASKPASS_SOCKET is set; main calls graft_git::run::askpass::respond
```

## Hard-to-reverse choices

1. **What `u` does after a push.** (a) DESIGN §6.4: `u` on a push opens a lease-guarded re-push of
   the pre-push state. (b) Push and fetch are journaled (Command Log, browser row) but
   **transparent to undo**: `next_undo`/`next_redo` skip them, `inverse` refuses
   `NotUndoable(Push|Fetch)`, so `u` after a push undoes the commit *locally* and the status says
   `· origin/main still has it`. Pull stays fully undoable (local branch + tree; the tracking ref
   is not in its `touches`, so undo never lies about the remote). **(b).** Why: undo stays 100 %
   local — no network, no password prompt, no hang inside the one key the promise rests on; the
   remote is moved only by `P`, through one rail (a rejection → the lease box). Scenario "origin
   back, keep local" is `u`, `P` (lease box), `U` — three visible steps, none a surprise. Before a
   push the box says `can't be undone from here — origin keeps what you send`.
2. **Credentials.** (a) `GIT_ASKPASS` = Graft's own binary in askpass mode, talking back over a
   0700-dir Unix socket named in `GRAFT_ASKPASS_SOCKET`; (b) a credential helper; (c) URL-embedded.
   **(a)** (DESIGN §1.14). Args can't ride in `GIT_ASKPASS` reliably (git may exec via `sh`), so
   the mode is chosen by the env var. `SSH_ASKPASS` + `SSH_ASKPASS_REQUIRE=force` set alongside
   (unverified in tests — say so). Must never: put the secret in an argv, `commands`, `Status`, a
   log line, a `Debug` print (`CredentialPrompt` redacts, `Secret` redacts) or a capture (masked
   `•`); the user's own credential helpers keep working (we only add askpass).
3. **Failure typing.** (a) new `RunError` variant; (b) `graft_git::remote::classify_failure` over
   the *full* stderr the streaming sink collected, giving `RemoteFailure`. **(b).** `RunError`
   keeps only a first line; the classifier needs the `[rejected]`/`stale info` lines further down.
   Consequence owned: English string-matching, stable under the runner's `LC_ALL=C`; every miss is
   `Other { first_line }` shown verbatim — never hidden, never misnamed.
4. **No timeout; `Esc` cancels.** A wall-clock timeout kills slow large fetches; a hang is visible
   as progress that stops. `Esc` while `operating` → `CancelJob` → SIGTERM (2 s grace) → entry
   `Aborted("cancelled")`. G5a owns it, for every write, not only remote ones (M3 left the gap).
   A "no data for N s" notice needs a tick the loop lacks → KNOWN_GAPS.
5. **Progress ships.** Streaming stderr segments → `parse_progress` → `Progress::RemoteProgress`
   (≤ 10/s) → the footer's `Busy` text. Small and PROMPT names it.

## Where I'd differ from DESIGN.md (Proposed changes)

1. §6.4 Push row: transparent-to-undo instead of a guided re-push under `u` (choice 1).
2. §1.14 one box with two fields → one field per ask: git calls askpass once for the username,
   once for the password; SSH host-key questions arrive the same way (unmasked). Truthful, smaller.
3. §4 "fetch one remote": `f` fetches `--all --prune`; per-remote fetch waits for the sidebar (M7).
4. A box before a push, none before pull/fetch. Rule: a box iff the write is not undoable or
   destroys uncommitted work. Pull is undoable; fetch changes nothing the user typed.
5. Scope cuts above (clone, remote management, merge/rebase pull, watcher).

## New UI (per-breakpoint; captures carry no colour, so every fact is text)

**Push box** — `graft_ui::confirm::ConfirmView`, no new widget; centred, `Cancel` under cursor.
```
80x24                                                     160x48 (same box, wider consequence on one line)
╭ Push main → origin/main ───────────────────────────╮    ╭ Push main → origin/main ──────────────────────────────────────────────────╮
│ 2 commits · sets upstream                           │    │ 2 commits · sets upstream · can't be undone from here — origin keeps       │
│ can't be undone from here — origin keeps what you   │    │ what you send                                                              │
│ send                                                │    │ git push --progress -u origin refs/heads/main:refs/heads/main              │
│ git push --progress -u origin refs/heads/main:refs/ │    │                                        [ Cancel ]   [ Push ]               │
│ heads/main                                          │    ╰ ←→ choose  ⏎ act  esc cancel ─────────────────────────────────────────────╯
│                        [ Cancel ]   [ Push ]        │
╰ ←→ choose  ⏎ act  esc cancel ─────────────────────╯
Lease variant: title `Force-push main → origin/main?`; line 1 `origin/main 9f8e7d6 → a1b2c3d · refused if origin moved since your fetch (lease)`;
line 2 `rewrites history for everyone who has 9f8e7d6`; command with --force-with-lease=…:9f8e7d6; button `[ Force push ]`.
```
**Credential prompt** — new `graft_ui::credential_prompt`, centred, modal, one field. Eye lands
on the title, then the field. Masked when the prompt names a password/passphrase; else plain.
```
80x24                                                     160x48
╭ Credentials — 127.0.0.1:8443 ─────────────────────╮    ╭ Credentials — 127.0.0.1:8443 ─────────────────────────────────────────────╮
│ Password for 'http://user@127.0.0.1:8443':         │    │ Password for 'http://user@127.0.0.1:8443':                                 │
│ ▏••••••••_                                         │    │ ▏••••••••_                                                                 │
│ typed here only · never stored by Graft            │    │ typed here only · never stored by Graft                                    │
╰ ⏎ send  ⌃U clear  esc cancel the push ────────────╯    ╰ ⏎ send  ⌃U clear  esc cancel the push ────────────────────────────────────╯
Username ask: line 1 `Username for 'https://github.com':`, field unmasked. SSH host key: `Are you sure you want to continue connecting (yes/no/[fingerprint])?`, unmasked.
Empty field + ⏎: sends an empty answer (git decides). Esc: cancels the whole operation (SIGTERM), never leaves it half-authenticated.
```
**Footer while a remote write runs** (existing `Status::Busy` slot, text only):
`fetch all remotes · receiving objects 45% (450/1000) · esc cancels` → on Esc `fetch all remotes · cancelling…`.
At 80 cols: `fetch all remotes · receiving 45% · esc cancels` (the `brief` drops counts under 100 cols).
**Failure line** (`Status::Sticky`, stays until Esc): `push rejected: origin/main has commits you don't · p to pull first, or P to force with lease` /
`authentication failed for 127.0.0.1:8443 · P to try again` / `pull stopped: main and origin/main diverged — merge or rebase (M5); shell: git pull --rebase` /
`pull refused: would overwrite src/a.rs (uncommitted) · commit or stash first` / `origin unreachable: Could not resolve host: github.com` /
`push refused: origin/main moved since your last fetch (stale lease) · f to fetch, then look`.
**Journal browser rows** (existing screen): Push `push main → origin/main · origin/main a1b2c3d → 9f8e7d6 · not undoable — origin has it`;
Fetch `fetch all remotes · remote-tracking refs · not undoable`; Pull `pull origin/main · 1 ref · the index · the working tree · backup saved  ← u`.
**After `u` skips a push:** `undid: commit "x" · origin/main still has it`. **Hints** gain `f fetch  p pull  P push` (after `S stash`).

## Genuinely risky groups

- **git-remote (G2)**: the askpass socket server and the streaming drain run beside the child
  supervisor — a wrong shutdown order strands a listener thread or a socket file. Judged by tests
  (hermetic client/server, a real fetch with progress) and the reviewer's cancel-under-load run.
  Not Fable: approach decided here, correctness testable.
- **ops-remote (G3)**: what a Fetch/Pull/Push entry *means* in the journal becomes structural for
  M5's Ghost Preview and the browser; but every rule is stated and testable (skip rule, tracking
  ref out of Pull's touches, `Refresh::Everything`). Opus.
- **credential wiring (G5b)**: secrets crossing threads and a modal field added to
  `text_field_open()` (M3 06 C1). Judged by the e2e wrong-password test and a grep of journal,
  captures and logs for the password bytes.

Fable builds: acceptance only. No group meets all three tests; the design is not underspecified —
each of the six hard problems is decided above.
