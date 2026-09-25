# 85 — usability walk 2 fixes: Esc and the history walk (A), stale diff after a take (B), pull words (D)

Status: done (run 1 compacted at 13/18; continuation 1 finished the rest, see NOTES)
Blocked by: none (84 merged). **86 runs after 85**: both edit the `graft` crate (`update/stash.rs` among them).
Crates: graft-ops (`apply/remote.rs` words only), graft-ui (graph header words), graft (and `docs/shots/m6`, the
`TESTING.md` rows named below, the m6 captions TSV)
Source: `docs/reviews/m6-usability-2.md` A, B, D; Minor m-b, m-i; the earlier walk's m2, m7, m11 (`KNOWN_GAPS.md`
:771, :776, :780). Every finding names its lines at `29290cb8`; re-check each against main.
Build with: Opus — A touches how the history walk is cancelled (`JobRunner::cancel` closes the walk's channel; a
wrong fix strands rows or holds a worker, KNOWN_GAPS :148-149). Every item is judged by tests that read state and
the tree, so not Fable.
Scout: `scout` first (ADR-0035, brownfield), ≤ 1 page, answering only "Scout brief".
Needs: none.

## Decided (planner, 2026-09-26) — not open here
- **A — cause:** `escape_key`'s last arm cancels `awaiting` (`update/keys.rs:518-523`), but a walk stays `awaiting`
  while parked (it serves rows on request, KNOWN_GAPS :148-149), so `Esc` kills an idle walk; `move_cursor` then asks
  rows only while `awaiting` (`:470-476`), so `G` past the laid-out rows draws blank.
  1. With nothing to back out of (every earlier `Esc` arm, and `dismiss_unseen`, unchanged and first), `Esc` cancels
     the walk **only while it is reading**: rows asked and not yet laid out, the header's `WalkView` `Opening`,
     `Reading` or `Jumping`. Parked or complete: `Esc` does nothing — no command, no status. One predicate,
     `walk_is_reading`, read by `escape_key` **and** by the header's `WalkView`, so the header shows reading exactly
     when `Esc` would cancel. Keymap §2.2's "cancel" means cancel what runs; nothing runs.
  2. A cancelled walk's header: `stopped at <N> rows · ⌃R reloads`, then `stopped at <N> rows`, then `stopped` —
     the widest that fits (`⌃` as the bar spells Ctrl keys).
  3. After a cancel, motions stay inside the laid-out rows: `G`, `j` at the end and `PgDn` land on the last laid-out
     row; nothing asks a dead walk for rows. Only `⌃R` starts a new walk.
- **B — cause:** a take (`o t b`, `⌃O ⌃T`, `O T`) writes the file through `TakeSide`, outside any `Operation`, so no
  refresh runs (`refresh` runs on `⌃R`, editor hand-back `update/mod.rs:390`, operations `update/operate.rs:57`);
  `show_cursor` then serves the pre-take diff from `diff_cache`, keyed by spec + entry only
  (`update/detail/ops.rs:293`). Fix at the event: `take_landed` calls `DetailState::file_written`, which drops every
  cached working-tree diff of that path and re-asks it when the changes pane's cursor is on it. Every other file write
  the scout finds outside an `Operation` and outside `refresh` calls it too. **Rejected:** refresh on resolver close —
  stale until close, and misses the next such write.
- **m-b:** the uncommitted header names **HEAD's own id**, read from the refs, never the graph's top row
  (`view/detail.rs:169-180`; the test at `:284` assumes the top row). Refs unread → `diff vs HEAD` with no id. In B's
  landing it read a stash commit's id.
