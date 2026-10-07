# 13j — HttpRemote owns its listener (cause fix for 13i)

Status: done (builder 2026-10-08) — 8 of 9 ticks; load-loop tick open, see NOTES
Build with: Opus — test-only crate, no structural risk (fails Fable test 1), but HTTP framing, drop ordering and
process-group teardown are easy to get subtly wrong; not Sonnet.
Needs: none.
**Crate set: graft-fixtures only.** Files: `crates/graft-fixtures/{Cargo.toml,src/remote/http.rs,src/remote/tests.rs,MAP.md}`,
`Cargo.lock`, one dated amendment line in `docs/design-notes/m4-remotes.md` (line 35).
Source: planner ruling on 13i, 2026-10-07. Closes KNOWN_GAPS `M8/http-remote-port-flaky` (conductor closes the row).

## Ruling

- **Option 1: replace tiny_http with an accept loop on the fixture's own `TcpListener`.** Cause fix: `drop` joins the
  thread that owns the listener, so "port free when `drop` returns" holds by construction, as for `HungRemote`.
- Option 2 rejected: a vendored fork to maintain in a repo that may be published, for one test crate.
- Option 3 rejected: symptom (a wait standing in for ownership; CLAUDE.md rule 2).
- **No ADR.** Test-only crate (ARCHITECTURE.md "fixtures: test-only, outside the table"), no edge change, no new
  dependency (std only), reversible by `cargo add`. Record it as a dated amendment on `m4-remotes.md:35` and in the
  `Cargo.toml` comment. ADR-0015 still binds: **no new `unsafe`**.

## Read first

- `.scratch/m8-polish-and-ship/issues/13i-http-remote-port-flake.md` (whole; it is short) — cause, line refs.
- `crates/graft-fixtures/MAP.md` lines 42–45 (`remote`, `HttpRemote`, `HungRemote`).
- ARCHITECTURE.md "`fixtures`: test-only, outside the table".
- Then source: `src/remote/http.rs` (whole), `src/remote/hung.rs:100-160` (the own-listener + wake + join pattern to copy),
  `src/remote/tests.rs` HttpRemote tests (~280–420).

## Scope

In:
- `http.rs`: remove tiny_http. The listener thread (already joined by `drop`) owns a `std::net::TcpListener` and runs
  the accept loop. Each accepted connection is handled on its own thread; `HttpRemote` keeps every handler's
  `JoinHandle` and its `TcpStream` clone.
- Minimal HTTP/1.1, server side only: request line, headers (case-insensitive names), body by `Content-Length` **or**
  `Transfer-Encoding: chunked`, `Expect: 100-continue` (send `HTTP/1.1 100 Continue\r\n\r\n` before reading the body),
  `Connection: close` on every response, then close. Malformed request → `400`, close; never panic in the handler.
- Keep everything the CGI bridge does today unchanged: env mapping (incl. `Content-Encoding` → git's gzip POSTs),
  Basic auth / `401` + `WWW-Authenticate`, `requests()` counters, `http-backend` in its own process group.
- Accepted streams get a read timeout (≤ 1 s per read) so an idle or half-open client cannot hold a handler past drop.
- `drop`, in this order: set the close flag; kill every running backend group (as today); `shutdown(Both)` every live
  accepted stream; wake `accept` by one self-connect (panic if it fails, as `HungRemote` does); join the listener
  thread, then every handler, all inside the existing 2 s deadline — else panic (MAP contract unchanged).
- Fix the false comment at `http.rs:287-288` to state what is now true and why.
- `Cargo.toml`: drop `tiny_http`, rewrite its comment. `Cargo.lock`: tiny_http, ascii, chunked_transfer, httpdate gone
  (check each has no other user with `cargo tree -i <crate>` before claiming it).
- MAP.md line 44: `tiny_http →` becomes "own listener →"; add the drop order.
- `m4-remotes.md:35`: append one dated line: "2026-10-07: tiny_http replaced by own listener (13i/13j: it never joins
  its accept thread)".

Out:
- Any public API change. Callers must compile untouched.
- Refactoring `hung.rs` into a shared listener (copy the pattern; a shared module is a later cleanup, not this ticket).
- Keep-alive, pipelining, TLS, HTTP/2.

## Signatures

