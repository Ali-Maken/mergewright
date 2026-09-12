# Two `unsafe` libc calls, confined to `graft-term::detect::tty`

PROMPT.md §6.2 requires `#![forbid(unsafe_code)]`. Every crate in the
workspace has it except `graft-term`, which has `#![deny(unsafe_code)]` and
two `#[allow(unsafe_code, reason = "…")]` sites in
`crates/graft-term/src/detect/tty.rs`. This ADR is the justification the M0
review asked for: what each call does, why it is sound, what would break
it, and why the allowance is scoped the way it is.

Decision: **keep the two calls, deny rather than forbid in `graft-term`
only, and allow at the two function sites, not the crate.**

## Why they exist at all

Both answer a question the standard library does not: "how many pixels is
this window?" and "is there anything to read on this fd yet?". Neither has
a safe wrapper in `std`, and the alternatives are worse:

- **`nix`, `rustix` or another wrapper crate.** They would move the same
  two syscalls behind someone else's `unsafe`, add a dependency for two
  calls, and make the soundness argument harder to read, not easier — it
  would live in a crate nobody reviewing Graft reads.
- **`crossterm::terminal::window_size`.** `crossterm` is an `app`-layer
  dependency; `graft-term` sits two layers below it and may not depend on
  it (ARCHITECTURE.md). It also cannot answer the `poll` question: the
  probe must own stdin while it reads replies, which is exactly what
  crossterm's event reader cannot share (ADR-0012, "two readers on fd 0").
- **Blocking reads with a timeout thread.** A thread per probe read, to
  avoid one `poll`, with a cancellation problem of its own.

Two calls, both in the one module that already exists to hold the I/O, is
the smaller cost.

## Call 1: `ioctl(fd, TIOCGWINSZ, &mut winsize)` — `window_pixels`

**What it does.** Asks the terminal driver for the window's size in both
cells and pixels, so the pixel tiers can work out a cell size from a real
measurement instead of an assumption (`CellSizeSource::WindowSize`). The
fd is this process's own stdout.

**Why it is sound.**

- The out-parameter is a live local `libc::winsize`, built field by field
  (`ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0`), not
  `mem::zeroed()`. Every field is a `u16`, so the local is fully
  initialised before the call and would be valid to read even if the
  kernel wrote nothing.
- `&raw mut size` produces a pointer to exactly that local, with exactly
  the type `TIOCGWINSZ` writes, and it is not aliased anywhere — the local
  is not borrowed elsewhere and does not outlive the call.
- The request constant and the struct both come from `libc`, so they match
  the platform they were compiled for. Unix only, and Unix only is a
  deliberate v1 scope (PROMPT.md, Mission; ADR-0005).
- A bad fd, or one that is not a terminal, is an ordinary error return
  (`!= 0`) that the function turns into `None`, not undefined behaviour.

**What would break it.** Passing the request constant with a mismatched
struct type — for example reusing this pattern for `TIOCSWINSZ` or another
`ioctl` whose out-parameter is not a `winsize`. Nothing else about the
call site is fragile: there is no length, no allocation and no lifetime to
get wrong. If a second `ioctl` is ever needed, it gets its own reasoned
allowance and its own entry here, not a widened one.

## Call 2: `poll(&mut pollfd, 1, timeout_ms)` — `wait_for_input`

**What it does.** Waits until stdin has bytes or the deadline passes, so a
terminal that never answers a probe costs Graft a bounded wait instead of
a hang (DESIGN.md §4.4's probe budgets).

**Why it is sound.**

- The pointer is to one live, fully initialised `libc::pollfd` local
  (`fd`, `events: POLLIN`, `revents: 0`), and the count argument is the
  literal `1`, so the array length and the pointee agree by construction —
  the classic `poll` mistake (a count larger than the array) is not
  expressible here without editing both halves of one line.
- `poll` writes only `revents`, a `c_short`, back into that local.
- The fd is this process's stdin, obtained from `io::Stdin` via
  `AsRawFd`, which is open for the whole program. `poll` on a closed or
  invalid fd is defined behaviour anyway: it reports `POLLNVAL`, and a
  negative return becomes `io::Error::last_os_error()`.
- The timeout is an `i32` clamped from a `Duration`, so it cannot be an
  out-of-range value; a negative timeout — the "block forever" case — is
  unreachable because the clamp's floor is zero.

**What would break it.** Watching more than one fd by growing the local
into an array and forgetting to grow the count with it, or handing out the
`pollfd` to be filled in elsewhere so the `events` field is no longer
known at the call site. Both are visible in the same three lines.

## Why `deny` and two site allowances, not `forbid` and none

`forbid` cannot be overridden — a `#[allow]` inside a `forbid`ing crate is
a compile error — so the choice is between a crate with no `unsafe` at
all and a crate whose `unsafe` is deliberate and annotated. `deny` gives
the same protection as `forbid` for every line that does not carry an
explicit, reasoned allowance, and the allowances are on the two functions
rather than on the module or the crate, so a third `unsafe` anywhere in
`graft-term` — including elsewhere in `tty.rs` — still fails the build.

The narrower scoping that was available and rejected: a statement-level
`#[allow(unsafe_code)]` on the `let … = unsafe { … };` itself compiles, so
the language does offer a smaller unit than the function. The function is
the chosen granularity anyway, because the `SAFETY:` argument is about the
whole function — in both cases it is the surrounding lines that establish
the invariant the call relies on (the `pollfd` is built two lines up; the
`winsize` is initialised field by field right before the `ioctl`), and an
allowance scoped tighter than its own justification invites someone to move
the call and leave the argument behind. Both functions are short and do
nothing but set up the call and make it.

Every other crate keeps `#![forbid(unsafe_code)]`, so this is one crate,
one module, two functions, and the rest of the workspace cannot use
`unsafe` at all even by accident.

## Consequences

- `graft-term` is the only crate that can ever contain `unsafe`, and
  reviewing Graft for `unsafe` means reviewing one file.
- Each site carries a `SAFETY:` comment stating the invariant, alongside
  the `reason` on the allowance, and both point here.
- A Windows port (a non-goal for v1) would need its own equivalents; the
  module is already the place they would go.
