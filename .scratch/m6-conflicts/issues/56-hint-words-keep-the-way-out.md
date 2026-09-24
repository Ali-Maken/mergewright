# hint-words-keep-the-way-out: at 120 columns the resolver's bar keeps `d start over` and `esc close` whole

Status: ready
From: user picks 2026-09-24 item 5 (batch 9, option A); `.scratch/m6-conflicts/user-facts-m632.md`; ADR-0044 §4
(amends ADR-0039's third amendment).
Amended 2026-09-24 (spec-only pass) after scout `.scratch/m6-conflicts/scouts/54-56.md`. The planner re-read
`graph_screen/hints.rs:71-87`, `hints/lists.rs:280-351` and `graph_screen/tests.rs:1141-1155`.
Build with: builder-sonnet — one struct field, one ladder step, and three list tables; the rule is ADR-0044 §4.
Needs: none. It is independent of the two colour specs, but in the same crate, so the same lane.
Crates: graft-ui only (plus `docs/shots/` regenerated). If a `graft` source test pins the bar and goes red, report
`blocked: needs graft`.
Lane: wave 1, lane U, third.

## Read first
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §4.
- `docs/adr/0039-*.md`, the third amendment (the ladder).
- `.scratch/m6-conflicts/scouts/54-56.md` "Hint ladder" (the code as it stands, with lines).
- `crates/graft-ui/MAP.md` line 22 (the ladder, `. more`, the `Resolver` table) and line 16 (the hints files).
- `.scratch/m6-conflicts/user-facts-m632.md` (the measured widths: 135 wanted, 137 for a rebase).

## Decisions (made in this pass)
1. **The flag is a field.** `Hint` (`hints.rs:74`, `Copy`) is built only by `hint()` (`:185`) and `planned()` (`:205`).
   The one other literal is `hints/tests.rs:1419`, a struct update (`..hint(..)`) that survives a new field. There is
   no literal outside graft-ui. So `keeps_words: bool` is a field, not a side table.
2. **Which lists mark what** (with 58's decision 5):
   - `resolver_list` (`lists.rs:291`): `d start over` (while `restart`) and `esc close`.
   - `resolver_card_list` (`lists.rs:328`): **`esc close` only**. The card has no `d`, correctly: an unmerged
     one-sided file has nothing chosen to start over. 58 adds no `d` there.
   - `resolver_done_list` (`lists.rs:343`): `d start over` (while `restart`; 58 makes it true on a done whole-file
     file) and `esc close`.
   - No other context marks any hint.

## Scope
**In:**
- `Hint` gains `keeps_words: bool`, `false` from `hint()` and `planned()`, and a builder `keeps_words(self) -> Self`.
- The ladder's step sequence (`hints/fitting.rs:50-66`) becomes:
  1. the three most relevant hints at `Full`;
  2. then every hint that keeps its words, at `Full`, in list order;
  3. then every remaining hint at its bare `Key`;
  4. then those raised one at a time from the left, short then full.
  It is still one fixed sequence, and the bar still draws its longest prefix that fits.
- The resolver's lists: decision 2.
- At 120 columns, the merge resolver with `B base` and `d start over` offered draws `d start over` and `esc close`
  whole; `j/k` and `M` go bare first.

**Out:**
- Short forms (`short-words-from-the-draft`, blocked).
- Hint order (`c finish` first belongs to `last-file-opens-the-summary`).
- Any context other than the resolver.

## The failing test first
`graft-ui/src/graph_screen/hints` tests: `resolver_bar_keeps_the_way_out_at_120`.
- `hints_for(Resolver { base: true, restart: true, whole_file: false, roles: Merge, done: false, finishable: false },
  Divider::Hidden, 120)` → `hints_text` contains `d start over` and `esc close`, and `M` and `j/k` appear bare.
- Red today: `fix-stop-lands-in-resolver-120x40` shows `M mergetool  d  esc` (user-facts-m632.md).

## Signatures
```rust
// graft-ui/src/graph_screen/hints.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hint { pub key: &'static str, pub verb: &'static str, pub state: HintState, pub rung: Rung,
                  pub keeps_words: bool }            // new field; `false` everywhere but decision 2's marks
impl Hint { #[must_use] pub fn keeps_words(self) -> Self; }   // the builder lists.rs uses
```

## Touches (each entry, and how the builder checks it)
- **Every other context's bar stays unchanged**, at every width.
  - Check: `only_the_resolver_marks_words` (iterate `every_hint_context()`; any marked hint outside `Resolver` fails)
    and the existing snapshot/width sweeps, green without edits.
- **Monotonicity in width.** Check: `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`
  (`graph_screen/tests.rs:1144`, **not** in the `hints` tests), unchanged and green.
- **The floor and `. more`.** The resolver is modal, so it has no `. more`. Check: the floor tests, green.
- **`graft` tests on the resolver's bar**: `update/resolver/tests.rs:621` `bar()` and
  `tests/file_list/start_over.rs:73,91` assert whole lists at **200** columns, where every hint is `Full`, so they
  should not change. No `graft` test at 120 was found. `every_key_the_bar_draws_is_answered`: the verbs and keys are
  unchanged.
  - Check: all green **unedited**. Red → `blocked: needs graft`, naming the test and line; the conductor files the edit
    in lane O after 58.
- **Captures of the resolver's bar at 100-160 columns.** The scout's grep found 12 with `  d  esc` or a trailing `esc`:
  `fix-card-names-numbers-*`, `fix-stop-lands-*`, `fix-file-list-many-120`, `m6-13-*`, `fix-file-keys-before-120`,
  `fix-file-list-rebase-step2-120`, `fix-take-all-120`, and contact sheets 1-2.
  - Check: `grep -rln "esc$\|  d  esc" docs/shots/m6` before and after; regenerate with `GRAFT_UPDATE_SHOTS=1` (a red
    run, then green). This runs `graft`'s capture tests from this worktree and edits no `graft` source. NOTES lists
    each file and its new bottom row. A change anywhere but the bottom row is a finding.
- **Captures shared with 55 and 58** (`fix-done-*`, `fix-file-keys-*`, `fix-stop-lands-in-resolver-*`): whichever
  merges second regenerates on top of the first, and capture text is never merged by hand (conductor).
- **ADR-0039:** ADR-0044 §4 records the amendment; 0039 is not edited.
- **Lane O** (57, 58): no shared crate. Check: `git diff --stat` names only `crates/graft-ui` and `docs/shots`.
- **Docs:** `crates/graft-ui/MAP.md` line 22: the step sequence, the flag, and which lists mark it.

## Acceptance (name the verifying test at tick time)
- [ ] `resolver_bar_keeps_the_way_out_at_120`: red first, then green.
- [ ] `resolver_bar_keeps_the_way_out_at_120_in_a_rebase` (rebase roles, 120 columns).
- [ ] `done_bar_keeps_the_way_out_at_100` (`resolver_done_list(true)`: `d start over`, `esc close` whole).
- [ ] `only_the_resolver_marks_words`
- [ ] `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` (existing, green)
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-bar-keeps-the-way-out-120x40.txt`, from the
      pty walk (`.scratch/m6-conflicts/pty-walk-m6.py`) against the built binary. No new `graft` capture test is
      added: that would be a `graft` edit.
  - Scene: the merge resolver on `notes.txt`, with `B base` and `d start over` offered, at 120 columns.
  - **What right looks like:** the bottom row contains `d start over` and ends `esc close`; `M` and `j/k` are bare
    keys; no hint is cut mid-word.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small. One context.

## NOTES