Public — unchanged, exactly as MAP.md line 44 lists (`serve`, `url`, `url_anonymous`, `port`, `requests`, `Drop`).
Read the current `serve` signature from `http.rs` and keep it byte-for-byte.

Private (names may differ; shape may not):

```rust
struct Request { method: String, target: String, headers: Vec<(String, String)>, body: Vec<u8> }

/// Reads one request; answers `100 Continue` itself when asked. `Ok(None)` = peer closed before a request line.
fn read_request(stream: &mut std::net::TcpStream) -> std::io::Result<Option<Request>>;

/// Status line + headers + body, always `Connection: close`.
fn write_response(stream: &mut std::net::TcpStream, status: u16, headers: &[(String, String)], body: &[u8])
    -> std::io::Result<()>;

#[cfg(test)]
/// Test seam: the listener thread sleeps this long after leaving its loop and before dropping the listener.
fn serve_with_exit_delay(/* serve's args */, exit_delay: std::time::Duration) -> Result<HttpRemote, CorpusError>;
```

## Acceptance (tick each with its test named inline)

- [x] (`remote::tests::http_remote_drop_waits_for_the_listener_to_close`: red by mutation, green after; NOTES) **Deterministic red/green**: `remote::tests::http_remote_drop_waits_for_the_listener_to_close` — via
      `serve_with_exit_delay(300 ms)`: `drop` takes ≥ 300 ms and a connect straight after it is refused. **Red** shown by
      temporarily removing the listener join (mutation): paste the failing assertion into NOTES, then restore. Green after.
- [x] (`remote::tests::http_remote_frees_its_port_on_drop`, untouched, green) `remote::tests::http_remote_frees_its_port_on_drop` passes, unchanged in name and assertion.
- [x] (`remote::tests::http_remote_drop_ends_a_wedged_backend`, untouched, green) `remote::tests::http_remote_drop_ends_a_wedged_backend` (`tests.rs:395`) passes, unchanged in name and assertion.
- [x] (`remote::tests::http_remote_reads_a_chunked_body`: asserts 200, `refs/heads/main` in the ls-refs answer, `chunked_requests()==1`, `requests()==(1,0)`) `remote::tests::http_remote_reads_a_chunked_body` — raw `TcpStream` POST with a chunked body reaches the backend
      (fetch/push outcome or `requests()` count; name what it asserts).
- [x] (`remote::tests::http_remote_answers_expect_100_continue`) `remote::tests::http_remote_answers_expect_100_continue` — raw client sends `Expect: 100-continue`, reads
      `100 Continue` before sending the body.
- [x] (`remote::tests::http_remote_drop_ends_an_idle_client`) `remote::tests::http_remote_drop_ends_an_idle_client` — a client connects and sends nothing; `drop` returns within
      2 s; port refused after.
- [x] (workspace nextest 5888/5888: graft-fixtures `remote::tests` 17, graft-ops `it::remote_ops` 20, graft `it::credential_by_key` 3, `it::m4_walk` 27, `it::m5_walk` 123 — counts from `cargo nextest list`) Every existing HttpRemote user green in the workspace run: graft-fixtures `remote::tests`, `graft-ops`
      `it::remote_ops`, `graft` `it::credential_by_key`, `it::m4_walk`, `it::m5_walk`. Name each in the tick.
- [ ] **Open — 198/200, both failures in setup, 0 on the port; see NOTES.** Load loop: `http_remote_frees_its_port_on_drop` 200× under `taskpolicy -b` with another workspace build running:
      0 failures. Record command and count in NOTES (output to a file; report the summary line only).
- [x] (`cargo tree --workspace -i` tiny_http / ascii / chunked_transfer / httpdate: "did not match any packages"; `unsafe` code only in `memory.rs`, other hits are comments and lint attributes) `cargo tree -i tiny_http` empty; no `unsafe` added (`grep -n unsafe crates/graft-fixtures/src` shows only
      `memory.rs`, ADR-0015).

## Touches (each checked and stated in NOTES)

- Callers: `graft-ops/tests/it/remote_ops.rs`, `graft/tests/it/credential_by_key.rs`, `graft/tests/it/m4_walk/{fixture,truth}.rs`,
  `graft/tests/it/m5_walk/fixture.rs` — compile and pass untouched (they are outside the crate set; if one needs an edit,
  stop: `blocked: needs <crate>`).
