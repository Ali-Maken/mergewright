# G-words — sixteen lines that say something untrue, unclear or cut

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Sonnet**: every wording is decided below; each item
is a string and a test.
Blocked by: 10, 10b, 10e, 10f, 10h, 14 (10b split 2026-10-07: 10e holds `apply/error.rs`, `apply/remote.rs` — items
4, 12–16; 10f `operate/failure.rs` — item 17; 10h `update/undo.rs` — item 5)
Needs: `keymap-spell-from-the-table` (10), `keymap-spell-in-words` (10b, the grep ratchet) and its word tickets
10e/10f/10h (same user-facing strings; any key a reworded line names stays spelled from the table — 10b's W1, and in
graft-ops `keys: &dyn KeyNames`), `daily-ops-fixes` (put-back words in graft-ops).
Crates graft-ui (detail header, history), graft (view, update words), graft-ops (error words only).

## Read first
- `CONTEXT.md` "These are our words" section (use his words, not ours)
- `crates/graft-ui/MAP.md` `## detail` (`CompareHeader`, `RadarLine`), `## history`; `crates/graft-ops/MAP.md`
  `## apply` (`AfterState`, `ApplyError`), `keys` (`OpKey`, `KeyNames`)
- `10b-keymap-spell-in-words.md` **Ruling** W1–W3 (how a key in a reworded line is spelled; unbound clauses)

## Items (file:line from the open rows; the wording is decided)
1. Compare header never shows `· N files +A −B` (`compare_header` builds SHAs only) → wire `DetailState::files`' totals.
2. `evil merge: clean (N files hand-resolved)` for files not checked (`detail_pane/header.rs:55`) → `evil merge: not
   checked · N files hand-resolved` when unchecked; `clean` only when checked and clean.
3. The both-sides-touched blind spot is told only in Explain → add it to TESTING.md's radar section, one line.
4. Unreadable backup ref reported as `AfterState::Refs` → a `BackupUnread` variant, words `the backup couldn't be read`
   (`apply/error.rs`).
5. Relaunch notice with a set-aside clause and the unreadable-journal warning cuts `J to inspect` at 80
   (`update/undo.rs:645-652`) → the set-aside clause sheds its detail first; keys never cut.
6. A failing smudge filter's refusal doesn't name the file (graft-ops `snapshot/raw.rs`) → name the path.
7. Stash list reads `now ago` (`view/stash.rs:76-83`; the journal's copy guards it, `view/journal.rs:338-341`) → one
   shared age formatter, both call it.
8. `Contained in:` with nothing after it on a stash commit (`detail_pane/header.rs:211`) → line not drawn when empty.
9. History shows `+3` on the list row (traced lines) and `this file +12 −0` in the header → header reads
   `this file +12 −0 · these lines +3`.
10. Chain preview: `4 commits replayed` beside `3 new` and `4 new` (`update/preview/plan_lines.rs:126-135, 152-165`,
    total `:91-101`) → each ref counts only its own commits, so the counts sum to the total.
11. A refused pull or rejected push is a journal row ending `undone` → `refused · nothing written`.
12. `lost_edit::write_back` counts every put-back path (`apply/lost_edit.rs:101-104`) while `write_raw` writes only
    paths in `raw/` (`restore/raw.rs:31-32`); words say "not all written back" when one failed → count what was tried.
13. `ApplyError::Journal.kept` documented as "where a set-aside's edits are" (`apply/error.rs:38-52`); `apply.rs:419`
    puts the raw failure's words there → doc comment matches use.
14. After a raw-step failure the words say "its exact bytes are in the backup" with no route
    (`error.rs:301-305`). Test the route `u` then `J`, row, `⏎`, `y`; if it restores, the words name it
    (`J puts it back`); if not, NOTES says so with file:line and the words drop the claim.
15. At 80 columns the rejected-push reason cuts to `has comm…` (`apply/remote.rs:301`). Check by test whether the next
    step's keys survive the cut; they must (`fit_status_line` keeps keys whole) — fix if not.
16. Rejected-push line `p pull first` sends him to `p`, which answers `diverged · m merge it  r rebase onto it`
    (`apply/remote.rs:239-241`, `in_flight.rs:176-190`). May already be fixed (M7/36): test; if still there →
    `p fetch and choose`.
17. `in_the_way_status` measures `words.reason.width()` on the unsanitised path (`update/operate/failure.rs:195`) →
    measure what is drawn (sanitised).

## Scope
In: 1–17. Out: any wording not listed; key spelling (done by G-keymap: 10, and 10b's split 10e–10i). A reworded line
that names a key keeps it spelled from the table (graft: `state.keymap`, 10b W1; graft-ops: `keys: &dyn KeyNames`,
10e), never a literal; the grep ratchet (`no_literal_keys::no_user_facing_literal_spells_a_default_key`) stays green
— a reworded literal still listed in `tests/no_literal_keys/*` is stale: delete its row.

## Signatures
`AfterState::BackupUnread` (graft-ops, pub). Nothing else public.

## Acceptance
- [ ] One test per item, named at tick (17 ticks). Item 3 is a TESTING.md line, quoted at tick.
- [ ] Captures whose text changes re-shot, listed in NOTES (grep the old strings across `docs/shots/`).

## Touches
- Every capture and test asserting the old strings (grep each); J rows (11); preview line rows (10); MAP.md graft-ops
  (`BackupUnread`), graft-ui; TESTING.md radar section. The remap-test rows for any site reworded here: item 5 →
  10h's `keymap_words::undo_journal`, items 15, 16 → 10e's `keymap_words::ops`, item 17 → 10f's
  `keymap_words::stops` (at least).

## Scout questions
None — rows carry their lines. If a line moved, find it by the quoted string and say so in NOTES.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~55k tokens.

## Conductor addition (from 21 review fix)
- KNOWN_GAPS `M8/askpass-refusal-wording`: `apply/remote.rs:128` files an askpass refusal as `AfterState::AskPass` ("could not be cleaned up"); give a refusal its own words.
