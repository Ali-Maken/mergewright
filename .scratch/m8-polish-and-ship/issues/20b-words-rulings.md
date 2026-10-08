# 20b — words rulings: ten open words/hint questions decided

Status: ready (conductor 2026-10-08) — was draft (planner spec-only pass, 2026-10-08)
Build with: **Sonnet**, both parts (planner's expectation; conductor confirms after the check-first or a scout).
Every wording is decided below; each code change is a string, a typed payload or a field, held by a named test. Not
Fable: no signature another ticket builds on, nothing structural. Not Opus: each judgement call has a stop rule.
Scout: none expected. Each row carries its line; each part's **Check first** is the scout (11f's pattern). Say so in
the report.
Needs: Part B's one clone box needs Part A's `graft_ops::apply::askpass_words`. Nothing else crosses.

## Parts, crates, and builder 19b

| Part | Crate set (files) | Overlap with 19b (graft `event_loop/places.rs`, `empty_state.rs`; graft-ui `empty_state`, `palette`) |
|---|---|---|
| **A** | graft-git `run/askpass.rs`, `run/askpass/shared.rs`; graft-ops `apply/remote.rs`, `apply/error.rs`, `apply/error/words.rs`, `apply/lost_edit.rs`, `set_aside/answer.rs`, `clone.rs`, the `remote::short` words, their tests; both MAP.md | **None.** graft only if a `NotAsTheyWere { .. }` pattern breaks (check first 3); those lines are not in 19b's files. |
| **B** | graft `update/banner/stop/tab.rs`, `update/banner/resolve_words.rs`, `update/banner/pressed.rs`, `update/undo.rs`, `view/journal.rs`, `jobs/clone.rs`, their tests, `update/tests/keymap_words/*`; graft-ui `graph_screen/hints/lists.rs`, `detail_pane/header.rs`, `detail.rs`, `journal_screen/*`; `TESTING.md` radar stop 10; both MAP.md; captures | **No file overlap.** Shared: `crates/graft/MAP.md`, `crates/graft-ui/MAP.md` (different rows; conductor merges). B must not touch graft-ui `palette*` or `empty_state*`. |

A and B are disjoint crates and could run together, **but the two-cargo-helper cap holds**: with 19b running, start
**A** now (smaller, no overlap at all); **B** when 19b or A merges. B's clone box (B6) waits for A merged.

## Rulings

