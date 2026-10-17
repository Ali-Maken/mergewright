# 05r — resolver fixes (review ease-pass-05, Majors 1–3 + minors 1, 2, 3, 5, 6, 8)

Status: partial — 15/17 boxes; open: M3 failure's Cancelled words (design question), and the capture box (Q19 changed the bar; L recaptures). Builder compacted twice (NOTES).
Build with: Opus (`builder-opus-xhigh`) — conflicts area; every fix is checkable by fixture (bytes on disk, `ls-files -u`, exact words, cursor index). No structural choice left open, so not Fable.
Needs: from 05 (built): `Picks`, `Pick { path, blocks, after: Arc<ConflictFile> }`, `Restore`, `BlockAt`, `next_open_in_file`, `next_open`, `Picks::{undoing, reconcile, take_landing}`, `walk::{walk_blocks, wrap_target, no_more_conflicts}`, `done_refusal` (`done.rs:57`), `untake_ended` (`unpick.rs:204`). Ids `resolver.prev_block` (`N`), `app.undo` (`u`). Find keymap rows by id (03r moved them into `keymap/table/<context>.rs`).
Review: `docs/reviews/ease-pass-05.md`. Parent: `05-resolver-picks.md`. **Lands before 05b** (05b's `n` none-open merge words extend the words site this ticket makes).

## Read first
- `docs/reviews/ease-pass-05.md` (all; Majors 1–3, Minors).
- `.scratch/ease-pass/issues/05-resolver-picks.md`: Decided here, Scope, NOTES (deviations: `after`, `undoing`, `reconcile`).
- `docs/design/02-keymap.md` §2.1 bullets 1–2 and "hint bar is a ranked selection", §2.2 rows `↑ ↓ ← →` / `j k h l`, §2.6 (both tables).
- `crates/graft/MAP.md` `update::resolver`, `jobs::conflict`, `view::resolver`. `crates/graft-ui/MAP.md` `keymap` (:377-419), `graph_screen::hints` (:199-221).
- Scout `.scratch/ease-pass/scouts/05-resolver-picks.md` for file:line facts. Then source.

## Decided here
- **M1 key: `←`/`→` (and `h`/`l`) step to the previous / next clash in this file, open or taken.** Why these: §2.2 already gives `← →`/`h l` the movement meaning ("move left/right", movement only, no verb), and §2.6 leaves them unbound in the resolver; lazygit, the nearest keyboard-TUI peer, uses `←/→` between conflicts (`docs/research/conflict-resolution.md:113`); the confirm box's `←`/`→` already mean previous/next choice (§2.3c). Not `[`/`]` (§2.5's context lines: one meaning per key). Not a change to `n` (ADR-0063, user-decided) or `N` (§2.1's reversal pair; 05 kept it). The mock binds nothing here, so nothing it showed changes.
  - In the file only, **clamped**: at the first/last block the cursor stays and the notice reads `first clash in <file>` / `last clash in <file>`. Never crosses files (`j`/`k` do), never wraps.
  - Lands as `N` lands (cursor on the block, scrolled into view). Re-take on a taken block replaces it (Q16 a), as today.
  - Whole-file card (`Opened.file == None`): notice `<file> has no clashes to step through`. A file marked done (`⏎`/`O`/`T` in a non-merge op): `done_refusal` words. While a take/untake runs: answered exactly as `N` is then (same gate).
  - **Bar unchanged** (the mock's). Discoverable through `?` (§2.12 lists every key of the place) and the dead-end words below. Whether the bar should show `← → clashes` is the user's question (bottom), not built.
- **M1 words: the false `no more conflicts this way`.** One fn makes `n`'s and `N`'s words when no open clash is left anywhere: `no open clashes left · ← → step through them` when this file's document holds a taken block, else `no open clashes left`. Every emit site of `no more conflicts this way` (`walk.rs` `no_more_conflicts`; grep the string) either goes through it or is shown to be true (a clash, open or taken, exists that way — say why in NOTES). 05b later adds its merge arm (`· y reviews the merge`) in the same fn.
- **M1 part 2: taken clashes survive a re-read while their picks live.** Today `j`/`k` back to a file re-parse it, so its taken clashes become plain text and `←/→` could not reach them. A re-read whose fingerprint equals the **newest pick in that file**'s `after.fingerprint` shows `after` (taken blocks kept); this also removes the mixed-document stack (review minor 8) on that path. No picks in the file (relaunch, sealed) → the read as today; nothing records those takes. Bytes differ → M2.
- **M2: say, not keep.** Keeping picks over changed bytes would write over his edit, so the drop stays (05 Decided); it is now said. Sticky status, the same channel as `b.txt changed on disk since that pick · nothing undone`:
  - another pick remains: `<file> changed since your picks · u can't undo them now · next u: <other file>`
  - none remains: `<file> changed since your picks · u can't undo them now`
  Every path landing a fresh read of a picked file goes through `reconcile` (`j`/`k`, `e` return, refresh/watcher — grep the read-arrival callers, list them in NOTES). `reconcile` is `#[must_use]`.
- **M3: the shell-staged guard drops, never sticks.** `u` on a pick whose file git no longer calls unmerged (`write.rs:93-99`) writes nothing, drops that file's picks (as `Changed`; 05 Decided "a stale pick never blocks the stack"), Sticky `<file> is not conflicted any more · nothing undone`. If the build puts the pick back instead, that is the cause to fix.
- **Cancelled untake** must mean "never wrote". Check in `jobs::conflict::untake` that cancel is observed only before the write; name the line in NOTES. If a cancel can be reported after the write, **stop and report** — a design question, not a build one.

## Scope
In (graft app: `update::resolver` — `walk.rs`, `picks.rs`, `answers.rs` (:71-74 and the read-arrival path), `unpick.rs`, `resolver.rs` (`act` arms, `write_refusal` :307-315), `take.rs` (:78 comment only); `jobs::conflict::write.rs` (guard outcome only); tests under `update/resolver/tests/` and `tests/it/resolver/picks*`. graft-ui: the `Resolver` keymap rows, `tests/keymap_ids.txt`, `keymap/tests/screen_keys.rs`. Docs: `docs/design/02-keymap.md` §2.1 and §2.6, the exact text below):
- `←`/`→`/`h`/`l` rows, handlers, words as Decided.
- `none_open_words`, `reconcile` → `Reread`, `dropped_words`, the guard's drop.
- Minor 1: `write_refusal` asks `picks.undoing()` before `busy_words`, so every refused key reads `still undoing the last pick` while an untake runs.
- Minor 2: `take.rs:78` comment → "Question 16 answered (a), 2026-10-17; a refuse answer would branch here".
- Minor 3: the keymap doc edits below (the planner had no edit tool; apply them verbatim).
- Minors 5, 6, 8: tests only (Acceptance).
Out: bar words and drawing (03b/11); 05b's merge staging and words; `N`'s meaning; per-path unmerged read (minor 7); `U` mid-merge words (minor 4); recaptures (L).

**Before writing code:** check `scroll_key` (`scroll.rs`, the `_ if plain` fallback in `act`) for `h`/`l`/`←`/`→`. If they scroll the document sideways today, **stop and report `blocked: ←/→ scroll sideways`** — the doc never gave them that, but taking it away is a spec change.

## 02-keymap.md edits (verbatim)
- §2.6 first table, the `n` / `N` row (:352) → `| \`n\` / \`N\` | \`n\`: the first open clash from the top, across files (Decided below). \`N\`: previous block, taken ones included; at the file's first block, cross to the previous file; past the start, wrap to the first open block (\`wrapped · 2 still open\`), else \`no open clashes left\` (with \`· ← → step through them\` when the file holds a taken block) (P2; ticket 05r). On a done file, \`N\` crosses straight to the next file not done (ticket 17 D6). |`
- §2.6 first table, `j`/`k` row (:353): replace "the document scrolls with `n`/`N`," by "the document scrolls with `N`, `←`/`→`,".
- §2.6 Decided table: the `n` row's action → `The first **open** clash from the top, across files (the mock's \`nextOpen\`); was: next block, taken ones included`. Add after it: `| \`←\` / \`→\`, \`h\` / \`l\` | Previous / next clash **in this file**, open or taken (§2.2's movement keys; lazygit's \`←/→\`). Clamped: \`first clash in <file>\` / \`last clash in <file>\`; never crosses files. Not on the bar; \`?\` lists them (planner 2026-10-17, review ease-pass-05 M1, under ADR-0063) |`
- §2.1, the resolver bullet (:71-72): "`n`/`N` to next/previous block," → "`n` to the first open clash, `N` to the previous block, `←`/`→` between clashes in the file,".

## Signatures
graft-ui keymap (`Resolver` context, after `resolver.prev_block`; arrows built as the file rows build `↓`/`↑`):
```rust
b("resolver.step_prev", Resolver, &[k('h'), /* ← */], "Previous clash in this file, taken ones included"),
b("resolver.step_next", Resolver, &[k('l'), /* → */], "Next clash in this file, taken ones included"),
```
graft app, `update::resolver`:
```rust
/// The block before (`forward = false`) or after `at`, open or taken; `None` at the file's edge.
pub fn step_in_file(blocks: &[BlockAt], at: usize, forward: bool) -> Option<usize>;

/// What a fresh read of a file means for its picks.
#[must_use]
#[derive(Clone, Debug)]
pub enum Reread {
    /// No pick in this file: show the read.
    NoPicks,
    /// Bytes equal the newest pick's `after`: show that document, taken clashes kept.
    Picked(Arc<ConflictFile>),
    /// Bytes differ: the file's `count` picks were dropped; say so.
    Dropped { count: usize },
}
impl Picks {
    /// Replaces `reconcile(path, fingerprint) -> bool`.
    pub fn reconcile(&mut self, path: &Path, read: &ConflictFile) -> Reread;
}
/// M2's Sticky words; `next` = the file the next `u` reaches.
pub fn dropped_words(path: &Path, next: Option<&Path>, keys: &Keymap) -> String;
/// `n`/`N` with no open clash left anywhere; `taken` = this document holds a taken block.
pub fn none_open_words(taken: bool, keys: &Keymap) -> String;
```
Keys inside words are spelled from the keymap the way `done_refusal` spells `d`; if that one is a literal, match it and say so in NOTES.

## Touches (check each, report how)
- Keymap: `keymap_ids.txt`; `screen_keys` (resolver `h`/`l`/`←`/`→` → the new ids); the resolver list in `bound.rs` (05b names it at :127,130); config override names `[keys.resolver]`; `?` listing (§2.12).
- `update/tests.rs::every_bound_action_is_hinted_live_or_is_named_here`: the new ids take the standing `resolver.prev_block` has. **Adding them to `NEVER_HINTED` is a spec change — don't.** If the test can't pass without a bar slot, stop and report.
- 05b: `n` none-open merge words go through `none_open_words`; `file_state`'s `Solved` reads open blocks, which `Picked(after)` keeps equal to the read's — tested below.
- `view::resolver` / 11: taken blocks now stay drawn after `j`/`k` back; 11's `◀ ▶ ◆` look applies to them. Captures are L's.
- `m5_walk` journeys and `N` tests: unedited; must pass.
- 14 (acceptance): bar unchanged, so no edit; the conductor may add journey `change-earlier-clash 3 → → o (lazygit 4)` to the note's fence.

## Acceptance
Each tick names its test inline.
- [x] **M1 walk (the review's case):** one file, 3 clashes; `o o o`; `N N` → clash 1; `t`; `→ →` → cursor on clash 3; `o` → clash 3 reads kept-yours on disk. Same with `l l`. 3 keys from clash 1 to a changed clash 3 (`→ → o`; lazygit `→ → ↓ space`, 4). — it `resolver::picks::steps::the_arrows_reach_a_taken_clash_below_and_change_it`, `…::h_and_l_reach_a_taken_clash_below_and_change_it` (on `c.txt`: the fixture's `a.txt` has 2 clashes, NOTES)
- [x] `←` back to clash 1; `←` again → `first clash in a.txt`, cursor stays; `→` past clash 3 → `last clash in a.txt`; no file crossed. — it `resolver::picks::steps::the_arrows_stop_at_the_files_ends_and_say_which` (`c.txt`); unit `tests::step::the_arrows_reach_taken_clashes_and_stop_at_the_ends`, `tests::step::the_arrows_never_leave_the_file`, `tests::step::a_step_scrolls_its_clash_into_view`
- [x] `step_in_file`: edges, all open, all taken, mixed, one block, none. — unit `tests::step::step_in_file_takes_every_block_and_stops_at_the_edges`
- [x] `←`/`→` on a whole-file card → `<file> has no clashes to step through`; on a done file in a rebase → `done_refusal` words; while a take runs → as `N`. — unit `tests::step::the_arrows_on_a_card_say_there_are_no_clashes`, `…::the_arrows_on_a_file_with_no_clashes_say_so`, `…::the_arrows_on_a_done_file_in_a_rebase_get_the_done_words`, `…::the_arrows_while_a_take_runs_walk_what_n_walks`
- [x] `N` with nothing open anywhere and taken blocks in the file → `no open clashes left · ← → step through them`; with no blocks → `no open clashes left`; `no more conflicts this way` appears nowhere it is false (`git grep` count in NOTES). — unit `tests::step::n_and_shift_n_with_every_clash_taken_point_to_the_arrows`, `tests::step::n_and_shift_n_with_no_clashes_say_only_none_open`, `tests::cross::n_past_the_last_block_says_no_open_clashes_left`; it `resolver::picks::n_goes_to_the_first_open_clash_from_the_top`, `m5_walk::resolving` scenario `n_past_the_last_block_says_no_more`. Spelled `←/→` (NOTES).
- [x] **Taken survive `j`/`k`:** pick clash 1 of `a.txt`, `j`, `k` → clash 1 still `BlockState::Took`, `←`/`→` reach it, `file_state` unchanged. — it `resolver::picks::reread::taken_clashes_survive_j_and_k` (the file list's `RowState` stands in for `file_state`, which does not exist: NOTES); unit `picks::tests::reconcile_shows_the_newest_picks_document_while_the_bytes_are_its`
- [x] **Minor 8, two picks across a re-read:** pick clash 1, `j`, `k`, pick clash 2, `u`, `u` → bytes equal the file git wrote. — it `resolver::picks::reread::two_picks_across_a_reread_undo_to_the_bytes_git_wrote`
- [x] **M2:** picks in `b.txt` then `a.txt`; edit `a.txt` outside; `k`/`j` back to it → Sticky `a.txt changed since your picks · u can't undo them now · next u: b.txt`; `u` → `b.txt`'s pick undone, bytes equal pre-pick. Same through the `e` return path. No other pick → the short form. — it `resolver::picks::reread::a_file_changed_since_its_picks_says_so_when_read_again` (`k`, then `u`), `…::a_file_changed_in_the_editor_names_the_next_u` (`e`, long form), `…::a_file_changed_in_the_editor_says_its_picks_went` (`e`, short form), `…::start_over_drops_the_files_picks_without_words` (`d`: no false words); unit `picks::tests::reconcile_drops_a_file_whose_bytes_are_not_its_last_picks`
- [x] **M3 guard:** `clashing_merge`; pick in `b.txt`, `k`, pick in `a.txt`, `j`; shell `git add a.txt`; `u` → `a.txt` bytes unchanged, `git ls-files -u -- a.txt` empty, Sticky `a.txt is not conflicted any more · nothing undone`, `picks.in_file("a.txt") == 0`; `u` → `b.txt`'s pick undone. — it `resolver::picks::reread::u_on_a_file_the_shell_staged_drops_its_picks` (passed before any change: the guard already drops, NOTES)
- [ ] **M3 failure:** `untake_ended` with `UntakeFailure::Failed` → pick back on top, `undoing()` is `None`, the error in words (Sticky), next `u` retries it. Cancelled → pick on top, `undoing()` `None`, words. Success → not on top. — unit `tests::undo_ends::a_failed_untake_puts_its_pick_back_and_says_why` (Failed, all held), `…::an_untake_that_never_answers_puts_its_pick_back` (Cancelled and JobFailed: on top, `undoing()` `None`, next `u` retries), `…::a_written_untake_leaves_its_pick_off_the_stack` (Success). **Open: Cancelled's words not built** — a cancel can be reported after the write (`jobs/pool.rs:319`), so the ticket's stop rule applies: design question in NOTES.
- [x] **Minor 1:** an untake in flight; `o`, `j`, `esc` each → `still undoing the last pick`. — unit `tests::undo_ends::keys_refused_while_an_untake_runs_say_so`
- [x] **Minor 5:** cherry-pick stop and revert stop: `o`, `u` restores bytes; `c` → stack empty at the next stop. — it `resolver::picks::stops::a_cherry_pick_stop_takes_and_undoes_and_its_next_stop_starts_clean`, `…::a_revert_stop_takes_and_undoes_and_its_next_stop_starts_clean` (in `stops.rs`, not `lifetime.rs`: that file is at the size ratchet)
- [x] **Minor 6:** rebase stop, `O` → `picks.in_file(file) == 0`, `u` → `done_refusal` words. — it `resolver::picks::stops::shift_o_at_a_rebase_stop_leaves_no_pick_to_undo`
- [x] `?` in the resolver lists `h ←` / `l →` with their words. — unit `tests::help_words::help_lists_the_arrows_that_step_between_clashes` (keys spelled `←` / `→`: the arrows lead the chord list); graft-ui `keymap::tests::screen_keys::the_arrows_step_between_clashes_in_the_resolver`
- [x] **Q19 (a), user 2026-10-17:** a solved file's bar ends `· ←/→ clashes` (`FileSolved` and `AllSolved`); an open clash's bar does not. — graft-ui `graph_screen::hints::tests::bars::a_solved_file_bar_ends_with_the_way_back_to_its_clashes`, `…::each_clash_bar_draws_its_words`; `update::tests::hint_coverage::every_bound_action_is_hinted_live_or_is_named_here` passes with the ids hinted, not in `NEVER_HINTED`
- [x] 02-keymap edits applied verbatim. — `git diff docs/design/02-keymap.md` in NOTES (two deviations, named there)
- [ ] No capture: nothing drawn changes but notice words and taken blocks persisting (L recaptures). — **Not true any more:** Q19 adds `· ←/→ clashes` to the solved bar, a drawn change. No capture taken (recaptures are L's, Scope Out); the bar words are checked by test only, never by eye.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~60k.

## For KNOWN_GAPS (conductor files; not this ticket)
- Unverified: untake reads the whole status per `u` (`write.rs:94`); a per-path `ls-files -u -- <path>` suffices — release perf run (minor 7).
- Unverified: memory per pick (`Arc<ConflictFile>`); a rebase continued in the shell while the app runs (05 NOTES).
- Broken (stale): `m6-01`, `m6-12` captures since 05's key changes — L recaptures.
- Usability walk: `U` in the resolver mid-merge says `finish or abort the merge first` while `u` undoes picks (minor 4); `N` now overlaps `←` (previous, taken included) but also crosses files and wraps.
- Takes made before a relaunch, or sealed by `⏎`/`O`/`T`, show as plain text: nothing records them (by design; `d` starts a file over).

## Question for the user (not blocking; build without it)
On a solved file the bar reads `n next file · u undo pick · esc pause` or `y review the merge · u undo pick · esc pause` (the mock). Add `← → clashes` so the way back to an earlier clash is on screen? (a) yes, at the end · (b) no, `?` and the notice are enough. Rec (a): ux-principles heuristic 6, recognition over recall; costs a departure from the mock's bar.

**Answered (a)**, user 2026-10-17 (through the conductor): built — `hints/lists/clash.rs` pushes `hint(STEP_CLASHES, "clashes")` on `ClashAt::FileSolved | AllSolved`; Acceptance line Q19.

## NOTES (builder, 2026-10-18)

**Compacted twice mid-ticket.** Remaining for the conductor to split: (1) M3 failure's *Cancelled* words — a design question, below; (2) the capture box — Q19 changed the solved bar, so L's recapture now covers it. Everything else is built and ticked.

**Scout:** `.scratch/ease-pass/scouts/05r-resolver-fixes.md`. **Pre-check:** `scroll.rs` claims no `h`/`l`/`←`/`→` (its only `Left` is the mouse button, `scroll.rs:48`) — not blocked.

**Reds (test first):**
- `←`/`→`: `tests::step::a_step_scrolls_its_clash_into_view` — `left: (Some(1), Some(2)) right: (Some(2), Some(2))` (cursor unmoved).
- None-open words: `tests::step::n_and_shift_n_with_every_clash_taken_point_to_the_arrows` — `left: Some("no more conflicts this way") right: Some("no open clashes left · ←/→ step through them")`.
- Survive `j`/`k`: `left: [Open] right: [Took(Ours), Open]`.
- M2 via `k`: `expected a sticky line, got Ready`. Via `e`: `got Done { what: "took yours · main in a.txt · 1 of 2 open" … }`.
- Failed untake: `expected a sticky line, got Failed("could not undo the pick in three.txt: disk full")` (now `Status::sticky`, `unpick.rs` `untaken`).
- Minor 1 (`o`, `j`): `left: ("still writing the last take", None)`.
- `d` after M2's change: `Sticky "a.txt changed since your picks · u can't undo them now"` — a start over is not "changed since"; fixed by `picks.seal` in `restart.rs` `restart_landed`.
- **Green before any change** (tests-only items, or behaviour that already held): M3 guard (`u_on_a_file_the_shell_staged_drops_its_picks`: the guard already drops — `write.rs:96-99` returns `Stale`, the `Stale` arm drops; checked by disabling the guard: red on `nothing written`, then restored); minors 5, 6, 8; `?` listing (graft-ui rows landed first). `steps.rs` it tests were written after the handler; the unit reds above came first.

**M3 Cancelled — stop and report (design question, not built).** A cancel *can* be reported after the write. `jobs/runner/reads.rs:169` checks cancel before the work; inside `jobs/conflict/write.rs` `untake`, the stop reaches only `status::read` (`write.rs:95`), and `apply_untake` (`write.rs:101`) takes none — so the job never stops after writing. But `jobs/pool.rs:319` (`Ok(_) if cancel.is_cancelled() => JobCancelled`) turns a finished `Produced(Untaken(Ok))` into `JobCancelled` when the token was cancelled meanwhile. `untake_ended` (`unpick.rs:233`) then puts the pick back over bytes already restored; the next `u` gets `Stale` "changed on disk since that pick · nothing undone" — false words, no data lost. Reachable today only at shutdown: no resolver code sends `CancelJob` for an untake (grep `CancelJob(` in `update/resolver*`: none); `cancel_all` at `jobs/runner/mod.rs:289` and `pool.rs:276`, where nothing reads the answer and picks die with the process. Options: (a) `untake_side` returns `Outcome::Answered` (`pool.rs:316` delivers it whatever the token says) — then Cancelled means "never wrote" and words can say `nothing undone`; (b) leave it, Cancelled words say "may have been undone". Rec (a). So `JobCancelled` today: pick back on top, `undoing()` `None`, no words (tested); `JobFailed`: the dispatch's generic `Failed("{id} failed: …")` (`update/dispatch.rs:283`).

**Words site / `git grep "no more conflicts this way"`** (crates + docs/design): **1** — the const at `walk.rs:36`, emitted once (`walk.rs:194`), reached only from `walk.rs:185`: `N` past the start on a **whole-file card** with no other file. True there: the card is the one conflict and nothing lies that way. Every document path goes through `none_open_words` (`walk.rs:203`): `first_open`'s none arm, `walk_blocks` rule 3 on a document, `cross_from_done`. Keys spelled from the keymap (`key_verb`/`spell_keys`), as `done_refusal` spells `d` (`keymap.after`); a clause drops when its keys are unbound.

**Read-arrival callers** — all funnel through `answers.rs:48` → `conflict_arrived` (`answers.rs:61`), which calls `reconcile` at `answers.rs:77`: `open.rs:84` (opening, `j`/`k`, `n`), `answers.rs:274` (`e`/`M` hand-back), `answers/taken.rs:59` (take `Changed` re-read), `unpick.rs:303` (untake `Stale` re-read). Watcher/refresh does not re-read the document (it re-reads the set only), so it has no arrival to reconcile. A card (`opened.file == None`) passes `None`: no blocks, no picks.

**Deviations (said, not silent):**
- Spelled `←/→`, not `← →`: the keymap's joined spelling for a pair (as `j/k`); words, the bar (`←/→ clashes`) and the keymap doc all match the build.
- Keymap rows list the arrows first (`&[LEFT, k('h')]`) so `?` and words spell `←`/`→`, not `h`/`l`.
- 02-keymap Decided row: "Not on the bar" → "On the bar of a solved file only, as `←/→ clashes` at the end (Q19 a, user 2026-10-17)". Rest verbatim. ADR-0063 cited.
- `reconcile(&mut self, path, read: Option<&ConflictFile>) -> Reread` (a card read has no document); `Reread` also derives `PartialEq, Eq` for tests.
- `file_state` does not exist; the file list's `RowState` stands in.
- The review's walk names `a.txt` with 3 clashes; the fixture's `a.txt` has 2 — `c.txt` (3) used.
- Minor 5 tests in `picks/stops.rs` (`lifetime.rs` would cross the 400-line ratchet). A revert begun in the shell refuses `c` (`nothing Mergewright started is in progress…`), so that test continues it in the shell and sends `FocusGained`.
- New modules: `resolver/gate.rs` (`write_refusal`, `busy_refusal` — moved out so `resolver.rs` stays under 400: 394, its `over_400.txt` row removed) and `resolver/step.rs` (`clash_keys` keeps `act` under clippy's cognitive-complexity cap). `busy_refusal` now also gates `files.rs` switch and `base.rs` toggle (Minor 1: every refused key says `still undoing the last pick`).

**Contradiction in the ticket:** "`m5_walk` journeys and `N` tests: unedited" vs Decided's new words and "taken survive a re-read", which change what they assert. Edited (assertion text only, no journey step added or removed): `tests/cross.rs` ×4 (words; renamed `n_past_the_last_block_says_no_open_clashes_left` etc.), `tests/file_keys.rs` ×1, it `picks.rs::n_goes_to_the_first_open_clash_from_the_top` (cursor `(1, Some(0))` → `(1, Some(1))`: the taken clash now stays; words), `m5_walk/resolving.rs` `n_past_the_last_block_says_no_more` (words) and `start_over_brings_the_markers_back` (reopened title counts `(2, 2)` → `(2, 3)`: the taken block still shown).

**02-keymap diff:** §2.1 resolver bullet; §2.6 `n`/`N` row and `j`/`k` row; Decided `n` row and the new `←`/`→`, `h`/`l` row — text as in Scope, with the two deviations above.

**Touches:** keymap ids (`keymap_ids.txt`, `screen_keys`, `bound.rs`), `?` listing (unit test), hint coverage (passes, ids hinted, not `NEVER_HINTED`), bar (graft-ui `bars.rs` tests), `[keys.resolver]` overrides (ids follow the `resolver.*` pattern; no override test added — unverified), m5 journeys (pass), captures: **stale, not recaptured** — the solved bar (any capture showing `✓ Every clash … solved`, e.g. m6-08) and m6-01/m6-12 (05's keys); L recaptures. 05b: `none_open_words` is the site its merge arm extends.

**Knock-ons the gate run caught (fixed):** `docs/man/mwr.1` regenerated (`--generate-man`; diff: the two new rows only); `no_literal_keys/prose.rs` row for `gate.rs`'s `a write is already running` (an article, as `files.rs`'s row — in `resolver.rs` it sat after `#[cfg(test)]` and was never scanned); MAP line shortened under 240.

**For KNOWN_GAPS (conductor):** the ticket's five rows stand; `n`'s keymap label still reads `Next block` (`?`, man page) though ADR-0063 made it the first open clash — out of this ticket's rows, unfixed; add: Cancelled untake reported after its write (`pool.rs:319`, above) — question; solved-bar captures stale since Q19 — broken (stale), L.