- **m-i:** `u`/`U` in the resolver answer on its notice row (info, not a problem), never silence
  (`update/resolver.rs:362-382`): an operation live → `finish or abort the <kind> first` (§2.6a's words); a Graft
  stash or set-aside clash → the clause its landing notice already uses (`esc, u puts it all back` / `esc, u undoes the
  <switch|rebase> too`, from `update::clash`, not a copy); else `esc first · u undoes from the graph`. No key bound.
- **D stays a gap (KNOWN_GAPS :806, M7).** Not cheap: `p` has no preview, and ADR-0045 sets edits aside only because
  the preview says so — a set-aside on `p` is either silent or a new pull preview (+1 on a 1-key daily journey, a
  direction change); and it wraps a network write: `ApplyError::Remote` carries no `EditsAt`, a credential prompt or a
  cancel would come with his edits reset into the stash list, and ADR-0045 §7's proofs would need redoing for remote
  failures. **Taken here: the words.** graft-ops says `pull refused: your edits to <paths> would be overwritten` (no
  keys, no "commit or stash", as `EditsAt::words`), the app adds keys `S s stashes them first`; the diverged line loses
  `(M5)` and `shell: git pull --rebase` (m11). The keys it names must work where he is: **m2** — `S` and `J` answer
  from the changes pane as `c`/`p` do (`update/stash.rs:171`, `update/journal.rs:153`).

## Read first
- The review: A, B, D, m-b, m-i; its "Daily tasks" table rows for them.
- `docs/design/02-keymap.md` §2.2 (`Esc`, `Ctrl+R`), §2.6 (resolver keys), §2.6a (`u` refused while live).
- `KNOWN_GAPS.md` :148-149 (walk holds a worker; layout on demand) — read only these two rows.
- `crates/graft/MAP.md` :141 (`awaiting`), :204 (`opening`), :343 (`walk_started`), :359 (`taken`, `take_landed`),
  :368 (`update::keys` files), :472-474 (`update::clash`), :488 (`stash_key`), :553-593 (`update::detail`), :815
  (`JobRunner::cancel`), :854-867 (`History`).
- `crates/graft-ui/MAP.md` :105-158 (`graph_screen`, `WalkView`).
- `crates/graft-ops/MAP.md` :185 (`ApplyError::Remote`), and `crates/graft-git/MAP.md` :222 (`RemoteFailure`).

## Scout brief (code, not intent; ≤ 1 page)
1. `escape_key` (`update/keys.rs` ~:490-530): every arm in order. The `Phase` values; how the header's `WalkView` is
   derived; which field says rows are asked and not yet laid out, versus a parked walk. Does a complete walk keep
   `awaiting`?
2. `move_cursor` (`:470-476`) and `History::move_cursor`: what a motion past the laid-out rows does with no walk
   awaited; where the blank pane comes from.
3. Every job that writes a working-tree file outside an `Operation` (`TakeSide`, `apply_take_all`, anything else),
   where each lands in `update`, and whether that landing runs `DetailState::refresh`. How `show_cursor` builds its
   `DiffKey` for `DetailSpec::WorkingTree`.
4. `working_tree_header`'s inputs; where HEAD's id lives on `AppState` (`RefSnapshot::head()`?) and when it is unread.
5. Where `RemoteFailure::WouldOverwrite` and `NotFastForward` are worded (`graft-ops/src/apply/remote.rs:239-253`),
   and how the app turns a pull failure into a `Status` (`update/remote/in_flight.rs`): is there a `keys` part?
6. `S`/`J` from the changes pane: why they fall through to nothing (`update/stash.rs:171`, `update/journal.rs:153`,
   `update/keys.rs:219-228`).

## Signatures
```rust
// graft — update (A). The one reading of "the walk is reading"; header and Esc both ask it.
pub(crate) fn walk_is_reading(state: &AppState) -> bool;

// graft — update::detail (B)
impl DetailState {
    /// A file changed on disk without an operation. Drops every cached WorkingTree diff of `path` (any options,
    /// any max_lines); when the changes pane's cursor is on `path`, re-asks its diff. Nothing else moves.
    pub fn file_written(&mut self, path: &Path, next_id: &mut impl FnMut() -> JobId) -> Vec<Command>;
}

// graft — view::detail (m-b): HEAD comes in as a parameter, read from the refs by the caller.
// working_tree_header(.., head: Option<Oid>, ..)   // None → "diff vs HEAD", no id

// graft-ops — apply/remote.rs (D): words only, no signature change.
```

## Acceptance (name the verifying test at tick time; red first where it says so)
- [x] **A1** `update::keys::tests::esc_on_a_loaded_history_does_nothing` (red first: a settled 6-commit fixture;
      `Esc` → no `Command::CancelJob`, `awaiting` unchanged, header `6 commits`).
- [x] **A1** `update::keys::tests::esc_while_reading_cancels_the_walk` (rows asked, not laid out → `CancelJob`;
      header `stopped at <N> rows · ⌃R reloads`).
- [x] **A1** `view::tests::the_header_reads_reading_exactly_when_esc_would_cancel` (every `WalkView` × the predicate).
- [x] **A2** graft-ui `graph_screen::tests::the_stopped_header_keeps_the_widest_wording_that_fits` (120 and 80).
- [x] **A3** `update::keys::tests::g_after_a_cancelled_walk_lands_on_the_last_laid_out_row` (red first: no
      `ShowRows`; cursor on the last laid-out row; the view has rows).
- [x] **A walk** (real repo, `tests/it/`, builder names the file; a fixture longer than one row request, laptop tier):
      `esc_then_g_on_a_long_history_draws_its_last_rows` — `esc G`: header unchanged, bottom rows drawn, no `stopped`.
- [x] **A capture** (rule 6; colour **unverified**, rule 5): `docs/shots/m6/esc-then-g-120x40.txt`, written by that
      walk through `check_shot`. **What right looks like:** the header reads `<N> commits`, not `stopped`; the graph
      pane's last rows are commits (sha, subject), not blank; the bar is the row's own keys.
- [x] **B** `update::resolver::tests::a_landed_take_drops_the_panes_cached_diff` (red first: pane cursor on the
      file, diff cached with markers; the take lands → no cached entry for the path, and a `ReadFileDiff` for it).
      Plus one test per other write site the scout found (named in NOTES), or the line showing it already refreshes.
- [x] **B walk** `m6_stash_put_back::pop::a_take_then_esc_shows_the_file_without_markers` — walked as
      `S p esc ⏎ o esc`, not `S p o esc` (see NOTES, continuation 1): the pane's served diff has no marker line; the
      file on disk is `committed\n`. Red observed (take_landed's `file_written` call removed → the pane serves the
      three marker lines).
- [x] **B capture:** `docs/shots/m6/stash-clash-take-esc-pane-120x40.txt`, from that walk (`check_shot` in
      `a_take_then_esc_shows_the_file_without_markers`; colour unverified). Body empty: yours is HEAD's content. **What right looks like:**
      the changes pane on `notes.md` under `CONFLICTS`; the body shows the taken line and no marker line; the header
      reads `diff vs HEAD <main's sha7>` (m-b), not the stash's.
- [x] **m-b** `view::detail::tests::the_uncommitted_header_names_head_not_the_top_row` (red observed by mutation,
      continuation 1: the graph's top row passed back in → `HEAD 0303030`; red first: row 0 is `fix`'s
      tip, HEAD is `main`; and a stash commit on top). Replaces the test at `:284`.
- [x] **m-i** `update::resolver::tests::u_in_the_resolver_says_where_undo_is` (red observed by mutation,
      continuation 1: the `u`/`U` arm disabled → notice `None`; the three arms, `u` and `U`; notice
      set, no command, `state.status` untouched). `every_key_the_bar_draws_is_answered` and
      `every_bound_action_is_hinted_live_or_is_named_here` green **without** a new `NEVER_HINTED` row — if one is
      needed, stop: that is a spec change.
- [x] **D, graft-ops** `apply::remote::tests::would_overwrite_names_the_paths_and_no_keys` and
      `not_fast_forward_says_no_plan_shorthand`; NOTES: `grep -rn "(M5)\|commit or stash first\|shell: git pull"
      crates/graft-ops/src` → empty.
- [x] **D, app** `update::remote::tests::a_pull_that_would_overwrite_names_s_s` (reason + keys `S s stashes them
      first`; keys whole at 80).
- [x] **m2** `update::keys::tests::s_opens_the_stash_list_from_the_changes_pane`,
      `update::keys::tests::j_opens_the_journal_from_the_changes_pane` (red observed by mutation, continuation 1:
      the `graph_has_focus()` guard put back on `S`/`J` → both red, and the D walk red at `S`; moved from stash/journal tests: they go
      through the full dispatcher; see NOTES).
- [x] **D walk** `pull_over_edits::a_ff_pull_over_overlapping_edits_names_keys_that_work`
      (`crates/graft/tests/it/pull_over_edits.rs`; 7 keys, NOTES) —
      origin ahead, an unsaved edit to the same file: `p` → the status above, tree and HEAD unchanged; then the keys it
      names from the changes pane (`S s ⏎`, then `esc p`) → HEAD at origin's tip, one stash listed. NOTES: the keys
      pressed and the count (no note count; D stays a gap).
- [x] Contact sheet rebuilt (`scripts/contact-sheet.sh --captions-order m6 docs/shots/m6`), captions rows for the two
      new captures; each part ≤ 200 000 B; no machine paths. Doc, no test: verified by the script's output (178
      captures, 0 without a caption, 4 parts, 176 526–179 772 B) and a grep of the parts for home/temp paths → none.
