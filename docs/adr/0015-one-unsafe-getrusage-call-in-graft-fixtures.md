# One `unsafe` `getrusage` call, confined to `graft-fixtures::memory`

ADR-0013 concluded with "every other crate keeps `#![forbid(unsafe_code)]`
… the rest of the workspace cannot use `unsafe` at all even by accident".
That is now one crate less true. `graft-fixtures` has
`#![deny(unsafe_code)]` and a single `#[allow(unsafe_code, reason = "…")]`
on `max_rss` in `crates/graft-fixtures/src/memory.rs`.

Decision: **keep the call, deny rather than forbid in `graft-fixtures`
only, allow at the one function, and amend ADR-0013 to say two crates
rather than one.**

## Why it exists at all

PROMPT.md §7.1 does not ask for "a memory measurement"; it names the call:
the heavy tier's gate is "peak RSS under 300 MB, measured with
`getrusage`". There is no `getrusage` in `std`, and the alternatives are
all worse here:

- **Read `/proc/self/status`.** Linux only, and ADR-0005 targets macOS
  too. It would mean two implementations, one of them the `unsafe` one
  anyway.
- **A wrapper crate (`rustix`, `nix`, `sysinfo`, `peak_alloc`).** A
  dependency, for one call, in a crate whose whole point is to have almost
  none. `sysinfo` in particular samples a process table — far more
  machinery, and a different number.
- **A global allocator that counts bytes.** It measures allocations, not
  residency, so it would miss the mapped pages a `mmap`-based git object
  reader brings in — which on a 100k-commit repository is most of what the
  gate is about. It would also have to be installed process-wide by a test
  crate, which is exactly the kind of action-at-a-distance a fixture crate
  must not have.

## The call: `getrusage(RUSAGE_SELF, &mut rusage)`

**What it does.** Reports this process's resource usage, of which one
field is wanted: `ru_maxrss`, the largest resident set the process has
held. That is the number PROMPT.md's 300 MB gate is written against.

**Why it is sound.**

- The out-parameter is a `MaybeUninit<libc::rusage>` created with
  `zeroed()`, not `uninit()`. Every field of `rusage` is an integer or a
  `timeval` of two integers, so all-zeroes is a valid `rusage` — the
  struct is fully initialised before the call and the later
  `assume_init()` is sound whether or not the kernel wrote every field.
- `as_mut_ptr()` yields a pointer to exactly that local, correctly aligned
  and of exactly the type `getrusage` writes. It is not aliased: the local
  is not borrowed elsewhere and does not outlive the call.
- `RUSAGE_SELF` and `rusage` both come from `libc`, so the constant and
  the layout match the platform they were compiled for. macOS and Linux
  only, deliberately (ADR-0005).
- A failure is an ordinary `-1` return, turned into
  `MemoryError::Getrusage(io::Error::last_os_error())`, not undefined
  behaviour. The only documented failures are an invalid `who` or a bad
  pointer, neither reachable here.
- The reading is not trusted blindly afterwards: a negative `ru_maxrss`
  becomes `MemoryError::Negative` rather than a clamp, because a clamp
  would turn a broken reading into a passing budget test.

**What would break it.** Switching `zeroed()` to `uninit()` and reading a
field the kernel did not write, or reusing the pattern for a call whose
out-parameter is not a `rusage`. Both are visible in the same five lines,
and a second `unsafe` anywhere in the crate still fails the build.

## The unit trap, which is not an `unsafe` problem but is the real one

`ru_maxrss` is **bytes on Darwin and kibibytes on Linux**. On a 300 MB
gate that is a factor of 1024 — a gate that never fires on one platform
and always fires on the other. The module states the unit per platform in
a named constant and `compile_error!`s on any third platform rather than
guessing. A new target adds its unit deliberately.

The second trap: `ru_maxrss` is a **process-wide high-water mark that
never falls**. A budget therefore asserts on the absolute peak, in a test
binary where nothing large ran first; the `rise` helper is for reporting
only. This is written at the top of the module because it is the mistake
a future budget test will otherwise make.

## Why `deny` and one site allowance, not `forbid`

The same argument as ADR-0013: `forbid` cannot be overridden, so the
choice is a crate with no `unsafe` at all or a crate whose `unsafe` is
deliberate and annotated. The allowance sits on `max_rss`, a function that
does nothing but the call, so any other `unsafe` in `graft-fixtures` —
including elsewhere in `memory.rs` — is still a compile error.

## Amendment to ADR-0013

ADR-0013's consequence "`graft-term` is the only crate that can ever
contain `unsafe`" is superseded. The rule is now: **`graft-term` and
`graft-fixtures`, at the three sites those two ADRs name, and nowhere
else.** Every other crate keeps `#![forbid(unsafe_code)]`, and a fourth
site needs its own ADR before it is written.

The blast radius of this one is smaller than ADR-0013's, not larger:
`graft-fixtures` is `publish = false` and a dev-dependency everywhere it
is used (ARCHITECTURE.md, ADR-0006), so this `unsafe` is never linked into
anything a user runs.

## Consequences

- Reviewing Graft for `unsafe` means reviewing two files:
  `graft-term/src/detect/tty.rs` and `graft-fixtures/src/memory.rs`.
- Each site carries a `SAFETY:` comment and a `reason` on the allowance,
  both pointing at the ADR that argues it.
- The workspace `Cargo.toml` comment about `forbid` names both exceptions,
  so the next person does not have to find this file to learn there are
  two.
