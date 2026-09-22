# acceptance-conflicts: the walk proves a real conflict is resolvable without leaving the app — merge, rebase-stops-twice, whole-file shapes, stash pop, base, start over, `$EDITOR` round trip, and `kill -9` mid-rebase then relaunch — with the TESTING.md checklist, the captures, the contact sheet, and the "Times I left" line

Status: ready
Blocked by: **01**, **02**, **03**, **04**, **05**, **06**, **08**.
Build with: Fable — the acceptance ticket (does not count against the Fable build budget): it decides what each scenario must prove from git's own state, and writes the "what right looks like" lines the user judges by.
Crates: graft (tests only: `crates/graft/tests/it/`), docs (`TESTING.md`, `docs/shots/`, `.scratch/m6-conflicts/pty-walk-m6.py`)
**Added by the conductor after group 08 was filed (2026-09-22):** one scenario,
`undo-a-resolution-in-a-stash-pop-conflict` — m5's stash-pop shape; `⏎` from the
pane; `o`, `⏎` → `all conflicts resolved`; `u` → the pane's `CONFLICTS` row is
back, `ls-files -u` lists the path, the file bytes equal the bytes before `⏎`;
`u` → the pop is undone (clean tree, stash still listed); `U` → the conflict is
back, markers on disk and `ls-files -u` non-empty. Truth from git, never from
`AppState`. **What right looks like:** `git ls-files -u` prints three lines for
`p` after the first `u`, none after the second, three again after `U`.
Also: `start-over-brings-the-markers-back` (already below) depends on group 08
for its "after `⏎`" step; its *what right looks like* line gains — `git ls-files
-u` lists the path with the **same stage oids** as before the take.

Needs: every other M6 ticket — `git-conflict-model`, `ops-conflict-operations`, `ui-conflict-resolver`, `app-conflict-resolver`, `app-resolver-whole-files-and-base`, `app-answers-beat-rereads`.

## Read first

- `docs/design-notes/m6-conflicts.md` — Promise (the four end states and the "Times I left" expectation), Decision 2 (b) (how the harness fakes `kill -9`), New UI (the mockups the captures are compared with).
- CLAUDE.md "The seven that pay" 1, 5, 6, 7; "Captures stay small"; ADR-0037 (contact sheet); ADR-0033 ("Times I left" is the only measurement).
- `crates/graft/tests/it/m5_walk/` **MAP** — `crates/graft/MAP.md` §tests::m5_walk (`Step`, `Wait`, `fixture.rs`, `truth.rs`, `every_scenario.rs`, the `--only/--list/--stems/--matrix/--capture` CLI, `no_secret_leaked`); `crates/graft/tests/it/coverage.rs::write_matrix` and `M5_STEMS`. **Reuse, never rebuild**: M6 scenarios are new entries in the same table with the same `Step`s; new `Step`s only where named below.
- TESTING.md "History walk" (lines ~1606–1643; scenarios 24–32 at ~1840–1886 — the shape to copy), "Times I left", "Hint bar".
- `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py` (the pty capture script; copy to `pty-walk-m6.py`, do not extend the m5 one), `scripts/contact-sheet.sh`.
- `crates/graft-fixtures/MAP.md` §Shape (`StoppedRebase`, `StoppedMerge`, the m5 `conflicting`/`twice_conflicting` shapes and the stash-pop conflict shape), §capture (`GRAFT_UPDATE_SHOTS=1`, redaction).
- `docs/field-notes.md` (the eye-check findings the captures must not repeat: caption per width, never captured mid-load, `ghost` never drawn).

## Scope

In:

