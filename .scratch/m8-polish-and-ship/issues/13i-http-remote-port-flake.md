# 13i — `http_remote_frees_its_port_on_drop` flake (KNOWN_GAPS M8/http-remote-port-flaky)

Debugger. No scout: one fixture file plus the dependency it wraps.

**Status: cause found and reproduced deterministically; not fixed.** The fix replaces or patches a dependency, which is a
design and dependency decision (rule 2: stop and ask). Options below.

## Cause

`HttpRemote::drop` returns before the listening socket is closed, because the socket belongs to a thread it cannot join.

- `crates/graft-fixtures/src/remote/http.rs:287-288` says "`Server::drop` closes the listening socket, so the port is
  free once `drop` returns". That claim is false.
- tiny_http 0.12.0 (the latest release), `src/lib.rs:288`: the `TcpListener` is moved into a detached
  `thread::spawn` accept thread. tiny_http keeps no `JoinHandle` for it.
- tiny_http `src/lib.rs:421-444`, `Server::drop`: sets `close`, makes one self-connect to wake `accept`, and returns.
  The accept thread then accepts the self-connect, hands it to its pool, sees `close` and exits. The listener's fd is
  closed only when that thread exits.
- So whether the port refuses at `tests.rs:313` depends on the accept thread being scheduled first. When the machine is
  idle, it is. In a loaded `nextest` run (17b: 5861/5862) it sometimes is not. The test then connects into the
  still-open listener's backlog, and the connect succeeds.

## Reproduction (throwaway, not committed)

I could not make the race show up under plain CPU load. A loop of 200 serve/drop/connect cycles gave 0/200 failures with
no load and 0/200 with 24 busy threads in the same process on 10 cores.

A deterministic repro does work: stall tiny_http's own threads. An `#[ignore]` test in `remote/tests.rs` lists the
process's Mach threads before `serve`, again after it, and `thread_suspend`s the new ones, except the fixture's own
`http-remote-<port>` listener. That is 5 or 6 threads: the accept thread and the pool. It then drops the remote and
connects. A control run without the stall runs alongside it. Results, 3 runs out of 3:

```
[DEBUG-port] stall=false stalled_threads=0 closed_after_drop=true  closed_200ms_after_resume=true
[DEBUG-port] stall=true  stalled_threads=5 closed_after_drop=false closed_200ms_after_resume=true
```

So the port is still open when `drop` returns, and it closes once the threads run again. This is a delay, not a leak.

Why the repro was not committed: it needs `unsafe` Mach calls (`task_threads`, `thread_suspend`, `thread_resume`), and
ADR-0015 allows this crate exactly one `unsafe` site (`getrusage`). Instead, the fix's regression check is the existing
test. Once the listener is owned by a thread that `drop` joins, the existing test is deterministic by construction,
as it is for `HungRemote`.

## Ruled out (by reasoning, not by test)

- **Another process reusing the port, or a TCP self-connect.** On this machine `net.inet.tcp.randomize_ports=0`, so
  macOS hands out ephemeral ports in order. For the freed port to come round again inside one test, about 16k ports
  would have to be allocated in that window.
- **The listener fd leaking into a concurrent `fork`.** On macOS, std sets CLOEXEC on a new socket in a separate call
  after creating it, so a concurrent spawn could inherit the fd. But nextest runs each test in its own process, and this
  test spawns nothing after `bind`.

## Same latent flake

- `tests.rs:395` (`http_remote_drop_ends_a_wedged_backend`) asserts the port is closed straight after drop, so it
  depends on the same race.
- `tests.rs:435` and `:485` are `HungRemote` tests and are not affected. `HungRemote` owns its listener and joins
  its thread (`hung.rs:130-155`).

## Fix options (decision needed)

1. **Own the listener (cause fix, recommended).** Replace tiny_http with an accept loop on the fixture's own
   `TcpListener`, in the listener thread that `drop` already joins, as `HungRemote` does.
   - The HTTP/1.1 it needs is small: request line, headers, a `Content-Length` or chunked body, `Expect: 100-continue`,
     and `Connection: close` on every response. About 150 lines.
   - It drops 4 crates from the lock (tiny_http, ascii, chunked_transfer, httpdate).
   - It deviates from `docs/design-notes/m4-remotes.md:35`, which names tiny_http.
   - Regression risk falls on every HttpRemote user: graft-fixtures remote tests, `graft-ops` `remote_ops.rs`, and
     `graft` `credential_by_key.rs`, `m4_walk` and `m5_walk`.
2. **Patch tiny_http (cause fix, at the dependency).** Vendor 0.12.0 under `[patch.crates-io]`, keep the accept
   thread's `JoinHandle`, and join it in `Server::drop`. About 10 changed lines, but it means maintaining a vendored
   fork in a repo that may be published.
3. **Symptom fix.** After the drop, wait (bounded) for the port to refuse connections, either in `HttpRemote::drop` or
   in the test. Cheap, but it is a wait standing in for ownership. Not done without an OK.

## Touches (if fixed)

- `crates/graft-fixtures/src/remote/http.rs`.
- `Cargo.toml` and `Cargo.lock` (options 1 and 2).
- The callers found by `grep -rn HttpRemote crates`: `graft-ops/tests/it/remote_ops.rs`,
  `graft/tests/it/credential_by_key.rs`, `graft/tests/it/m4_walk/{fixture,truth}.rs` and
  `graft/tests/it/m5_walk/fixture.rs`.
- `docs/design-notes/m4-remotes.md:35` (option 1).
- No keymap, journal or captures are touched; this is a test-only crate.

Nothing changed in source by this ticket.
