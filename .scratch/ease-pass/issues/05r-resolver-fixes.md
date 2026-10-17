# 05r — resolver fixes (review ease-pass-05, Majors 1–3 + minors 1, 2, 3, 5, 6, 8)

Status: ready
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
- [ ] **M1 walk (the review's case):** one file, 3 clashes; `o o o`; `N N` → clash 1; `t`; `→ →` → cursor on clash 3; `o` → clash 3 reads kept-yours on disk. Same with `l l`. 3 keys from clash 1 to a changed clash 3 (`→ → o`; lazygit `→ → ↓ space`, 4). — it test
- [ ] `←` back to clash 1; `←` again → `first clash in a.txt`, cursor stays; `→` past clash 3 → `last clash in a.txt`; no file crossed. — it test
- [ ] `step_in_file`: edges, all open, all taken, mixed, one block, none. — unit
- [ ] `←`/`→` on a whole-file card → `<file> has no clashes to step through`; on a done file in a rebase → `done_refusal` words; while a take runs → as `N`. — unit
- [ ] `N` with nothing open anywhere and taken blocks in the file → `no open clashes left · ← → step through them`; with no blocks → `no open clashes left`; `no more conflicts this way` appears nowhere it is false (`git grep` count in NOTES). — unit + it
- [ ] **Taken survive `j`/`k`:** pick clash 1 of `a.txt`, `j`, `k` → clash 1 still `BlockState::Took`, `←`/`→` reach it, `file_state` unchanged. — it test
- [ ] **Minor 8, two picks across a re-read:** pick clash 1, `j`, `k`, pick clash 2, `u`, `u` → bytes equal the file git wrote. — it test
- [ ] **M2:** picks in `b.txt` then `a.txt`; edit `a.txt` outside; `k`/`j` back to it → Sticky `a.txt changed since your picks · u can't undo them now · next u: b.txt`; `u` → `b.txt`'s pick undone, bytes equal pre-pick. Same through the `e` return path. No other pick → the short form. — it tests
- [ ] **M3 guard:** `clashing_merge`; pick in `b.txt`, `k`, pick in `a.txt`, `j`; shell `git add a.txt`; `u` → `a.txt` bytes unchanged, `git ls-files -u -- a.txt` empty, Sticky `a.txt is not conflicted any more · nothing undone`, `picks.in_file("a.txt") == 0`; `u` → `b.txt`'s pick undone. — it test
- [ ] **M3 failure:** `untake_ended` with `UntakeFailure::Failed` → pick back on top, `undoing()` is `None`, the error in words (Sticky), next `u` retries it. Cancelled → pick on top, `undoing()` `None`, words. Success → not on top. — unit tests
- [ ] **Minor 1:** an untake in flight; `o`, `j`, `esc` each → `still undoing the last pick`. — unit
- [ ] **Minor 5:** cherry-pick stop and revert stop: `o`, `u` restores bytes; `c` → stack empty at the next stop. — it tests (`lifetime.rs`)
- [ ] **Minor 6:** rebase stop, `O` → `picks.in_file(file) == 0`, `u` → `done_refusal` words. — it test
- [ ] `?` in the resolver lists `h ←` / `l →` with their words. — unit
- [ ] 02-keymap edits applied verbatim. — `git diff docs/design/02-keymap.md` in NOTES
- [ ] No capture: nothing drawn changes but notice words and taken blocks persisting (L recaptures).

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
