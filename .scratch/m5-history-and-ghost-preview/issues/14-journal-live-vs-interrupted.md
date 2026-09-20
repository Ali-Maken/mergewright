# journal-live-vs-interrupted: a merge standing right there is not "interrupted" — seam A of group 09

Status: ready
Blocked by: 09 (merged, `4c000e3`)
Build with: **Opus.** Journal semantics: the wrong version silently closes a
live rebase's entry, and the M3 recovery path must keep working unchanged.
Crates: graft
Scout: **not needed.** Group 09 read this code and wrote the seam below from
it, naming files, types and call sites. Read its handoff instead (§"Seam A" of
`09-app-conflict-interim.md`) — that *is* the scout report, one day old, by the
builder that just shipped the banner this sits on.

## Why this exists

Group 09 stopped at a clean seam with pieces 1–4 merged. This is its piece 5,
unchanged in scope. **Three captures on main are known-wrong until it lands**
and say so inside their own header.

## Read first

- `.scratch/m5-history-and-ghost-preview/issues/09-app-conflict-interim.md`,
  sections "Seam A", "Gotchas for whoever takes these", and "Journal
  semantics: no contradiction with group 07".
- `crates/graft/MAP.md` for `update/journal.rs`, `jobs/undo.rs`,
  `view/journal.rs`.
- PROMPT.md §9 for the launch `Sticky` wording.

## Scope

Nothing changes on disk. A live conflict is a `Pending` line with no outcome —
the same bytes an interrupted entry has — and the difference is decided at
**read** time against git's own state, so older journals need no migration.

1. `jobs::undo::JournalRead` gains `pub live: Vec<EntryId>`, filled from
   `graft_ops::apply::recover_interrupted`'s `Recovered::live`.
2. `AppState::live_entries`, set on each journal read.
3. `update::journal::interrupted_notice` **excludes** live entries. This is
   the fix the captures wait on: the footer stops saying "an operation was
   interrupted" about a merge standing two rows above, and the banner's key
   hints reach the footer instead.
4. `view::journal`: a live entry reads `in progress — c continue  a abort`;
   never `interrupted`, never `← u`.
5. **Launch**: `live` non-empty and `in_progress` `Some` → `banner_focus =
   true` and `Sticky { reason: "<kind> still in progress from a previous run",
   keys: Some("c continue  a abort") }`.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] `relaunch_into_a_live_conflict` — test:
- [ ] `interrupted_entry_without_live_state_is_still_closed` — the M3 recovery
      path is **unchanged**; this is the one that catches the dangerous
      version of this change — test:
- [ ] `journal_row_for_live_entry_says_in_progress` — test:
- [ ] The footer no longer claims "interrupted" about a live operation — test:

**Mutation-check each of the four.** Break the behaviour, name which test goes
red. A tick whose test stays green under the matching mutation is a false tick
(Critical). Report the table. If a mutation convicts two things at once, say
so — do not claim an isolation you did not get.

## Re-capture (this is the point of the ticket)

Re-run `conflict_interim::capture_banner_for_docs_shots -- --ignored` and
**delete the "KNOWN WRONG" paragraph** from all three capture headers. If the
paragraph is still true after your change, the ticket is not done.

Captures: ≤ 200 KB each. Colour ships **unverified** — say so (rule 5).
**Do not tick the visual result**; the user judges by eye (rule 6).

## Gates

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short -- -D
warnings`, `cargo nextest run`, `cargo test --doc` — **workspace-wide, never
`-p`**. `CARGO_TARGET_DIR` **outside the repo** (`/tmp/graft-target-g14`), and
**delete it before you hand back** — abandoned target dirs cost 56 GB in this
milestone. Commit, never push.

## Report

Say which fixes are cause and which are symptom. Say whether you used a scout
and, since this ticket says not to, that this is why. Anything you could not
verify goes in the ticket as unverified — which is not broken.
