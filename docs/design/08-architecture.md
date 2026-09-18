# Graft: Design — Architecture

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [Open design questions](07-open-design-questions.md) · Next: [Proposed changes](09-proposed-changes.md)

---

## 8. Architecture

PROMPT.md §6.1 fixes the crate map and the dependency rule. This section fills
in the part §6.1 leaves open: which crate each *type* used elsewhere in this
document actually lives in, and why. It uses the `codebase-design` skill's
vocabulary — **module**, **interface**, **depth**, **seam**, **adapter** — so
each crate can be checked against the same question: is this a small interface
hiding a lot of implementation, or is it a pass-through? `ARCHITECTURE.md` at
the repo root is the short, standalone version of this section for whoever is
about to add or move a module; this section is where its reasoning lives.

### 8.1 Crate map

| Crate | Owns | May depend on |
|---|---|---|
| `core` | Plain data only — no behaviour, no algorithms: commit and ref identity, events, the record shapes other crates write and read, errors for core's own parsing | nothing internal |
| `graph` | The pure layout engine (§3) and the pixel/Unicode/ASCII renderers | `core` |
| `git` | Repository reads (`gix`), the typed git command runner, output and diff parsers | `core` |
| `ops` | Operations, the undo journal (§6), Mistake Radar detection, Ghost Preview planning | `git`, `core` |
| `term` | Terminal capability detection (§4), image-protocol encoding, escape-sequence sanitising | `core` |
| `ui` | Widgets, screen layout, theme, view models | `graph`, `term`, `core` |
| `app` | The binary: event loop, `AppState`, `update`, the job runner, config, wiring | everything |

As a layering picture rather than a table — each row may only reach into the
row(s) above it:

```
core
graph   git   term        (each: + core)
        ops                (+ git, core)
ui                          (+ graph, term, core)
app                          (+ everything above)
```

`graph`, `git` and `term` are siblings: none of them may depend on each other.
That's what forces any type all three need — see §8.2 — up into `core`.

### 8.2 Key types and where they live

Several types this document already defines (§3.1, §6.1) are written without a
crate label, because they read naturally as part of the algorithm they support
rather than as part of an architecture diagram. Placed against the dependency
rule in §8.1, most of them have exactly one legal home. Nothing below changes
the shapes already defined in §3.1 or §6.1 — it says which crate each one
compiles in and, where that isn't obvious, why.

**`core`** holds plain data types only — no behaviour, no algorithms.
Everything that *does* something with that data — laying out a graph,
parsing `git` output, detecting a Mistake Radar pattern, replaying a journal
entry, probing a terminal — lives in the crate that owns that behaviour
(`graph`, `git`, `ops`, `term`, `ui`), never in `core` itself (ADR-0007).
`core` types exist purely because two crates that may not depend on each
other both need the same shape:

- `Oid` (raw git object id) and `CommitId` (an `Oid` known to name a commit) —
  needed by `git` (produces them), `graph` (lays them out), `ops` (journals
  and scans them) and `ui` (displays them).
- `Commit` (§3.1: id, parents, author, committer, date, message) — `git`
  parses it from `gix`/porcelain output, `graph` lays it out, `ops` scans it
  for Mistake Radar, `ui` renders it. Four crates, none of which may depend on
  each other, so this can only live in `core` even though §3.1 introduces it
  inside the graph-layout discussion.
- `RefName`, `WalkStrategy` (topological / date-order / first-parent-only,
  §3.2) — chosen by `app`/`ui`, executed by `git`'s rev-walk, shapes `graph`'s
  input order. Same reasoning: `git` and `graph` are siblings.