- [x] MAP.md: graft (`walk_is_reading`, `escape_key`'s rule, `file_written`, the header's HEAD, resolver `u`, `S`/`J`
      from the pane); graft-ui (the stopped header ladder); graft-ops (the two lines' words). Doc, no test.
- [x] TESTING.md (doc, no test; rows listed in NOTES, continuation 1): rows for the two new captures and the D walk; any row quoting `stopped at`, `commit or stash first`
      or `(M5)` reworded. NOTES list each row.

## Touches (check each, report how)
- **Every test and walk that presses `Esc` with nothing open** and expected a cancel: flips for a parked walk. Check:
  `grep -rn "CancelJob" crates/graft/src/update/keys*` and the tests asserting it; NOTES list each changed.
- **Captures reading `stopped at`**: `grep -rl "stopped at" docs/shots` — each re-taken; NOTES say which now read
  `<N> commits` and why.
- **The walk's worker** (KNOWN_GAPS :148): a parked walk is no longer freed by `Esc`; `⌃R` and shutdown still free it.
  Check: the existing pool/cancel tests green; NOTES name them.
- **M6/24** (`KNOWN_GAPS.md` :642, walk cancelled before `Started`): unchanged; its tests green.
- **The detail pane's cache and `kept_cursor`**: `file_written` moves no cursor. Check: the m3/04 cursor tests green.
- **Resolver hint bar**: unchanged. Check: bar tests, `every_planned_hint_is_still_unbound`.
- **Captures showing `diff vs HEAD <sha>`**: `grep -rl "diff vs HEAD" docs/shots` — any whose id changes is re-taken;
  NOTES list them.
- **Remote failure words** in captures and TESTING.md: `grep -rn "commit or stash first\|(M5)" docs/shots TESTING.md
  README.md`; each reworded or re-taken.
- **`S`/`J` from the pane**: the pane's own keys (`s` stage, `j` move) unchanged — `S`/`J` are uppercase. Check: the
  changes-pane key tests green; keymap §2.2/§2.4 claim no `S`/`J` there (grep in NOTES).
- **Journeys:** counts unchanged. Check: `journey_counts_match_the_note` and the m5_walk stash journeys green.
- **Acceptance walks and captures to re-run after the last edit:** `m5_walk::every_scenario::journey` (all);
  `.scratch/m6-conflicts/pty-walk-m6.py` m6-20…25 at three sizes with `GRAFT_UPDATE_SHOTS=1`, then a second pass
  without it — every capture `settled`; `git status docs/shots` lists only the captures named above, those the greps
  found, and the sheet parts.
- **Doc rows owed (conductor, not the builder):** keymap §2.2 `Esc` — "with nothing open, cancels only a history read
  in flight"; §2.6 — `u`/`U` answered in words, not bound. KNOWN_GAPS rows :771 (m2), :776 (m7), :780 (m11) close;
  :806 (D) stays, source updated.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Large (~75k). If it compacts, the split is: **85a** A + D + m2 (keys, header, pull words; the `esc-then-g` capture) /
**85b** B + m-b + m-i (detail cache, header's HEAD, resolver `u`; the pane capture).

## NOTES (builder, 2026-09-26)

**Compacted once mid-ticket.** Status partial. Scout used: `.scratch/m6-conflicts/scouts/85.md`.

### Ticks and their tests (all green in the workspace run, 3502/3502 nextest + doctests)
- A1: `update::keys::tests::esc_on_a_loaded_history_does_nothing`, `esc_on_a_paused_walk_does_nothing`,
  `esc_while_reading_cancels_the_walk` (first screen asked and a PgDn ask), `view::tests::the_header_reads_reading_exactly_when_esc_would_cancel`
  (10 states, every `WalkView` variant). Red observed for three of the keys tests by reverting the fix.
- A2: graft-ui `graph_screen::tests::the_stopped_header_keeps_the_widest_wording_that_fits` (120, 80, 70) and
  `a_paused_walk_and_a_reading_one_read_apart`.
- A3: `update::keys::tests::g_after_a_cancelled_walk_lands_on_the_last_laid_out_row` (red observed).
- A walk: `crates/graft/tests/it/esc_and_the_walk.rs` `esc_then_g_on_a_long_history_draws_its_last_rows` (Linear, 200
  commits, 120x40). A capture: `docs/shots/m6/esc-then-g-120x40.txt` (written by `check_shot`; colour unverified).
- B: `update::resolver::tests::a_landed_take_drops_the_panes_cached_diff` (red observed).
- m-b: `view::detail::tests::the_uncommitted_header_names_head_not_the_top_row` (red **not** run separately).
- m-i: `update::resolver::tests::u_in_the_resolver_says_where_undo_is` (merge live, Pop clash, set-aside switch and
  rebase, plain; `u` and `U`). Red not run separately. No `NEVER_HINTED` row added; both bar sweeps green.
- D graft-ops: both tests named in the tick. `grep -rn "(M5)\|commit or stash first\|shell: git pull" crates/graft-ops/src` → empty.
- D app: `update::remote::tests::a_pull_that_would_overwrite_names_s_s`.
- m2: **names differ from the tick** — the tests go through the full dispatcher (`update`), so they live in
  `update::keys::tests::s_opens_the_stash_list_from_the_changes_pane` and
  `update::keys::tests::j_opens_the_journal_from_the_changes_pane`. Red not run separately.

### Decisions (read these)
- **Cause of the blank pane after `G`** (cause, not symptom): after a cancel, `History::move_cursor(Bottom)` put cursor
  and top at `rows-1`, outside the held window; nobody can fetch it, so `visible()` was empty. Fix: once the walk is
  Cancelled/Failed, motions clamp to the held window (`History::reachable`, `scroll/query.rs`, `scroll/apply.rs`).
  "Last laid-out row" is therefore read as the last row **on hand**; the header keeps `stopped at <laid_out> rows`.
- `walk_is_reading` = `awaiting.is_some()` and (no history, or `History::is_reading`: streaming and a jump, an
  outstanding request or no window yet). The header draws Reading/Jumping exactly then; otherwise a new
  `WalkView::Paused` (`N rows so far`). Reading words became `reading… N rows`. M6/24 exception kept: no history and
  nothing awaited still draws Opening.
- B write sites: `file_written`'s only caller is `take_landed`. The other resolver writes already refresh as
  Operations: ⏎ stage, `d` restart, whole-file `o`/`t` (`update/mod.rs:520-545`, `operate.rs:57`); `e` and `M` hand back
  through `update/mod.rs:390`.
- m2: opening `S`/`J` from the pane moves focus to the graph (the overlay takes the graph's keys).
- D: the m4 walk `pull-dirty-file-refused` (`tests/it/m4_walk/fetch_pull.rs`) now asserts the new words — that is a real
  repo pressing `p`, so the first half of the D walk is covered; the `S s ⏎`, `esc p` half is **not walked**.

### Touches (checked)
- Esc tests that expected a cancel: `update::tests::esc_cancels_the_walk_the_screen_is_waiting_for` and
  `no_chunk_lands_after_esc` now cancel while reading (`laid_out_so_far`). Pool/cancel tests green.
- Captures: `grep -rl "stopped at" docs/shots` → none. `diff vs HEAD` in 10 m3/m4/m5 captures; all `check_shot`
  captures green in nextest, so none drifted. Remote words: `grep` of `commit or stash first\|(M5)` in docs/shots,
  TESTING.md, README.md → empty after rewording TESTING.md rows 4-5 (m4 pull) and README.md pull paragraph (its diverged
  wording was already stale: the app says `m merge it · r rebase onto it`).
- Journeys and m5_walk: in `tests/it`, green in the workspace run. Changes-pane key tests green.
- MAP.md: graft (`History::is_reading`, `DetailState::file_written`), graft-ui (`WalkView`). Not yet: graft-ops words,
  escape rule, header HEAD, resolver `u` lines.

### Remaining (for the split)
1. B walk `m6_stash_put_back::pop::a_take_then_esc_shows_the_file_without_markers` + capture
   `docs/shots/m6/stash-clash-take-esc-pane-120x40.txt`.
2. D walk `a_ff_pull_over_overlapping_edits_names_keys_that_work` (`S s ⏎`, `esc p` from the changes pane).
3. `.scratch/m6-conflicts/pty-walk-m6.py` m6-20…25 at three sizes, update pass then check pass — **not run**.
4. Contact sheet rebuild + captions TSV rows for the two new captures.
5. TESTING.md rows for the two new captures and the D walk; remaining MAP.md lines (above).

## NOTES (continuation 1, 2026-09-26)

Did not compact. Scout: not re-read — the remaining items were walks and docs; the ticket and run 1's NOTES named the
code. Branch continues from `dc7da822` by a merge commit (`reset --hard` was refused by the permission system).

- **B walk deviates from the tick's keys** (cause, not a workaround): on `S p o esc` the pane never read the file
  before the take (`state.detail.diff` is `Idle` under the resolver), so nothing stale is cached and the walk was
  **green with the fix removed** (observed). The review's walk had looked at the file first. `S p esc ⏎ o esc` is that
  look: `esc` serves the diff with the three marker lines (asserted), `⏎` reopens the resolver; with the
  `file_written` call removed the walk goes red (observed). After the take the pane body is empty at 120x40 and
  187x50, rightly: yours (`committed`) is HEAD's content. So the capture cannot show "the taken line"; its header
  says so, and the test reads the served diff.
- **m-b in the walk is not red-capable:** in this fixture main's tip is the graph's top row (the stash sorts below),
  so the top-row mutation leaves the walk green. The unit test carries m-b's red.
- **Red by mutation, each restored** (`git status crates/*/src` clean after): B walk (above); m-b (top row passed in →
  `HEAD 0303030`); m-i (arm disabled → notice `None`); m2 (`graph_has_focus()` guard put back → both unit tests and
  the D walk red).
- **D walk** keys: `p` (sticky `pull refused: your edits to shared.txt would be overwritten`, keys
  `S s stashes them first`, HEAD and tree unchanged), `⏎` into the changes pane (focus `Files` asserted), `S s ⏎`,
  `esc p` → HEAD at origin's tip, one stash listed. **7 keys**; no note row (D stays a gap, M7).
- **pty re-run:** `pty-walk-m6.py` for m6-20…25 at 80x24, 120x40, 187x50: a `GRAFT_UPDATE_SHOTS=1` pass, then a check
  pass. Every capture settled (m6-22 and m6-24 at 187x50 also print `over 8 KB`, 9 771 and 9 237 B — the script's soft
  cap, as before). `git status docs/shots`: **no pty capture changed**.
- **TESTING.md rows:** three paragraphs after "Preview warns a put-back clash" (85 A `Esc` on a history nothing is
  loading; 85 B a take, then back to the pane; 85 D a pull over his edits), and both captures added to "The Rust
  captures of the same pass". `grep "stopped at\|commit or stash first\|(M5)" TESTING.md` → empty.
- **MAP.md:** graft — `walk_is_reading` (with `escape_key`'s rule), `detail_view` (the header's HEAD from the refs),
  resolver `u`/`U`, `journal_key`/`stash_key` (from the pane); graft-ops — `ApplyError::Remote`'s two lines' words.
  graft-ui's stopped ladder was already in `WalkView` (run 1).
- **Captions TSV:** rows for `esc-then-g-120x40.txt` and `stash-clash-take-esc-pane-120x40.txt`, appended (ticket
  order, after 84).
- Gates after the last edit, workspace-wide: fmt ok, clippy ok, nextest 3504/3504 (1 skipped), doctests ok.
