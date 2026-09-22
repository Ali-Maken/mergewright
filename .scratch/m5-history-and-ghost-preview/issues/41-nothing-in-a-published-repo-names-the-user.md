# nothing-in-a-published-repo-names-the-user

Status: ready
Blocked by: nothing (branch off `main` at `e1acf45` or later).
Crates: `graft`, `graft-fixtures`; plus `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py` and `docs/`.
Build with: **Opus.** Ticket 40 closed half this class and the half it left is
the half that leaks. Getting the seam right matters more than the sweep.
Scout: **not needed** — every count below was measured by the conductor on
`e1acf45`, with the command that measured it.

## Why this exists

CLAUDE.md, Gates: **"Raw terminal streams never committed. The repo may be
published."** It is not published yet, which is the only reason this is a
ticket and not an incident.

```
git ls-files | xargs grep -l "/Users/devuser" | wc -l   ->  124
```

**124 tracked files carry the user's real home path** — 114 under
`docs/shots/`, 4 under `docs/perf/`, 6 elsewhere. They embed his name and his
machine's layout. Not a secret; still not ours to publish. Ticket 40 also
counted **47 files with `/var/folders`** and **5 with ephemeral ports**.

Ticket 40 made every **Rust-generated** capture assert on each run and
normalised the paths those go through. It did not reach the **script-written**
captures: `pty-walk-m5.py` writes its own, and those are **neither asserted nor
normalised**. So the class is closed on one side and open on the other, and the
open side is the leaking one.

## Piece 1 — normalise at the seam, not per file

Find where a real path can enter a captured frame and stop it there, the way
`captures::frame_at` already does for `repository.path`
(`crates/graft-fixtures/src/capture.rs`, added by ticket 40 — **read it
first and extend it rather than writing a second convention**).

Three classes to normalise, all to stable placeholders that **preserve column
width** so no grid shifts:

- the home directory (`/Users/<name>/…`)
- the OS scratch directory (`/var/folders/…`, `/tmp/…`)
- ephemeral ports

**Do not** do this with a `sed` pass over `docs/`. A sweep fixes the 124 files
that exist and none of the ones written next week; ticket 40's whole lesson is
that the instrument matters more than the sweep. Fix the writers, then
regenerate.

## Piece 2 — bring the script-written captures under ticket 40's mechanism

`pty-walk-m5.py`'s captures should be asserted the same way: compared on a
normal run, rewritten only under the explicit bless, and **a regeneration stays
a red run** so a changed capture cannot reach a commit unlooked-at. That last
property is ticket 40's, it is the reason this class will not reopen, and it
must hold here too.

If a pty capture genuinely cannot be asserted — a timing-dependent frame, say —
**name it and say what makes it unassertable**, and make sure it is still
normalised. An honest exception is fine. An unexamined one is how both halves
of this got here.

## Piece 3 — a test that fails if the leak comes back

One test, cheap, run on every build: **no tracked file contains a home
directory, an OS scratch path or a port.** It is the only thing that keeps
piece 1 true after everyone has forgotten this ticket.

Scope it to what is actually forbidden, not to the literal string
`/Users/devuser` — a test that only knows this user's name passes on
someone else's machine while still shipping their path.

## Piece 4 — regenerate, then check the sheet

Regenerate everything and rebuild the M5 contact sheet
(`scripts/contact-sheet.sh m5`). Per-capture cap **200 KB** stands; the sheet
is **exempt** (ADR-0037 §2 — it is currently 281 KB and that is allowed).
**Raw terminal streams are never committed.**

**Do not read captures back into your context** — sizes, `head -3` and
`git diff --stat` only.

## NOT yours

- The **Q1 dead-keys defect** (bar draws 9 keys behind an overlay, none
  answered) — that is ticket 42.
- The acceptance walk (37, 38, 39).
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's. Report rows.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] `git ls-files | xargs grep -l "/Users/"` returns **0** — give the number
      before and after.
- [ ] No tracked file carries an OS scratch path or an ephemeral port — give
      both counts before and after.
- [ ] Normalisation happens at the **writer**, not by a sweep over `docs/` —
      name the function(s), and show that a capture written fresh on a
      different path comes out identical.
- [ ] Script-written captures assert on a normal run, and a regeneration is
      **still a red run** — test: name it, and prove it by breaking one
      capture and quoting the failure.
- [ ] Any capture left unassertable is listed with the reason — give the list,
      or say "none".
- [ ] A test fails if a home path, scratch path or port reappears in a tracked
      file — test: name it, and say how you checked it actually bites.
- [ ] Contact sheet rebuilt; `grep -c 'NOT RECORDED'` is 0 — give the number
      and the sheet's size.
- [ ] Whole suite green; report before and after. Baseline **2415 passed, 1
      skipped**, and the 1 is a timing tool — if that number changes, say why.

Rule 6: you do not tick "looks right". The user does.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.**

`CARGO_TARGET_DIR=/tmp/graft-target-g41`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

Counts before and after for all three classes. Whether the pty captures could
join ticket 40's mechanism or needed their own. What is still non-deterministic.
Anything unverified — which is not broken.