1. **Harness additions** (`m5_walk/` keeps its name; the table is one table): `Step::Kill` — the harness drops the app without shutdown while git's child is running (byte-identical on disk to SIGKILL; the m5 `Relaunch` step already reopens), asserting in `truth.rs` that `rebase-merge/` or `MERGE_HEAD` exists and the journal's last line is `Pending`; `Wait::ConflictOpened` (resolver overlay present with `file: Some`); `Wait::Resolved(path)` (`ls-files -u` no longer lists it). `truth.rs` gains `resolved_bytes(path)`: the working-tree bytes, compared with `git checkout --ours/--theirs` bytes computed on a scratch clone. `no_secret_leaked` unchanged.
2. **Scenarios** (numbered after m5's 49, `M6_STEMS` in `coverage.rs`), each ending in a truth check from git, never from `AppState`:
   - `merge-resolved-with-the-keys`: `StoppedMerge`, `⏎` on `both.txt`, `o`, `⏎` → resolved; `three.txt`: `o`, `n`, `t`, `n`, `b`, `⏎`; the bytes equal ours / theirs / ours+theirs per block; `c` → merge commit exists, `ls-files -u` empty, journal one entry, `u` reverts the merge.
   - `rebase-stops-twice-resolved-twice`: m5's `twice_conflicting`; resolve, `c`, resolve again, `c`; one journal entry; `u` restores the pre-rebase tip.
   - `every-whole-file-shape`: `StoppedMerge`; each of the seven `UnmergedHow` files resolved through the card's `o` or `t`; the tree afterwards matches a scratch clone resolved with the raw git commands from G2's table.
   - `crlf-and-no-final-newline-survive`: `crlf.txt` `t`, `nofinal.txt` `o` → bytes equal the git-computed sides (line endings intact).
   - `zdiff3-and-base-on-demand`: `zdiff3.txt` shows base without `B`; `three.txt` `B` shows three bases; `both-added.txt` `B` says `no base (added on both sides)`.
   - `start-over-brings-the-markers-back`: `o`, `⏎` (staged), reopen, `d`, `Start over` → `ls-files -u` lists it, `open() == 1`.
   - `editor-edits-are-never-clobbered`: `e` with `$EDITOR` set to a script that rewrites the file; back in the resolver `o` → `changed on disk · re-read` (if the fingerprint was stale) or the take applies to the re-read file; the edit's bytes survive.
   - `mergetool-hand-over-and-back`: `M` with `merge.tool` set to a script that resolves and exits 0 → the resolver closes with `resolved outside`; exit 1 → `mergetool exited 1 for <path>` and the resolver re-reads (closes KNOWN_GAPS 479's "headless mergetool path untested").
   - `stash-pop-conflict-resolves-without-a-banner`: m5's stash-pop shape; `⏎` from the pane; `o`, `⏎` → `all conflicts resolved`, no banner, journal row as G5 says.
   - `kill-9-mid-rebase-then-relaunch`: `twice_conflicting`, first stop, `o` on one block (file half-taken on disk), `Step::Kill`, `Relaunch` → banner focused, pane on `CONFLICTS`, `Sticky` `rebase still in progress from a previous run · ⏎ resolve  c continue  a abort`, `⏎` → the resolver shows the half-taken file (`1 of 2 open`), finish, `c`, `c` → done; one journal entry; **a second variant** `Relaunch` then `a` → tree back to before the rebase.
   - `u-right-after-continue-is-accepted` (G6 piece 2; or the scenario is dropped with G6's proof line).
   - `repush-lease-is-the-entrys-tip` (G6 piece 1): the m5 push shape, remote moved, `u` → `Re-push old tip` → refused by the lease in words.
   - `resolver-is-modal-and-esc-returns`: `u`/`Tab`/`/` swallowed; `Esc` → banner focus; from the pane → pane.
   - `tiny-frame-says-so`: 50×8 → the one line; `Esc` closes.
3. **TESTING.md "Conflicts walk"** section: the command (`cargo nextest run -p graft --test it m6 -- --list` style, per the m5 section), one entry per scenario with **what right looks like** stated concretely (`the result column reads "if is_expired() {" on line 40 and "git show :3:both.txt" gives the same bytes`, never "looks correct"), and the **"Times I left"** table gains its first rows: one per scenario where a reach for the shell would have happened in M5 (`mergetool`, `$EDITOR`, `git checkout --ours`, `git rm`) with the M6 outcome (`0 — resolved in the resolver`), and the residual reaches named as rows with expiry M7 (submodule, rename/rename, `conflict-marker-size`).
4. **Captures** under `docs/shots/m6/` via `pty-walk-m6.py` against the real binary: 120×40 and 80×24 of the resolver on `three.txt` (one open, one took theirs, one open with base shown), the whole-file card, the `2 conflicts still open` refusal, the relaunch `Sticky` (this one taken after `kill -SIGKILL` of the real process, the script asserts the pid died), the `all conflicts resolved · c continue` end state. Each capture's caption is the TESTING.md line; captured after `Wait::Ready`, never mid-load (field-notes). Each ≤ 200 KB; `GRAFT_UPDATE_SHOTS=1` regenerates. Rule 5: colour unverified, said in the sheet's header.
5. **Contact sheet** `docs/shots/m6/contact-sheet.txt` via `scripts/contact-sheet.sh m6-conflicts`, in ticket order.
6. **Context guard**: every capture and truth dump goes to a file; the builder reads summary lines and reports paths only; `--matrix` writes `docs/perf/`-style coverage to `.scratch/m6-conflicts/matrix.txt`.

Out: colour; the heavy tier (fuzz, release perf — the M6 review's); any product change (a failing scenario is filed in the ticket as a finding with its file:line, rule 7, not fixed here).

## Signatures

```rust
// crates/graft/tests/it/m5_walk/step.rs (additions)
pub enum Step { /* existing */ Kill }
pub enum Wait { /* existing */ ConflictOpened, Resolved(&'static str) }
// crates/graft/tests/it/m5_walk/truth.rs (additions)
pub fn resolved_bytes(repo: &Path, path: &str) -> Vec<u8>;
pub fn side_bytes(repo: &Path, path: &str, side: Side) -> Vec<u8>;   // git checkout --ours|--theirs on a scratch clone
pub fn last_journal_line_is_pending(repo: &Path) -> bool;
// crates/graft/tests/it/coverage.rs
pub const M6_STEMS: &[&str];
```

## Acceptance

- [ ] Every scenario above is in the walk table and passes with `cargo nextest run` (fast tier, under 60 s total for M6's rows on a laptop — measured, number in the report) — `every_scenario` covers them; `--list` prints them; `--matrix` shows every M6 stem hit.
- [ ] `kill-9-mid-rebase-then-relaunch` proves from git: `rebase-merge/` present after `Kill`, the half-taken file's bytes on disk before relaunch contain one `<<<<<<<`, after the walk `ls-files -u` empty and `rev-parse HEAD` is the rebased tip — `truth.rs` assertions named in the scenario.
- [ ] TESTING.md "Conflicts walk" has one entry per scenario with a concrete "what right looks like" line, and "Times I left" has its first rows with the M6 count per reach — reviewed by reading the section (the reviewer counts entries vs scenarios).
- [ ] `docs/shots/m6/` has the six captures at both widths, each ≤ 200 KB, redacted (`no_secret_leaked` + the m5 redaction of home/scratch/port), captioned; the contact sheet concatenates them in ticket order; the relaunch capture's script log shows the pid killed with SIGKILL — paths in the report.
- [ ] The builder's report lists what it observed and what it could not (rule 1): mergetool with a real tool is **unverified** unless a script stood in; colour unverified (rule 5); the sheet is for the user's eye (rule 6).
- [ ] Any scenario that fails is a finding in this ticket with file:line (rule 7), and the walk row stays red — no product edit, no skipped test.

## Gates

fmt / clippy / nextest / doc, workspace-wide (tests-only change, still the full set).

## Size

One context; scenarios ~600 lines in the existing table shape, TESTING.md ~150 lines, the pty script copied and trimmed. Captures are read as paths only.
