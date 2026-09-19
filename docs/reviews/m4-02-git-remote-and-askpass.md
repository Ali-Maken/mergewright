# Review: M4 group 02 — git-remote-and-askpass

Ticket: `.scratch/m4-remotes/issues/02-git-remote-and-askpass.md`
Diff: merge `807a425` (`96c7cf3`, `11742b0`, `ffc7a55`, `4bc3619`), scoped to `crates/graft-git` (confirmed via `git show 807a425 --stat`; the wider `5f6e6fd..807a425^2` diff also shows `graft-fixtures` deletions, but those are divergent-branch noise — the actual merge touches only `graft-git` plus the ticket file).
Design reference: `docs/design-notes/m4-remotes.md`, hard choices 2 (askpass) and 3 (`classify_failure`).

## Gates run

Per the conductor's scope limit (other builders concurrent), I ran only:
- `cargo test -p graft-git` (`CARGO_TARGET_DIR=target-review02`): **275 passed, 0 failed**, ~9.8s wall under nextest (~25s including a cold build).
- `cargo nextest run -p graft-git --stress-count 10` restricted to the three askpass concurrency tests (`round_trip`, `cancelled_answers_status_1`, `drop_unblocks_waiting_client`): **10/10 iterations, 60/60 individual runs passed**, no flakes.
- Did **not** run `cargo fmt --check` / `cargo clippy` / heavy-tests clippy / the workspace suite, per the scope instruction. The ticket's own "Gates" section reports these green; not independently reverified here.

5 slowest tests (all pre-existing, none from this group except #5):
1. `diff_fixture::parse_unified_matches_numstat_for_every_shape` — 6.99s
2. `diff::patch::tests::whole_hunk_reproduces_gits_own_bytes_on_every_corpus_shape` — 3.25s
3. `history::table::tests::every_corpus_shape_round_trips_with_a_commit_graph` — 1.45s
4. `status::tests::read_matches_gits_own_counts_and_letters_on_every_shape` — 1.30s
5. `run::tests::stderr_sink_sees_progress_and_capture_is_whole` — 1.18s (new; a real 20-commit fetch through the stderr sink — reasonable)

Peak memory footprint (`/usr/bin/time -l cargo test -p graft-git`, wrapper process): 57 MB. `target-review02/` size: 611 MB (debug, includes the whole workspace dependency graph).

## Ticks checked

Checked all 9 acceptance ticks (every named test exists, compiles, and passed in the run above): `list`/`upstream_of` reads (7 tests), `classify_failure` (12 tests, 10 real + 1 documented-SSH + 1 real-404), `parse_progress` (5 tests), stderr sink + capture (1), segment splitting incl. proptest (2), askpass round-trip/cancel/drop (3, plus stress-tested), socket dir perms/cleanup (2), askpass env vars + credential-free passthrough (2), password-never-leaks (2), MAP.md (doc, read directly). **9/9 held.**

## Findings

### Major

**M1 — `AskPassServer::shutdown`'s wake-failure comment is wrong; the failure path permanently strands the listener thread.**
`crates/graft-git/src/run/askpass.rs:215-245`. When the "wake" `UnixStream::connect(&self.socket)` fails, the code leaves the listener thread unjoined with the comment "it exits at its next connection." But `shutdown()` unconditionally removes the socket file and directory right afterward (`note(std::fs::remove_file(&self.socket)); note(std::fs::remove_dir(&self.dir));`). Once the path is gone, no process can ever connect to wake the thread — `accept()` on the already-open fd will never return, so the "next connection" the comment promises can never arrive. On this (rare, e.g. `EMFILE`/`ENOBUFS`) failure branch the thread — and its blocked socket fd — leaks for the life of the process, silently (the error is dropped, correctly per rule 3's "safe to ignore" comment for the *directory-removal* case, but that comment doesn't cover this thread-leak consequence). This is exactly the risk the design note names for this group ("a wrong shutdown order strands a listener thread"), and no test exercises it — the existing `drop_unblocks_waiting_client` test only proves the common case where wake succeeds. Fix: either join unconditionally with a bounded timeout, or fix the comment and add a regression test that forces the wake-connect to fail (e.g. remove the socket first) and asserts the thread is *not* silently expected to self-heal.

### Minor

**m1 — No write timeout on the reply socket (`write_reply`, `askpass.rs:335`).** Only `set_read_timeout` is set (for the prompt phase); the final answer write has none. In practice this is very unlikely to block (answers are small relative to default Unix-socket buffers), but it's the one write in the credential path with no bound, and `close()`'s `join()` would hang on it if it ever did. Cheap to add alongside the existing read timeout for defense in depth.

**m2 — `read_segmented`'s poisoned-sink error is reported as `RunError::Spawn`.** `run/exec.rs`: a sink that panicked on an earlier clone's run surfaces on the next run as `io::Error::other("the stderr sink panicked...")`, which `apply()` maps to `RunError::Spawn`. That variant's callers/`Display` frame it as "git could not be spawned," which is misleading for what is actually an app-side bug (a panicking callback). Cosmetic today (nothing branches on the variant), but worth a dedicated `RunError` case or at least a doc note if this is intentional.

**m3 — `classify_failure`'s dropped `unable to access '<url>'` trigger is a self-authorized deviation from the ticket's literal spec** (`remote/failure.rs`, NOTES deviation #2). The reasoning is sound and tested (the phrase also prefixes 403/TLS errors, which would be misclassified as `Unreachable`), and every case it affects still resolves correctly through the other rules (`Could not resolve host`, `Failed to connect to`, etc., all still match, confirmed by `unreachable_from_real_stderr`). Not a defect, but per CLAUDE.md a ticket-level heuristic change like this is worth explicit conductor sign-off rather than only a self-reported NOTES line, since it changes what `RemoteFailure::Unreachable` promises for HTTP 403/TLS stderr shapes (they now land in `Other`, which is honest but was not the ticket's original design).

**m4 — `WouldOverwrite` path parsing is only tested with a single path.** Both real-stderr fixtures (`FF_ONLY_DIRTY_TRACKED`/`_UNTRACKED`) list exactly one file. The `map_while(strip_prefix('\t'))` logic looks correct for a longer list, but it's unverified — a two-or-more-file real capture would close the gap.

**m5 — Prompt/failure text is passed through raw with no escape-sequence handling in this crate.** `RemoteFailure`'s `target`/`reason`/`first_line` and the askpass `prompt: &str` all carry attacker- or remote-reachable text (a hostile pre-receive hook, a crafted URL) verbatim. This ticket is correctly scoped out of UI (`Out: any UI`), and the app almost certainly runs everything through the existing `term::sanitize` at draw time — but there's no note tying this crate's raw strings to that expectation. Worth a one-line callout in the G5b (askpass UI wiring) ticket so it isn't assumed handled.

## Assessment

Core mechanics — socket 0700 perms + removal, `Secret` zeroing (safe-code, `unsafe`-forbidden, tested on a live buffer), password never touching argv/env/Debug/command-line/child-env/git-stderr (real end-to-end exchange, tested), segment splitting across arbitrary read boundaries (proptest), and the failure/progress parsers against real captured git 2.54 stderr — all solid and well tested. No password leak found. No error silently swallowed without a "why safe" comment. One real (if narrow) logic bug in shutdown ordering (M1) in the exact area the design note flagged as risky; the rest is polish.