- `MistakeRadarKind` (CONTEXT.md's seven kinds) — `ops::MistakeRadar` produces
  it, `graph::Row.marker` (§3.1) carries it, `ui` renders the marker and its
  Explain text. Because `graph` may not depend on `ops`, **`graph::layout`
  takes a `Map<CommitId, MistakeRadarKind>` as a plain input parameter and
  never computes it** — `app` runs `ops::MistakeRadar::scan` and `graph::layout`
  separately and passes one into the other. `graph` stays ignorant of `ops`
  the same way it's ignorant of the terminal.
- `RenderingTier`, `Capabilities` (§4) — `term::detect` is the one impure
  function that produces a `Capabilities` (it's the only crate that queries
  the terminal); `graph`'s renderers and `ui`'s diagnostics view both consume
  the result as data, and both are `term`'s siblings or dependents, so the
  type itself has to sit below `term`, in `core`.
- `RasterFrame` — the pixel renderer's output: a premultiplied-alpha pixel
  buffer plus geometry, produced by `graph`'s pixel renderer (§1, ADR-0003)
  and consumed by `term`'s image encoder, which turns it into Kitty/iTerm2/
  Sixel bytes. `graph` and `term` are siblings, so `RasterFrame` lives in
  `core` for the same reason `Capabilities` does. This is the split between
  *drawing* the graph (pure, in `graph`) and *encoding it for this terminal*
  (protocol-specific, in `term`) that §1 and §4 don't spell out separately.
- `JournalEntry`, `OperationKind` (§6.1) — §6.1 says the undo journal "lives
  in the `ops` module," which is true for the journal's *behaviour* (append,
  replay, compute the inverse — all in `ops::UndoJournal`). But the undo
  journal browser (§1.10) is a `ui` view, and `ui` may not depend on `ops`, so
  the record *shape* itself — `JournalEntry` and `OperationKind` — has to be a
  `core` type: `ops` writes it, `app` reads it back to build the browser's
  view model, `ui` renders that view model. Same data-vs-behaviour split as
  `Commit`.
- `JobId`, `Event` — `Event` carries terminal input (key, mouse, resize,
  paste, focus, tick) plus job-completion notices (`JobFinished(JobId)`,
  `JobFailed(JobId, core::Error)`). It stays crate-agnostic on purpose: it
  never names a `git`, `ops` or `graph` type directly, which is what lets
  `core` own it without depending on anything. `app`'s job runner looks up
  what a finished `JobId` actually produced.
- Each crate keeps its own `Error` enum (§6.2 of PROMPT.md); `core::Error`
  only covers `core`'s own fallible constructors (parsing an `Oid` from a
  string, and so on) — it is not a shared error type other crates wrap. A
  constructor rejecting a malformed string is enforcing the type's own
  invariant, not an algorithm acting on domain data — the same "no
  behaviour" boundary a plain newtype has in any language, not an exception
  to it.

**`graph`** (+ `core`): `Lane`, `Row`, `LayoutCursor`, `BranchIdentity` (§3.1)
— all layout-engine-internal, only ever touched by `graph` itself and by `ui`
(which already depends on `graph`). The public interface is small on purpose:
roughly `layout(commits, strategy, markers) -> impl Iterator<Item = Row>`, one
function per rendering tier taking a `Row` stream and producing either styled
text cells (Unicode/ASCII) or a `core::RasterFrame` (pixel). Everything about
lane assignment, compaction (§3.5) and incremental checkpoints (§3.6) is
internal — a deep module: one call in, a fully laid-out graph out. `graph`
never reads a repository: the streaming layout pulls commits through
`graph::CommitSource` — `next(limit, stop)` for the frontier, `read(ids)`
for rebuilds — which `app` implements over `git::history::Walk` and
`git::history::read_commit` (m1-10), so `graph` and `git` stay siblings.

**`git`** (+ `core`): `RepoHandle` (an open repository), `GitInvocation` /
`GitArg` (the typed command builder — Technology §"Writing to the repo": never
a joined shell string), `WorktreeStatus`, `DiffHunk`, `WordDiff` (porcelain
and diff parsing output). Reads are **`gix`-primary — there is no second
adapter** (ADR-0011). PROMPT.md's Technology section allows `git2` only as
"an acceptable fallback for anything `gix` lacks," and this design takes
that narrowly: if a specific capability turns out to be missing or broken
in `gix`, that one function gets a `git2`-backed body behind the same
`pub(crate)` reader signature, with the gap named in `KNOWN_GAPS.md` so it
can be retired when `gix` catches up — never a parallel `RepoReader`
implementation of status, rev-walk, diff and blame, whose doubled parsing
and test surface nothing in the product motivates. Writes are a second,
separate path: build a `GitInvocation`, spawn the user's `git`, parse its
output — no shared code with the read side beyond the argument and output
types.

