# Graft

A terminal Git client whose centrepiece is a real graphical commit graph. This
glossary fixes the words used across `PROMPT.md`, `DESIGN.md`, the ADRs and the
code. Use these terms everywhere; prefer them over the `_Avoid_` alternatives.

## These are our words, not the user's

**Every entry below is internal by default.** It fixes what *we* call a thing
in specs, ADRs, design docs, module names, type names, test names and commit
messages, so that two documents written a milestone apart mean the same thing.

**None of it is licence to put the word on screen.** A term earns a place in
the interface only when the interface *defines* it there — by saying it plainly
in the same breath, or by a glyph the user has already been taught. If the user
has to connect a symbol to a noun the UI never explains, the word has failed,
however correct it is in here.

The test, in one line: **would a Git user who has never read this file know
what it means the first time they see it?** No → say the outcome instead.
`Ghost Preview` is the worked example. It is the right word in `DESIGN.md` and
the right module name; on screen it became `topic e7e39a7 → ghost`, where the
user saw `○` and `—` and had to guess at a noun nothing defined. The fix was
not a better word — it was `→ 5 new commits`, which names the outcome and needs
no glossary at all.

This cuts both ways. A word that *is* defined on screen should still be the
glossary's word and not a synonym: the user-facing text and the code should
agree wherever the user-facing text speaks at all.

A term that is deliberately never user-facing says so in its own entry.

> "Graft" is a codename. The real name is chosen in M8.

## History and shape

**Default branch**:
The branch that is a repo's main line of history: the one everything else grows
out of and merges back into. Usually `main`; older repos use `master`. Named
after Git's own `init.defaultBranch` and the `origin/HEAD` a remote publishes.
_Avoid_: trunk, mainline, base branch, master (as a generic term)

**Integration branch**:
A long-lived branch that collects work but is not the **default branch**, such
as `develop` or a `release/*` line. It never occupies lane 0.
_Avoid_: trunk, secondary trunk, develop (as a generic term)

**Lane**:
A vertical column in the graph occupied by one branch's first-parent chain for
the whole of that chain's life. The **default branch** holds lane 0 and is
always straight.
_Avoid_: column, track, rail, swimlane

**Rendering tier**:
Which of the three drawing techniques the graph is using: **pixel** (real
graphics via a terminal image protocol), **unicode** (box-drawing glyphs) or
**ascii**. Chosen by capability detection, overridable in config.
_Avoid_: mode, backend, fallback level

**Ghost Preview**:
The predicted result of a history-changing operation, drawn into the graph as
dimmed or dashed nodes before the user confirms it.
**Internal only — never drawn on screen.** The user is shown the outcome
(`after rebase`, `→ 5 new commits`), never the word "ghost".
_Avoid_: dry run, preview mode, simulation

**Mistake Radar**:
The feature that detects suspicious history patterns (foxtrot merge, backward
merge, evil merge, accidental pull merge, duplicate commits, force-pushed
remote, risky state) and marks them in the graph.
_Avoid_: linter, warnings, diagnostics

**Marker**:
The small indicator Mistake Radar places on a graph row. Each marker offers
**Explain** (plain-English account) and **Fix…** (safe remedies).
_Avoid_: badge, flag, annotation

**Ref badge**:
The label drawn on a graph row for a local branch, remote branch, tag, `HEAD`
or stash. Distinct from a Mistake Radar **marker**.
_Avoid_: chip, tag, label

## Operations and safety

**Operation**:
One user-initiated unit of Git work that Graft can record and undo as a whole,
such as a commit, merge, rebase, reset, discard or stash drop.
_Avoid_: action, command, task

**Undo journal**:
The durable record of ref states and working-tree snapshots taken before each
operation, making every operation reversible. Browsable; each entry revertable.
_Avoid_: history, undo stack, operation log

**Command Log**:
The visible, append-only list of the exact `git` commands Graft ran. A
transparency feature for the user; not the undo mechanism.
_Avoid_: console, output pane, terminal log

**Job**:
A unit of Git work running off the UI thread, cancellable, reporting back as an
event. The UI thread never waits on one.
_Avoid_: task, worker, async op

## Testing

**Fast tier**:
The default test suite, run after every change and the gate for every ticket.
Small fixtures only, no network, no fuzzing, no benchmarks.

**Heavy tier**:
Tests run only at milestone reviews: the 100k-commit synthetic repo, the
performance and peak-memory budgets, and wider property testing.

**Manual tier**:
Checks only a human runs, from `TESTING.md`, at the M1 and M8 checkpoints.

**Done (file)**:
A conflicted file he has finished: every conflict taken and the file staged, no
longer unmerged; shown `done ✓`.
_Avoid_: resolved

**Fixture repo**:
A synthetic repository generated by script for tests, exercising one history
shape (foxtrot merge, criss-cross merge, orphan branch, non-UTF-8 names, …).
_Avoid_: test repo, sample repo
