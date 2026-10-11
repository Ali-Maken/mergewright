# 45 — the two Majors from the M8 usability walk

Status: done
Build with: Opus (conductor dispatch).
Source: `docs/reviews/m8-usability.md` Majors A and B.

## A. Launch lands on the stash row, not `HEAD`, whenever a stash is newer than `HEAD`

- **Observed (review):** world with a stash newer than `HEAD` → cursor on the stash row at launch.
- **Cause (review, verified at c7aa1b52):** `crates/graft/src/update/facts.rs:30` `banner::refs_answered` starts the
  seek to `HEAD` (`banner/stop.rs:375` `history.seek`, which sets `jump`/`seeking` at `history/scroll/query.rs:96-97`);
  then `facts.rs:38` `stash_entries_read` → `update/tree_answers.rs:193-194` `history.change_view` →
  `history/scroll/apply.rs:239` `self.jump = None`. The Find request's answer is then dropped (`asked` moved on), and
  the view change's window lands the cursor by `reanchor` on row 0, the stash. Reproduced on a pty before the fix
  (screen: `▸ ■ [stash@{0}] On main: parked work`).
- **Fix (cause):** `change_view` no longer drops a jump still out: a pending seek is asked again in the new view
  (`History::find`, factored out of `seek`, never looks in the old view's rows), a pending `G` re-asks `Last` — the
  same rule `promote` already follows (27c).

## B. `?` in a rebase conflict labels `o`/`t` "Take ours"/"Take theirs" while the bar says `o upstream  t yours`

- **Line (review):** `crates/graft-ui/src/keymap/table.rs:288-289`, `:292-313` — fixed labels, no operation variant.
- **Ruling used:** ADR-0043 §4, "One name per side, everywhere"; keys stay positional (`o` stage 2, `t` stage 3).
  Keymap §2.6 says `o` takes ours (stage 2) — the meaning; ADR-0043 gives its name per operation. Not ambiguous.
- **Fix (cause):** the palette's `Source` carries the open resolver's `SideRoles` (`side_names(state).roles`, the
  bar's source); `listing` renames the six side rows (`take_*`, `all_*`, `file_*`) from them before filtering. Table
  labels unchanged (man page, no-operation contexts).

## Acceptance (each tick names its test)

- [x] A, app level: `it::outside_repo::a_stash_newer_than_head_does_not_take_the_launch_landing` — real binary on a
  pty, `FeatureBranches` + `git stash push -m "parked work"`; red on base (timed out, cursor on the stash row), green
  after; 3/3 reruns green alongside `starting_inside_a_repository_lands_on_head`, `launch_and_open_land_alike`.
- [x] A, unit: `history::tests::a_seek_still_out_survives_a_view_change` — red on base (the view change asked
  `Range`, not `Find`), green after.
- [x] B, rebase: `update::resolver::tests::help_words::help_in_a_rebase_names_the_sides_as_the_bar_does` — all six
  rows: `Take upstream`, `Take yours`, `Upstream/Yours in every block`, `… for the whole file, done`; red on base
  (`Take ours`).
- [x] B, stash pop: `update::resolver::tests::help_words::help_in_a_stash_pop_names_the_sides_as_the_bar_does` —
  `Take yours` / `Take stashed`; red on base.
- [x] Gates workspace-wide: fmt clean; clippy `-D warnings` clean; nextest 6327/6327; `cargo test --doc` ok.

## NOTES

Touches:
- `History::change_view` callers: `update/keys.rs:450` (view keys) and `tree_answers.rs:194` (stash list). A `G` or
  search seek still out across a view key now survives instead of being withdrawn; full nextest green
  (`a_view_change_puts_the_cursor_back_on_the_selected_commit`, promotion tests unchanged).
- `History::seek` split: asking half is `find` (pub(super)); behaviour of `seek` unchanged.
- Palette `Source` new field `sides`; `outside_source` (empty state) `None`. `⌃P` listing in the resolver gets the
  same names (same `listing`). Recent runs store targets, not labels — unaffected. `run.rs` cursor re-find compares
  rows built by the same `rows()`, unaffected.
- Captures: `grep -rl "Take ours\|Ours in every block" docs/shots` → none; full nextest regenerated nothing
  (`git status docs/shots` clean). No stale capture.
- Man page `docs/man/trib.1:288-306` keeps the table's generic labels (no operation there) — left as is.
- Merge resolver help changes too: `Take ours` → `Take yours` (its bar says `o yours`), per ADR-0043 §4.
- File-size ratchet rows raised (shows in diff): `history/tests.rs` 1468, `palette/mod.rs` 404,
  `resolver/tests.rs` 954, `it/outside_repo.rs` 581.
- MAP.md: no public item added or renamed (`find` is pub(super), `Source` pub(crate)); no line for `change_view`.

Scout: none — the review cited every line; verified by reading them at HEAD and by the pty repro. No compaction.
`code-review` skill not run (two small fixes; reviewer follows).