**`ops`** (+ `git`, `core`): `UndoJournal` (records and replays
`core::JournalEntry`, §6), `MistakeRadar::scan(commits) -> Map<CommitId,
MistakeRadarKind>`, `GhostPreview::plan(operation) -> PreviewPlan` (predicted
ref and commit deltas). `ops` never touches `graph`: to draw a Ghost Preview,
`app` takes the `PreviewPlan`, synthesises the predicted commits into the
walk, and runs the result through the same `graph::layout` used for real
history — dimmed/dashed styling is a `ui` rendering concern, not something
`ops` or `graph` need to know about.

**`term`** (+ `core`): `term::detect::detect(&Environment, &mut impl
TerminalHost) -> Capabilities` (§4.1–§4.4, the only impure function in this
list — it's the one that talks to the terminal). A free function in `term`
rather than the `Capabilities::detect` this document first sketched, because
`Capabilities` lives in `core` and `core` holds no behaviour (ADR-0007,
ADR-0014); the budget is not a parameter because it follows from the
environment (§4.4), and `TerminalHost` is the seam the scripted terminals in
`term`'s tests substitute for the real one. Also an image encoder,
`encode(&RasterFrame, Protocol) -> Bytes`. Detection and encoding are two different seams for two different
reasons: detection varies by *terminal*, encoding varies by *protocol*
(Kitty/iTerm2/Sixel) — keeping them separate means adding a protocol doesn't
touch detection and vice versa.

`term` also owns the escape-sequence sanitiser PROMPT.md §6.3 requires:
`term::sanitize(&str) -> SanitizedText`. `SanitizedText` is a newtype with
a private field, and `sanitize` is its **only** constructor — holding one is
proof the cleaning ran. The cleaning replaces with a visible placeholder
(`␛` for controls, `�` otherwise), never silently deletes, so a spoofing
attempt is *seen* rather than papered over. It neutralises:

- all C0/C1 control characters and DEL, `\n` and `\t` aside (see below);
- `ESC` itself — which alone kills every CSI, OSC (OSC 8 hyperlinks
  included), DCS, APC and PM sequence;
- **the whole of Unicode's `Cf` (format) category**, plus U+2028 and U+2029
  (ADR-0014). That category is where the bidi overrides and isolates
  (U+202A–202E, U+2066–2069) and the zero-width characters (U+200B–200F,
  U+2060, U+FEFF) live, and all of them can make what's drawn differ from
  what git acts on — but so can the soft hyphen, the Arabic letter mark, the
  interlinear annotation characters and the U+E0000 tag block, which can
  carry a whole hidden sentence. The rule is the category, so a Unicode
  revision that adds a format character adds it here; the ranges are spelled
  out in `term::sanitize` as of Unicode 16.0 and are refreshed with the
  tables, rather than being five ranges chosen by hand and never revisited.
  U+2028/2029 are not `Cf` and are here because they end a line in anything
  that understands them.

`\n` and `\t` are the one thing the caller chooses, because whether they are
structure or an attack depends on where the text is going (ADR-0014):
`term::sanitize` keeps them for a multi-line pane, `term::sanitize_line`
replaces them for anything drawn in a single row, and
`term::sanitize_path`/`sanitize_path_bytes` are the same for a path — a file
name may legally contain a newline, and a status row that passes one through
scrolls the screen.

The one mandatory application point is `app`'s view-model assembly — the
only place `git`/`ops` output meets `ui` types (§8.3). Every view-model
field carrying repo-derived text is typed `SanitizedText`, so an
unsanitised `String` reaching `ui` is a compile error, not a code-review
catch. All five untrusted categories from PROMPT.md §6.3 cross that
boundary and nowhere else: **commit messages** (subject and body — graph
rows, detail pane, blame), **author names** (author and committer,
mailmap-resolved included — rows, chips, blame), **file names** (status,
diff headers, conflict lists, file tree), **file contents** (diff hunks,
blame lines, conflict panes), and **branch names** (all refs: branches,
tags, remotes, stash titles — badges and sidebar).

**`ui`** (+ `graph`, `term`, `core`): plain view-model structs assembled by
`app` (for example a `GraphViewModel` wrapping a slice of `graph::Row`, or a
`JournalViewModel` wrapping `core::JournalEntry`), plus the ratatui widgets
that render them. `ui` never imports `git` or `ops` — every fact a screen
needs, including undo-journal entries and Mistake Radar explanations, arrives
already resolved into a `core` or `graph` type, and every repo-derived
string arrives as a `term::SanitizedText` (see `term` above).

**`app`** (+ everything): `AppState` (the aggregate of every sub-state),
`update(state: &AppState, event: core::Event) -> (AppState, Vec<Command>)`,
and `Command` itself. `Command` is deliberately **not** a `core` type, even
though PROMPT.md §6.1 lists "events, commands" together under `core`'s
domain: a command such as "run this `git` fetch" or "record this journal
entry" has to carry a `git::GitInvocation` or a `core::JournalEntry`, and
`core` may not depend on `git`. Only `app` sees every crate, so `Command` —
and the job runner that executes each one as a cancellable `core::Job` and
reports a `core::Event::JobFinished` back to `update` — lives there. `Event`
stays in `core`; `Command` lives in `app`. That split is the one place this
section adds something PROMPT.md's phrasing doesn't quite say.

### 8.3 Data flow

```
                 ┌─────────────────────────────────────────┐
                 │                   app                    │
Terminal/job  ──▶│  Event ──▶ update(state) ──▶ (state, [Command])
input             │              │                    │      │
                 │              ▼                    ▼      │
                 │           view(state)         JobRunner   │
                 │              │                    │      │
                 └──────────────┼────────────────────┼──────┘
                                ▼                    ▼
                         ui::render(...)      spawns a core::Job per
                         (ratatui frame)      Command, off the UI
                                              thread, cancellable;
                                              result comes back as
                                              an Event
```

`update` never does I/O (PROMPT.md §6.1) — it only turns an `Event` into a new
`AppState` and a list of `Command`s to hand to the job runner. The UI thread
draws from `state` and otherwise only waits on the next `Event`; it never
waits on a `Job` directly, which is what keeps input responsive during a
two-minute fetch (PROMPT.md, non-negotiable principle 3).

### 8.4 Keeping crates deep

Depth is checked the same way for every crate in §8.1: could a caller do
everything they need through the handful of functions in §8.2, without
reaching past them into the crate's internals? `graph::layout` is the
clearest case — one function call replaces lane assignment, compaction and
checkpointing (§3), which is exactly the leverage a deep module is supposed
to give its one caller (`app`). Where a crate needs more than one concrete
implementation behind a function — `term`'s per-protocol encoders, or a
`git` read that a named `gix` gap forces onto `git2` (ADR-0011) — that
split is an **internal seam**, private to the crate and invisible to
callers, not a reason to expose more surface area. `pub(crate)` is the default everywhere (PROMPT.md §6.1); a type only
becomes `pub` when another crate in §8.1's dependency graph actually needs it.

### 8.5 Dependency rule and enforcement

The rule from PROMPT.md §6.1: dependencies only point downward (§8.1's
picture), and it's a Cargo workspace so a cycle fails to compile. That
catches *cycles*. It does not, by itself, catch a new *acyclic* dependency
that still breaks the layering — `ui` adding `git` as a path dependency
compiles fine (no cycle) but violates "`ui` never imports `git` or `ops`."
PROMPT.md §6.1 says "the compiler rejects cycles, so the structure can't
quietly erode," which is true but incomplete: cycle-rejection alone does
nothing about a direct, acyclic `ui → git` edge, which is exactly the kind
of erosion §6.1 is trying to prevent and which the compiler happily accepts.

Graft therefore enforces the full edge set, not just acyclicity: a fast-tier
test parses `cargo metadata`'s dependency graph and asserts it matches
§8.1's table exactly — no more edges, no fewer. It lives in `app`'s test
suite, since `app` is the only crate that already depends on everything it
needs to inspect, and ships as an M0 ticket alongside the workspace
scaffold. Adding a crate or an allowed edge means updating this test in the
same commit; that one-line tax is what turns "a reviewer might notice" into
"the fast tier fails" (ADR-0006).

---
