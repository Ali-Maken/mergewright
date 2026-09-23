# fix-stop-opens-the-resolver: a stop with anything unmerged lands in the resolver on the first conflicted file; `Esc` goes to the banner

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 12-landing-after-a-stop.md, 13-resolver-file-list.md
Was: draft · **P5 approved** (user, 2026-09-23: "keep all 5"). The conductor rewrites the §2.6a sentence in `02-keymap.md` (the note's "Keymap rows owed").
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Sonnet**: two call sites of an existing function, fully specified.
Needs: `fix-landing-after-a-stop` (`Status::Stopped`, `stop_reason`, the banner the resolver's `Esc` returns to); `fix-resolver-file-list` (the list shown on landing).
Crates: graft.

## Read first
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P5.
  - Amendment 2: Journeys J1, J2, J7, "Where each event lands him"; New UI "Below 180 columns".
- `docs/research/ux-principles.md` §3.1.
- `crates/graft/MAP.md` §`update::resolver` (`open_resolver`, `OpenedFrom`), §`update::operate` (`operated`), §`update::banner` (`resumed`, `first_conflicted`, `unmerged_paths`).
- **Scout question:** whether the `Conflicted` answer's `paths` arrive in git's status order (`unmerged_paths`'s), or only after the re-read.

## Scope
In:
1. On a `Conflicted` answer (`operate::operated`) and on a `Conflicted` again (`banner::resumed`) with at least one unmerged path:
   - After `Status::Stopped` and banner focus are set, call `open_resolver(first, OpenedFrom::Banner)`.
   - `first` is the first of the answer's paths in git's order (answers beat re-reads). If the answer's order is not git's, wait for the working-tree read and use `first_conflicted`. Say which in NOTES.
   - The file list's `▸` is on `first`.
2. `Esc` in the resolver → the banner is focused, with the Stopped line and its keys on the bottom row (today's `OpenedFrom::Banner` path).
3. **Unchanged:**
   - a stop with nothing unmerged (banner only);
   - a launch into a live operation (banner, Decision 2);
   - a stash-pop conflict (no banner; resolver from the pane, as today).
4. **Existing tests.** Update `fix-landing-after-a-stop`'s `a_stop_keeps_the_banner_keys_on_the_bottom_row` to press `Esc` first; its assertions are unchanged. Say so in NOTES.

Out: any new key.

## Acceptance (name the verifying test when you tick)
- [ ] `update::operate::tests::a_conflicted_merge_lands_in_the_resolver`: `m m` → the resolver is open on `first_conflicted`, `opened_from == Banner`, the cursor is on block 0, and `file_cursor` is that file's index.
- [ ] `update::banner::tests::a_rebase_stopping_again_lands_in_the_resolver`.
- [ ] `update::resolver::tests::esc_from_a_landed_resolver_shows_the_banner_keys`.
- [ ] `…::an_edit_stop_with_nothing_unmerged_keeps_the_banner`.
- [ ] Journey `a-stop-to-first-conflict` at 0 keys (acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-stop-lands-in-resolver-120x40.txt`, the two-file fixture. **What right looks like:**
  - The frame right after the stop is the resolver, with the header still `MERGING`.
  - The strip lists both files, with `▸` on the first.
  - The identity row reads `▸ <that file> · …`.
  - The bar starts with the resolver's keys (`o yours`).
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small. One context.
