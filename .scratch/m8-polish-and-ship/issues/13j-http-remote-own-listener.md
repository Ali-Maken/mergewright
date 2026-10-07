# 13j — HttpRemote owns its listener (cause fix for 13i)

Status: ready (conductor 2026-10-07) — was draft
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

- [ ] **Deterministic red/green**: `remote::tests::http_remote_drop_waits_for_the_listener_to_close` — via
      `serve_with_exit_delay(300 ms)`: `drop` takes ≥ 300 ms and a connect straight after it is refused. **Red** shown by
      temporarily removing the listener join (mutation): paste the failing assertion into NOTES, then restore. Green after.
- [ ] `remote::tests::http_remote_frees_its_port_on_drop` passes, unchanged in name and assertion.
- [ ] `remote::tests::http_remote_drop_ends_a_wedged_backend` (`tests.rs:395`) passes, unchanged in name and assertion.
- [ ] `remote::tests::http_remote_reads_a_chunked_body` — raw `TcpStream` POST with a chunked body reaches the backend
      (fetch/push outcome or `requests()` count; name what it asserts).
- [ ] `remote::tests::http_remote_answers_expect_100_continue` — raw client sends `Expect: 100-continue`, reads
      `100 Continue` before sending the body.
- [ ] `remote::tests::http_remote_drop_ends_an_idle_client` — a client connects and sends nothing; `drop` returns within
      2 s; port refused after.
- [ ] Every existing HttpRemote user green in the workspace run: graft-fixtures `remote::tests`, `graft-ops`
      `it::remote_ops`, `graft` `it::credential_by_key`, `it::m4_walk`, `it::m5_walk`. Name each in the tick.
- [ ] Load loop: `http_remote_frees_its_port_on_drop` 200× under `taskpolicy -b` with another workspace build running:
      0 failures. Record command and count in NOTES (output to a file; report the summary line only).
- [ ] `cargo tree -i tiny_http` empty; no `unsafe` added (`grep -n unsafe crates/graft-fixtures/src` shows only
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