- `docs/capabilities/m4-remotes.md` cites HttpRemote test names: no test renamed, so `graft-capabilities` checker stays
  green; say so with its test name.
- `TESTING.md` mentions HttpRemote: grep; edit nothing outside the file set — report any stale line for the conductor.
- No keymap, journal, captures, or shipped-binary code: test-only crate (ARCHITECTURE.md), edge test unchanged.

## Gates

Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size

One context: one ~400-line file rewritten in part, ~5 tests, docs lines. ~50k tokens.

## NOTES (builder, 2026-10-08)

No scout: the ticket named the exact files and line ranges; read them directly (~700 lines). No compaction.

**Red (mutation: `Drop` returned right after the wake, no wait/join), then restored:**
```
thread 'remote::tests::http_remote_drop_waits_for_the_listener_to_close' panicked at crates/graft-fixtures/src/remote/tests.rs:430:5:
drop returned in 1.146125ms, before the listener thread let go of the socket
```
Green after restore; all 17 `remote::tests` green.

**Deviation from the spec — a refused wake no longer panics (cause found by the load loop).** First 200x loop: 74/200
passed. Every failure: "could not wake the listener (Connection refused)". Instrumented (throwaway): `thread.is_finished()
== true` at that moment. Cause: the test's open-port probe (`tests.rs:312`, connect + drop) sits in the backlog; `drop`
sets `closing`; the listener then accepts the probe, sees `closing`, returns and closes the socket — the wake is refused
because the listener is already gone, not stuck. Fix (cause): a refused wake is recorded, not fatal; step 5's 2 s bound
still panics if any thread really remains, and its message carries the wake error (`http.rs` Drop step 4 comment).
After: 40/40 sequential, then the loop below. MAP line says so. **`HungRemote` has the same latent race** (`hung.rs`
Drop panics on a refused wake); its only drop test waits for the accept first, so it is not hit today. Not changed (outside
this ticket's file set) — for the conductor: KNOWN_GAPS candidate, unverified.

**Load loop** (script in scratchpad; summary `docs/perf/13j-http-remote-own-listener.txt`):
`taskpolicy -b <graft-fixtures lib test binary> --exact remote::tests::http_remote_frees_its_port_on_drop`, 200x
sequential, cold `cargo build --workspace --release` builds back to back throughout (7 completed, separate target dir).
`SUMMARY: 198 passed, 2 failed, of 200`. Both failures panicked in setup at `tests.rs:20` (`LocalRemote::new` → clone
deadline Timeout, 37 s and 149 s under background QoS), before `HttpRemote::serve` ran. The port assertion
(`tests.rs:313`) failed 0/200. Tick left open: the criterion says 0 failures; whether setup timeouts under
`taskpolicy -b` count is the conductor's call. The fixture's clone deadline under background QoS is a separate question,
not a 13i/13j cause.

**Behaviour notes.** One handler thread per connection; `Running` now tracks every backend group (was one; handlers are
concurrent now). Read timeout 1 s per read (spec): a client pausing > 1 s mid-body would be cut off — not seen in any
test; unverified risk. After answering, the handler half-closes and drains the client to EOF (lingering close) so a `400`
sent before the body is read is not destroyed by a reset.

**Touches.**
- Callers (`graft-ops/tests/it/remote_ops.rs`, `graft/tests/it/{credential_by_key,m4_walk/*,m5_walk/fixture}.rs`): untouched, compile, green in the workspace run (counts in the tick).
- `docs/capabilities/m4-remotes.md` cites `http_right_password_clones_and_pushes`, `http_wrong_password_is_401_...`, `http_remote_frees_its_port_on_drop` — none renamed; `graft-capabilities` `every_capability_citation_checks_out` green in the workspace run.
- `TESTING.md` (lines 1545, 1703) names `HttpRemote` only, not tiny_http: nothing stale. `tiny_http` remains only in history (`docs/log/m4.md`, `docs/log/m8.md`, `docs/reviews/m4-01-remote-fixtures.md`) — left as history.
- Public API: `serve` signature byte-for-byte; new items are `#[cfg(test)] pub(crate)` (`serve_with_exit_delay`, `open_connections`).
- No keymap, journal, captures or shipped code: test-only crate.

Gates (workspace-wide, final code): fmt ok, clippy -D warnings ok, nextest 5888/5888 (`--test-threads 4`), doctests ok.