Principles cited: keymap §2.1 (a drawn key acts; a verb not valid at the cursor is not drawn; doorway; every key spelled
from the live table), §2.6a, ADR-0058 §4 (words or nothing outside modal places), ux-principles §2 heuristics (H2 real
words, H4 consistency, H6 recognition, H9 recover from errors), §3.1, §3.6; 10b W1/W3 (keys from the table; an unbound
key's clause dropped).

| # | Row (KNOWN_GAPS Id / source) | Lines (copied — builders don't read KNOWN_GAPS) | Ruling | Part |
|---|---|---|---|---|
| R1 | `M8/askpass-refusal-wording` | graft-git `run/askpass.rs:203` `note(self.shared.refusals())` → `run/askpass/shared.rs:55-57` `Err(PermissionDenied, "<first>; <n> refused in all")` → graft-ops `apply/remote.rs:149-152` pushes it as `AfterState::AskPass` → `apply/error.rs:248` "the credential prompt's socket could not be cleaned up". Fires when a process of **another uid** is turned away by the socket, not on his `Esc`. | **Change.** Today's words are untrue (H2, H9). A refusal gets its own words: `another user's program asked for your login and was turned away · nothing was sent: <detail>`. Typed, never by string or `ErrorKind` (a cleanup failure can be `PermissionDenied` too: rule 2): graft-git carries a `Refused` payload inside the `io::Error`; graft-ops reads it with `refused()`. Cleanup failure words unchanged. One words fn serves the remote ops and the clone notice. | A (+B6) |
| R2 | `M8/20-item14-words` (and `M7/r30p-q1`, KNOWN_GAPS :1072, same question) | graft-ops `apply/error/words.rs:18-27` (`its exact bytes are in the backup`); route test `recover_bytes::after_a_failed_raw_step_u_then_the_put_back_again_restores_his_bytes` (bytes return only after the way is cleared **and** `u` confirmed, then the put-back again); `J` has no `OpKey` (`keys.rs:14-45`); same words serve the set-aside put-back (`set_aside/answer.rs:269`). | **Change, lost-edit put-back only.** The route is tested there; name it without `J`: he just used that put-back (H6, H9, §3.1). Clause after the backup words: `: fix that, then <u> and put it back again` (plural `them`). `<u>` = `OpKey::Undo` through `KeyNames`; unbound → today's words exactly (W3). **Set-aside put-back keeps today's words**: there `u` undoes the whole operation and does not rewrite the path (graft-ops MAP `NotAsTheyWere`; `M7/30s-raw-fail` stays open), so the clause would be untrue. Ruling 2026-10-01 §1's "no `u` clause" still holds for `u` alone; this clause is a route, not a claim that `u` restores. | A |
| R3 | `M8/18c-file-url-remote-words` | 18c NOTES :94: a failed `file://` clone reads `no remote named '/nonexistent/…'`. `CloneError::explain` uses `remote::short` (18a NOTES). `RemoteFailure::NoSuchRemote { name }` (graft-git MAP `remote`) is what git's `'<x>' does not appear to be a git repository` classifies to. | **Change, in the shared words.** git prints the location, not a name, for a configured remote with a bad URL too, and Graft only fetches remotes it read from config; so `NoSuchRemote` is in practice "nothing there". `remote::short` for it: `no repository at '<name>', or no access to it` (git's own two causes, H2). Every remote op and clone get it (H4: one failure, one wording). Variant not renamed. | A |
| R4 | 11f Part A NOTES :360-363, :397-399 (no KNOWN_GAPS row): `c` refused off a CONFLICTS row in the changes pane is the bare reason | `resolve_keys` → `None` off a CONFLICTS row (`update/banner/resolve_words.rs`; graft MAP :484); one `⇥` from the file list lands on the diff (`banner/stop/tab.rs:51-56`), `tabs_to_banner` = 2 (`tab.rs:72-74`). | **Change.** A refusal names the way on (H9). Off a CONFLICTS row in the changes pane (files or diff), append the banner clause spelled for the real count: `1 file still conflicted · ⇥ ⇥ banner` from the file list, `· ⇥ banner` from the diff. The changes bar says `⇥ diff` there (`TabWord`), so only the refusal can tell him. Graph stays reason-only (its bar already draws `⇥ banner`). | B |
| R5 | 11f Part A NOTES :364-369 (no row): `u`/`U` refused from the changes pane say `⇥ banner` | `undo.rs:178`, `:453` (`banner_keys`, `undo.rs:184-186`, "pressed on the graph"); `in_progress_refusal` and `undone`'s `OperationLive` arm pass the literal `"⇥ banner"` (graft MAP :433, :440). | **Change.** The words name one `⇥` where one does not reach the banner: untrue (H9, §2.1 "a key displayed must work"). Every "go to the banner" clause comes from **one speller**, `tab_keys(refusal_tabs(state), keys)`, with `⇥` spelled from `app.next_pane` (W1; unbound → clause dropped, W3). §2.6a's words keep their shape; only the count becomes true. | B |
| R6 | 11f Part A NOTES :381-383 (no row): `space` is `resolved` in the bar, `space marks resolved` in the refusal | graft-ui `graph_screen/hints/lists.rs:522` `hint(STAGE, "resolved")`; graft `resolve_words` (`space marks resolved`). | **Change.** One key, one label (H4). Both read **`space mark resolved`**: a hint label is a verb (`resolve`, `edit`, `stage`), and the refusal's other clauses are key+label (`⏎ resolve  M mergetool  e edit`). One const in graft-ui, used by both, so they cannot drift. | B |
| R7 | `M8/evil-merge-not-checked-words` | `evil merge: not checked · 2 files hand-resolved` (graft-ui `detail_pane/header.rs:124-127`); `EvilResolved` means every other file was checked (`graft-ui/src/detail.rs:115-117`); graft-ui MAP `RadarLine`. | **Change, after check first B1.** Today's words say the whole merge was not checked; only the hand-resolved files were. Different facts, different words (§3.6, H2): `evil merge: none found · 2 hand-resolved files not checked` (`1 hand-resolved file`). Narrow rung: `evil merge: none found · 2 not checked`. `clean` stays reserved for every file checked and clean. | B |
| R8 | `M8/journal-refused-covers-failure` | graft `view/journal.rs:192-202` marks every `Aborted`, not-in-force, not-reversed entry `refused · nothing written`, git failures included. `EntryStatus::Aborted { reason }` (graft-core MAP) carries no kind. | **Change.** "refused" claims a cause the entry does not record; parsing `reason` is a special case (rule 2). The mark says the one fact every such row shares: **`nothing written`**. Variant `RowMark::Refused` → `RowMark::NothingWritten`. Q3 (`M8/aborted-refs-moved`, → 21) is not touched here. | B |
| R9 | `M8/15b-deviations` | 15b NOTES :38-50. | **Keep as built, all three.** (a) search reaches the header via `GraphView::search_tail`: one drop order in one file (`layout.rs`), no user-visible effect. (b) `n`/`N` lead only with a hit: with zero hits they step nowhere (`update/search.rs:169`), and §2.1 never draws a verb not valid at the cursor. (c) the last non-search group is cut with `…`, not dropped: a failure is shortened, never silently dropped while a cell is free (H1, rule 3's spirit); the search itself stays whole-or-absent. No code. | — |
| R10 | `M8/15-u-hidden-shell-begun-merge` | 15 NOTES :226-230; `in_progress_refusal` `None` when `shell_merge_known` (graft MAP :440-441); ops' gate passes only a step journaled inside the merge (§2.6a). | **Keep as built.** `u` acts there only when a whole-file choice or start-over was journaled inside this merge; otherwise it refuses. Drawing `u undo` in every shell merge would draw a refusing key (§2.1; 15's `no_key_a_live_operation_bar_draws_refuses_while_live`); drawing it exactly when it acts needs the executor's gate copied into the bar across three crates for one rare state. `?` lists it (§2.1 doorway); §2.6a documents it. No code. | — |

**Rows that close:** on A merged — `M8/askpass-refusal-wording`, `M8/20-item14-words`, `M7/r30p-q1`,
`M8/18c-file-url-remote-words`. On B merged — `M8/evil-merge-not-checked-words`, `M8/journal-refused-covers-failure`
(R4–R6 have no rows; 11f's NOTES questions close with B). **Ruled now, no code** — `M8/15b-deviations`,
`M8/15-u-hidden-shell-begun-merge`. **Stays open:** `M7/30s-raw-fail`, `M8/aborted-refs-moved`.

## Signatures

```rust
// Part A — graft_git::run::askpass (pub). `close(self) -> io::Result<()>` is unchanged: its refusal error now
// carries this payload; `io::Error::new(ErrorKind::PermissionDenied, Refused { .. })`.
#[derive(Debug)]
pub struct Refused { /* first refusal + count, private */ }
impl Refused { pub fn count(&self) -> usize; }
impl std::fmt::Display for Refused {}   // "<first>; <n> refused in all" — no "askpass: " prefix
impl std::error::Error for Refused {}
pub fn refused(error: &std::io::Error) -> Option<&Refused>;   // get_ref()?.downcast_ref()

// Part A — graft_ops::apply (pub)
pub fn askpass_words(error: &std::io::Error) -> String;   // R1 refusal words, else today's cleanup words
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawStepOf { SetAside, LostEdit }
// AfterState::NotAsTheyWere { error: RestoreError, files: usize }  gains  `of: RawStepOf`

// Part B — graft (crate-private)
pub(crate) fn tab_keys(tabs: u8, keys: &Keymap) -> Option<String>;  // was literal "⇥ ⇥ banner"; None: tabs 0 or app.next_pane unbound
pub(crate) struct RefusedAt { pub ctx: ContextId, pub on_conflict_row: bool, pub tabs: u8 }  // tabs = refusal_tabs(state)

// Part B — graft-ui
pub const MARK_RESOLVED: &str = "mark resolved";   // graph_screen hints; graft's resolve_words uses it
// journal_screen::RowMark::Refused → RowMark::NothingWritten, label "nothing written"
```
If `apply` is not a pub path in graft-ops, put `askpass_words`/`RawStepOf` beside `AfterState`'s pub path and name it
in NOTES. `tab_keys` callers that pass `Keymap::shipped()` today (the commit editor's row, W4) keep passing it.

## Part A — graft-git, graft-ops

**Check first** (record each with file:line in NOTES; any surprise → stop, report, no fix):
1. Which error `close` returns when cleanup **and** a refusal both fail. If one hides the other: question (rule 3), not fixed.
2. `rg -n 'no remote named' crates` — exactly one producer (`remote::short`). More → stop.
3. `rg -n 'NotAsTheyWere' crates/graft crates/graft-ops`: every constructor and pattern. A graft pattern without `..`
   gets `..` (the only graft edit allowed; list the lines; none may be in 19b's files).

Acceptance (each tick names its test at tick time):
- [ ] R1 graft-git: a peer of another uid (or the existing refusal test's fixture) → `close`'s `Err` has
  `refused(&e).is_some()` and `count() == n`. Test: `askpass::tests::a_refusal_is_typed` (or the existing refusal test, extended).
- [ ] R1 graft-ops: a remote op whose askpass refused → its `AfterState` words read `another user's program asked for
  your login and was turned away · nothing was sent: …`, never `could not be cleaned up`; a cleanup failure still reads
  the old words. Test: `error::tests::askpass_refusal_has_its_own_words` (both arms, red first).
- [ ] R2: lost-edit put-back, raw step blocked → words end `: fix that, then u and put it back again`; plural form for
  two paths; with `app.undo` unbound (a `KeyNames` returning `None`) → today's words byte-for-byte. Set-aside put-back
  → today's words. Tests: `recover_bytes::a_failed_raw_step_names_the_route` (it) + `words::tests::the_route_clause_drops_when_undo_is_unbound`.
  The existing route test stays green unedited.
- [ ] R3: missing `file://` bare → `CloneError::explain` reads `clone failed: no repository at '<url>', or no access to
  it`; a fetch from a remote whose URL is a missing path reads the same tail. Tests: `tests/it/clone.rs::a_failed_clone_leaves_no_folder`
  (assert the words) + one `remote::tests` case. `tests/it/remote_ops.rs` updated only where it quotes the old words (list them).
- [ ] MAP.md: graft-git `run/askpass` (`Refused`, `refused`), graft-ops `AfterState::AskPass`, `NotAsTheyWere` (`of`, route clause), `## clone` explain.

## Part B — graft, graft-ui

**Check first:**
1. `graft-ui/src/detail.rs:115-117`: `EvilResolved { files }` is produced only when every non-hand-resolved file was
   checked with no finding. If not → R7 stops; report with the line.
2. `rg -n '⇥ banner|"⇥' crates/graft/src`: every literal banner clause. Each goes through `tab_keys`, or NOTES says why not.
3. What the resolver's `u` notice says while live (graft MAP :505, `live_refusal_words`). If it names `⇥`: question, not
   fixed here (the resolver is modal; its way out is `Esc`).
4. `rg -n 'refused · nothing written|space resolved|marks resolved|evil merge: not checked|hand-resolved' crates docs TESTING.md`
   → the full list of tests, captures and docs to update, in NOTES.

Acceptance:
- [ ] B1 R4: live merge, 1 conflicted file, changes pane, cursor off the CONFLICTS rows: `c` → status `1 file still
  conflicted · ⇥ ⇥ banner`; pressing `next_pane` twice focuses the banner (driven, not asserted by count). From the diff:
  `· ⇥ banner`, one press. On a CONFLICTS row: unchanged. Test: `keymap_words::resolve::off_a_conflict_row_the_refusal_names_the_tabs_that_reach_the_banner`.
- [ ] B2 R5: same fixture, `u` from the file list → `finish or abort the merge first · ⇥ ⇥ banner`; from the graph →
  `· ⇥ banner` (unchanged); `U` likewise; the `undone` `OperationLive` arm likewise. Test:
  `keymap_words::undo_journal::u_refused_from_the_changes_pane_names_two_tabs`.
- [ ] B3 R5 remap: `[keys.app] next_pane = "ctrl+n"` → the clauses read `⌃N ⌃N banner`; `next_pane = "none"` → reason
  alone. Test: `keymap_words::resolve::the_banner_clause_is_spelled_from_his_keymap`.
- [ ] B4 R6: CONFLICTS row bar draws `space mark resolved`; the `c` refusal there ends `space mark resolved`; both
  read `MARK_RESOLVED`. Tests: graft-ui `hints::tests::a_conflict_row_says_mark_resolved` + the existing
  `keymap_words::resolve` pin updated (quote the line).
- [ ] B5 R7: hand-resolved merge with no finding → `evil merge: none found · 2 hand-resolved files not checked`;
  1 file singular; narrow → `evil merge: none found · 2 not checked`; all checked clean → `evil merge: clean`.
  Test: `detail_pane::tests::a_hand_resolved_merge_says_not_checked_never_clean` (renamed/extended; red first).
  TESTING.md radar stop 10's quote updated to the new words.
- [ ] B6 R1 (after A merged): the clone's askpass notice uses `graft_ops::apply::askpass_words`. Test:
  `jobs::clone::tests::a_refused_askpass_notice_uses_the_ops_words` (or NOTES: the notice has no test seam, with the line).
- [ ] B7 R8: a refused push and a git failure that wrote nothing both read `nothing written`; no row reads `refused`.
  Test: `view::journal::tests::a_refused_push_reads_refused_nothing_written_not_undone` renamed
  `…_reads_nothing_written_not_undone`, plus a git-failure case. J's legend checked (`LEGEND`).
- [ ] Sweeps green unedited: `every_bound_action_is_hinted_live_or_is_named_here`,
  `no_key_a_live_operation_bar_draws_refuses_while_live`, `no_literal_keys::no_user_facing_literal_spells_a_default_key`
  (delete any row for the old `⇥` literal; say which).
- [ ] Captures whose text changed re-shot (from check first 4's list), listed in NOTES; each `docs/shots/` file named.
  Known: `docs/shots/m6/fix-finish-shell-merge-120x40.txt` (evil-merge line). By eye: user; colour unverified.
- [ ] MAP.md: graft `resolve_keys`, `RefusedAt`, `tab_keys`, `in_progress_refusal`, `undone`; graft-ui `RadarLine`,
  `journal_screen` `RowMark`, `MARK_RESOLVED`.

## Touches (both parts list each with how it was checked)
- **Other screens using the same words:** graph `OperationLive` bar `⇥ banner` (unchanged, `TabWord`/`tab_to_banner`);
  commit editor refusal and `operation_refusal` (now through the spelled `tab_keys`, shipped keys kept); resolver `u`
  notice (check B3); §2.6a prose ("`Space` marks it resolved"); Explain's blind-spot text (`hand-resolved`); every
  remote op's `NoSuchRemote` words (fetch, pull, push, clone); J rows and legend.
- **Captures:** grep list from B check 4 and A's `no remote named` grep; A changes no capture unless a shot shows a
  remote failure (list it).
- **Sweeps / remap tests:** `keymap_words::{resolve, undo_journal, stops}`, the grep ratchet, coverage slots
  (unchanged — no new `HintContext`).
- **Journal/undo:** words only; no entry shape changes (`RowMark` is a view type).
- **Docs:** TESTING.md radar stop 10 (B). **Conductor**, after B: `02-keymap.md` §2.6a :391 — `· ⇥ banner` "with one
  `⇥` per pane between him and the banner, spelled from `app.next_pane`"; closed rows to `docs/gaps/closed.md`.
- **Other crates:** graft-core untouched; graft-ui `palette*`/`empty_state*` untouched (19b).

## Not ruled here
- Resolver `u` notice while a shell-begun merge would let graft's `u` act (graft MAP :505 vs :440): a question for the
  usability walk; no source line checked.

## For the user at the stop
- None. (Re-shot captures go on the M8 contact sheet for his eye, as usual.)

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Part A ~45k tokens. Part B ~65k tokens. One context each.
