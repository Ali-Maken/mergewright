# Graft: Architecture

A one-page map of the crates: what each one owns, what it may depend on, and
where to start reading. Read this before adding a module or moving code
between crates — and update it in the same ticket if that ticket changes the
crate map. The full reasoning behind every choice here is
[`DESIGN.md` §8](DESIGN.md#8-architecture); this file is the short version.

"Graft" is a codename (see `CLAUDE.md`). Domain vocabulary — lane, default
branch, rendering tier, Ghost Preview, Mistake Radar, marker, ref badge,
operation, undo journal, Command Log, job — is fixed in `CONTEXT.md`; this
file uses those terms rather than redefining them.

## The shape

A pure core, a thin shell (PROMPT.md §6.1). The parts that *decide* — graph
layout, diff/porcelain parsing, Mistake Radar, undo planning, `update` — are
plain functions: data in, data out, no terminal, no filesystem, no clock, no
threads. Only the outer layer touches any of those. Data flows one way:

```
Event ──▶ update(state) ──▶ (new state, [Command]) ──▶ view(state)
```

`update` never does I/O; it returns `Command`s. The job runner executes each
`Command` off the UI thread as a cancellable `Job` and reports the result back
as an `Event`. The UI thread only ever waits on the next `Event` — never on a
`Job` — which is why input stays responsive during a two-minute fetch.

## Where to start reading

- **Changing how history is drawn** (lanes, compaction, a new rendering tier)
  → `graph`. Start at `DESIGN.md` §3 for the algorithm.
- **Changing what Graft reads from a repo** (status, log, diff parsing) →
  `git`. Reads are `gix`-primary; `git2` backs single named gaps only
  (ADR-0011), never a second full adapter.
- **Changing what a write operation does, or how undo works** → `ops`. Start
  at `DESIGN.md` §6.
- **Changing terminal detection or an image protocol** → `term`. Start at
  `DESIGN.md` §4, then `graft-term/src/detect/`: `environment` reads the env
  vars, `probe` writes the queries, `reply` parses what comes back, `tmux`
  owns the passthrough round, and `mod` is the decision that turns all of it
  into a `Capabilities`. `detect::tty` is the only module that touches the
  real terminal, so everything above it is tested against scripted replies.
- **Changing what counts as hostile repository text** (a new escape or
  invisible-character class) → `graft-term/src/sanitize/`. One function and
  a small set of entry points; `DESIGN.md` §8.2 sets the minimum classes and
  the replacement policy, and the module doc is the authority on what is
  actually neutralised (it takes all of Unicode's `Cf` category, which is a
  superset).
- **Changing a screen or a keybinding** → `ui` for the widget, `app` for the
  key-to-`Command` mapping. Start at `DESIGN.md` §1–§2.
- **Adding a new kind of `Command` or wiring a new job** → `app`. It's the
  only crate allowed to see every other crate, and `update` and the job
  runner both live there.

## The crates

Every package is named with a `graft-` prefix — `graft-core`, `graft-graph`,
`graft-git`, `graft-ops`, `graft-term`, `graft-ui` — because a bare package
called `core` collides with Rust's built-in `core`. `app` is the binary, and
its package is simply `graft`. This page uses the short names throughout; the
prefix exists only in `Cargo.toml` and in `use` paths (`graft_core::…`).

`app` is a library (`crates/graft/src/lib.rs`) with a thin binary
(`main.rs`) on top of it. Everything lives in the library so that `app`'s own
tests, its integration tests and the manual `terminal_probe` example all drive
the same code the binary runs; `main.rs` only starts it. This is a target
split inside one package, not a new crate: the dependency table below is
unchanged.

| Crate | Owns | May depend on |
|---|---|---|
| `core` | Plain data only — no behaviour, no algorithms: commit and ref identity, events, the record shapes other crates read and write, errors for its own parsing | nothing internal |
| `graph` | The pure layout engine and the pixel/Unicode/ASCII renderers | `core` |
| `git` | Repository reads (`gix`; `git2` only per named gap, ADR-0011), the typed git command runner, output and diff parsers | `core` |
| `ops` | Operations, the undo journal, Mistake Radar detection, Ghost Preview planning | `git`, `core` |
| `term` | Terminal capability detection, image-protocol encoding, escape-sequence sanitising | `core` |
| `ui` | Widgets, screen layout, theme, view models | `graph`, `term`, `core` |
| `app` | The binary: event loop, `AppState`, `update`, the job runner, config, wiring | everything |

As layers — each row may depend only on row(s) above it:

```
core
graph   git   term        (each: + core)
        ops                (+ git, core)
ui                          (+ graph, term, core)
app                          (+ everything above)
```

`graph`, `git` and `term` are siblings and may not depend on each other. `ui`
never imports `git` or `ops`: every screen renders from a plain view model
`app` assembles, not from a live repository handle. Repo-derived strings
(commit messages, author names, file names, file contents, ref names) cross
into `ui` only as `term::SanitizedText` — a newtype only `term::sanitize`
can construct, applied by `app` at the view-model boundary (`DESIGN.md`
§8.2), so escape-sequence injection is a compile error, not a review catch.

## Shared types worth knowing where they live

`core` holds plain data only — no behaviour, no algorithms. Laying out a
graph, parsing `git` output, detecting a Mistake Radar pattern, replaying a
journal entry, probing a terminal: all of that lives in the crate that owns
the behaviour, never in `core` (ADR-0007). A few types read naturally as
part of one crate's algorithm but, checked against the table above, can
only compile in `core` — because two crates that may not depend on each
other both need the same shape. The full list and reasoning is `DESIGN.md`
§8.2; the ones that come up most:

- `Commit`, `Oid`, `CommitId`, `RefName` — produced by `git`, consumed by
  `graph`, `ops` and `ui`.
- `MistakeRadarKind` — produced by `ops::MistakeRadar`, attached to a
  `graph::Row` as plain input data (`graph` never calls into `ops`), rendered
  by `ui`.
- `Capabilities`, `RenderingTier` — detected by `term`, consumed by `graph`'s
  renderer choice and `ui`'s diagnostics.
- `RasterFrame` — `graph`'s pixel renderer output; `term` encodes it into
  Kitty/iTerm2/Sixel bytes. Drawing and protocol-encoding are deliberately two
  different crates.
- `JournalEntry`, `OperationKind` — written by `ops::UndoJournal`, read by
  `app` to build the undo journal browser's view model for `ui`.

`Command` is the one exception: it is **not** a `core` type, even though it
carries `Event`s' name in PROMPT.md §6.1. A command like "run this fetch" has
to name a `git::GitInvocation`; `core` can't depend on `git`. `Command` lives
in `app`, the only crate that can see what it needs to name.

## Dependency rule and enforcement

Cargo rejects cycles, which stops the graph above from ever growing a loop.
It does **not** stop a new acyclic edge that breaks the intended layering —
`ui` adding `git` as a dependency compiles fine and violates the rule anyway.
A fast-tier test closes that gap: it parses `cargo metadata` and asserts the
edge set above matches exactly, so a stray dependency fails the fast tier
instead of waiting for review (`DESIGN.md` §8.5, ADR-0006).

## Keeping crates deep

Every crate here should be checkable against one question: can a caller get
everything they need through the handful of functions it exposes, without
reaching into its internals? `graph::layout` is the model case — one call
replaces lane assignment, compaction and incremental checkpointing. Where a
crate needs more than one concrete implementation of something (`term`'s
per-protocol encoders, or a `git` read that a named `gix` gap forces onto
`git2` — ADR-0011), that split is an *internal* seam, private to the crate —
not a reason to widen its public interface. Default to `pub(crate)`; a type
becomes `pub` only when another crate in the table actually needs it.

## Elsewhere

- `DESIGN.md` — the full design: view mockups, keymap, the graph layout
  algorithm, terminal-capability detection, the undo journal, this
  architecture in more detail, and proposed deviations from PROMPT.md.
- `CONTEXT.md` — the glossary. Use its terms, not synonyms.
- `docs/adr/` — decision records, one per significant choice.
- `KNOWN_GAPS.md` — everything unfinished, stubbed or deferred.
