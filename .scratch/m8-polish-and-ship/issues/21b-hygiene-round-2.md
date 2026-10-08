# G-hygiene 2 — rows routed to 21 since it closed

Status: ready (conductor 2026-10-08) — was draft (planner spec-only pass 2026-10-08; from KNOWN_GAPS rows routed `→ 21` / `| 21` and the 11f B2 open box)
Build with: per part, below. No Fable: no new public signature another ticket builds on; every item has a test or a
move-only check that judges it.
Needs: none from other open tickets. Part order and blockers per part.

Builders never read KNOWN_GAPS.md: each item quotes its row (Id, text, file:line as filed). **Lines moved since
filing** — find each site by the name quoted, say the line you found in NOTES. A row's claim that does not hold at
HEAD is a stop for that item: say what you found, with its line, and go on to the next item.

## Parts, crate sets, order

| Part | Crate set (disjoint files) | Build with | Blocked by | Size |
|---|---|---|---|---|
| **21b-1** hints family | graft-ui: `graph_screen/hints.rs`, `hints/fitting.rs`, `map_screen/render.rs`, `stash_screen.rs` (+ their `tests`), captures | Sonnet — every fix spelled out; the rest is move-only with a mechanical check | none | ~65k |
| **21b-2** blame, column, palette | graft-ui: `blame*`, `graph_column/*`, `palette.rs`, `context_menu.rs`, `hints/lists.rs`, `detail_pane/header.rs`, `history.rs`, new `whitespace_tail.rs`; graft: `update/place/blame_state.rs` + any `graft_ui::blame::whitespace_tail` import line | Sonnet — same | none (files disjoint from 21b-1) | ~55k |
| **21b-3** small crates | graft-git `run/stop.rs`, `run/tests/stop*`; graft-fixtures `remote/*`; graft-ops `snapshot*`, one probe test; graft-core one new test | **Opus** — timing, a fixture-timeout decision gated on a measurement, a probe whose red is a stop | none | ~40k |
| **21b-4** graft fixes + splits | graft: `event_loop/startup.rs`, `session.rs`, `empty_state.rs`, `update/palette/find.rs`, `update/palette/run.rs`, `view/tests.rs` (one test), `update/fold_count/tests.rs`, `tests/it/blame_place.rs`, `tests/it/palette_find.rs` | **Opus** — rule-3 guard on the hang-up path; a wrong narrowing re-opens 13g's exit 101 | **19b merged** (19b edits `empty_state.rs`, `startup.rs:189`, `exit_wait.rs:161`) | ~60k |
| **21b-5** graft big moves | graft: `update/undo.rs` (+ new children), `update/banner/resume.rs`, `update/tests.rs`, `update/tests/keymap_words/stops.rs` | Sonnet — move-only | 21b-4 merged; 20's remainder merged if it holds `update/undo.rs`; no open builder in graft `update/` | ~65k |
| **21b-6** allow-reason guard | every crate: `Cargo.toml` workspace lints + `#[allow` lines only | Sonnet — mechanical | all of 21b-1…5 merged; runs **alone** | ~30k |

Two at a time (user's cap): **wave 1** 21b-1 ‖ 21b-3 · **wave 2** 21b-2 ‖ 21b-4 · **wave 3** 21b-5 · **wave 4** 21b-6.
None runs beside 27 (rename sweep). All land before 28.

**Scout:** brownfield (ADR-0035). One scout per part for 21b-1, 21b-2, 21b-4, 21b-5 (≤1 page: the file's sections,
the seams the split will cut, which tests and captures pin each item). 21b-3 and 21b-6 need none: the rows name
the lines. The conductor may change Build with after a scout.

## Move-only check (every split item; 13l A1's method)
1. `cargo nextest list --workspace | sort` before and after: `diff` empty (NOTES: line count).
2. Line-set check: every non-blank line in the new files exists in the old file, except `use`/`mod`/`//!` lines,
   visibility widened to `pub(super)`/`pub(crate)` for a sibling, and `cargo fmt` re-wraps (NOTES lists them).
3. Public paths unchanged (re-exported); workspace compiles; MAP.md gains a layout line.
4. `git diff --stat <base> -- docs/shots` empty; no `.snap.new`; insta snapshots moved beside their tests if the
   tests move (11f B1's lesson).
5. `wc -l` before/after in NOTES. Target: **every file the item names ≤ 400 lines** (PROMPT.md 6.2, `:436`),
   inline tests moved to a `tests.rs` child first (11f B1). A file that can't get there on a clean seam: stop on
   that file, say the seam you'd need.
6. One commit per file split, nothing else in it.

---

## 21b-1 — hints family (graft-ui)

**Read first:** `crates/graft-ui/MAP.md` lines 18–25 (`HintKey`, hints files, `hints_for`), 444–448 (m8/11b, 11e,
11f B); `.scratch/m8-polish-and-ship/issues/11f-doorway-words-review-fixes.md` box "B2 one rule" (:268–275);
`docs/design/02-keymap.md` the `stash.*` rows (don't bind anything new).

**Items**
1. **M8/11f-b2-ready-join** — "11f B2 open: Map footer's Ready arm keeps its own join (graft-ui
   `map_screen/render.rs:155`); `place_footer_text` can't take notes — needs a helper in
   `graph_screen/hints/fitting.rs`. Also `stash_screen.rs` production 463 lines (see M8/doorway-files-over-400)."
   Fix: one join, in `fitting.rs`: `PlaceFooter::keys_text`; `place_footer_text` and the Map Ready arm both call it.
2. **M8/marked-row-no-ctrlp** — "`marked_list` (graft-ui `graph_screen/hints.rs:654–663`) inserts the constant
   `DOORWAY`. `spelled` (`:511–523`) drops it when `app.help` is unbound. The `?`→`⌃P` fallback (`doorway_hint`,
   `:488–492`) runs only on the trailer (`spelled_bar`, `:471–477`). So with `help = "none"`, a marked row shows no
   doorway." At HEAD: `marked_list` `:659`, `doorway_hint` `:488`, `spelled` `:511`. Fix: `marked_list` takes
   `keys` and inserts `doorway_hint(keys)` (spelled before ranking), so `⌃P keys` ranks where `? keys` did; both
   unbound → no doorway.
3. **M8/stash-footer-close-key** — "the stash footer says `esc close` only (`stash_screen.rs:222, :225`), though
   `S` closes it (`update/stash.rs:204`, `keymap/table.rs:432`). J says `J/esc close` (`journal_screen.rs:254`),
   Map says `M/esc back` (`map_screen/render.rs:164`). Fix: `[stash.close, app.back]`." Every stash footer state
   (Ready, Loading/Empty/Failed) → `S/esc close`. `close` vs `back` stays the usability walk's — not here.
4. **Splits** (M8/doorway-files-over-400, M8/keymap-file-sizes; at HEAD): `graph_screen/hints.rs` 888,
   `hints/fitting.rs` 697, `map_screen/render.rs` 468, `stash_screen.rs` 463. Do 1–3 first, then split. Seams from
   MAP: hints.rs → the list filters / `spelled` + doorway / `HintState`…`Hint`; fitting.rs → the ladder
   (`ladder`, `fit_list`, `hints_text`) / stop rows (`StopWording`, `stopped_row`, `fit_status_line`) / place
   footers (`shed_footer`, `PlaceFooter`, `fit_place_footer`, `place_footer_text`); render.rs → per-status footer
   vs rows; stash_screen.rs → view types vs widget. The scout confirms.

**Signatures**
```rust
// graft-ui src/graph_screen/hints/fitting.rs (or its place-footer child after the split)
impl PlaceFooter {
    /// The fitted clauses, then the doorway, joined "  " — the one join every place footer draws.
    pub(crate) fn keys_text(&self, clauses: &[String]) -> String;
}
pub(crate) fn place_footer_text(clauses: &[String], keys: &Keymap, width: usize) -> String; // unchanged; body = fit_place_footer(..).keys_text(clauses)
// graft-ui src/graph_screen/hints.rs (private)
fn marked_list(inner: &HintContext, divider: Divider, keys: &Keymap) -> Vec<Hint>; // was (inner, divider)
```
`map_screen/render.rs` private `ready_clauses`/`notes`/`footer_line` keep 11f's signatures.

**Acceptance**
- [x] 1 (`hints::fitting::tests::keys_text_is_place_footer_text`, `keys_text_joins_kept_clauses_then_the_doorway`; named Map/stash tests unedited, green): `rg -n 'join\(' crates/graft-ui/src/map_screen/` shows no footer join (NOTES quotes the output). Pass
      unedited: `footer_ends_with_the_keys_doorway` (Map), `footer_never_draws_a_fragment`,
      `the_doorway_ends_the_keys_and_the_notes_follow`; graft
      `the_map_loading_footer_draws_the_doorway_only_if_question_mark_answers`. New
      `hints::fitting::tests::keys_text_is_place_footer_text` (same clauses, keys, width → equal strings, at 3 widths
      incl. one that sheds). Captures: `git diff --stat <base> -- docs/shots` empty.
- [x] 2 (`hints::tests::a_marked_row_with_help_unbound_ends_ctrl_p_keys`, red on base): new `hints::tests::a_marked_row_with_help_unbound_ends_ctrl_p_keys` (`help = "none"` → the bar holds
      `⌃P keys` where `? keys` sat; `help` and `palette` both unbound → no doorway, nothing else changes). **Seen red
      on base** by assertion (NOTES: the failing line). `a_marked_row_keeps_undo_and_search_before_the_doorway`
      passes unedited. Shipped-keymap captures byte-identical.
- [x] 3 (`stash_screen::tests::every_stash_footer_names_both_close_keys`, red on base; captures re-shot — caption in graft stale, see NOTES): `stash_screen::tests` expectations updated to `S/esc close` in every state; new
      `stash_screen::tests::every_stash_footer_names_both_close_keys` (Ready, Loading, Empty, Failed; `stash.close`
      unbound → `esc close`; both unbound → no close clause). Seen red on base. Re-shoot, then a second clean pass
      with `GRAFT_UPDATE_SHOTS` unset: `docs/shots/m8/doorway-stash-120x36.txt`, `doorway-stash-60x20.txt`
      (nextest); `docs/shots/m3-06-stash-list-{80x24,160x48}.txt`
      (`python3 .scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py {80x24,160x48} stash-pop`); rebuild
      `scripts/contact-sheet.sh m3`. NOTES quotes each footer row. `git diff --stat` names only these + the sheet.
      Colour unverified (rule 5); the look is the user's (rule 6).
- [x] 4 (`nextest list` diff empty, 6069 = 6069; line-set check, NOTES): move-only check per file (above); each of the four ≤ 400.

**Touches:** `place_footer_text` callers (J `journal_screen.rs`, S, Map Loading) — via the named tests; every
graft test asserting a stash footer string (`rg -n 'esc close  \? keys' crates/` — list each, fix or say why it
isn't a stash footer); `assert_place_footer` users; `docs/design/01-view-mockups.md:392` stash footer (**not
edited** — 23's row M8/mockup-stash-footer; NOTES says the new words so 23 takes them); keymap: no binding change
(`stash.close` already `S`); the palette's HERE list at the stash (reads the keymap, not the footer — check one
test); MAP.md graft-ui (11f B line; new file layout).

---

## 21b-2 — blame, graph column, palette, context menu, lists (graft-ui + one graft file)

**Read first:** `crates/graft-ui/MAP.md` `## blame` (:48–65), `HistoryView` (:75), `palette` (:44), `context_menu`
(:9), `graph_column` (:8, `## graph_column`); `crates/graft-git/MAP.md:30` (`BlameOptions`);
`docs/reviews/m8-stop-and-look.md` m6, m7 (:137–147).

**Items**
1. **M8/whitespace-tail-home** — "`whitespace_tail` and `put_whitespace_tail` live in `blame.rs` but are used by
   `detail_pane/header.rs:350-351` and `history.rs:299-300`. Move them to `graft-ui/src/whitespace_tail.rs`."
   With the two constants. Callers import the new path; no re-export left in `blame`.
2. **M8/blame-ws-default-twice** — "graft-ui `blame.rs:232` literal `true` vs graft-git `blame.rs:77`
   `BlameOptions::default`." Fix (review m6): `BlameView` carries `default_ignored`, filled by graft
   (`update/place/blame_state.rs`, the only non-test `BlameView {` site) from
   `graft_git::BlameOptions::default().ignore_whitespace`. **Order:** add the field and the test with the literal
   still in place → see the test red by assertion → wire it.
3. **M8/11-allow-no-reason** — "`graft-ui/src/palette.rs:350` `#[allow(clippy::too_many_arguments)]` with no
   `reason`." Remove it by grouping the arguments into one struct if they are one concept (precedent:
   `running::Screen`, review m8-01 Major 2); else `reason = "…"`. NOTES says which.
4. **Splits** — M8/look-sizes-2: "graft-ui `blame.rs` 601, `graph_column/mod.rs` 443 (PROMPT 6.2). Split after the
   M1 follow-ups and m6/m7" (HEAD: 602, 444); M8/11-files-over-size: "`graft-ui/src/palette.rs` 430" (HEAD 430);
   M8/keymap-file-sizes: "`lists.rs`, … `context_menu.rs`" (HEAD: `hints/lists.rs` 593, `context_menu.rs` 870).
   Items 1–3 first, then the splits.

**Signatures**
```rust
// graft-ui src/whitespace_tail.rs (new; `pub mod whitespace_tail` in lib.rs)
pub fn whitespace_tail(ignored: bool, default_ignored: bool) -> Option<&'static str>; // moved, unchanged
// put_whitespace_tail and the two constants: moved, signatures and visibility unchanged
// graft-ui src/blame.rs
pub struct BlameView<'a> { /* …existing… */ pub whitespace_ignored: bool, pub default_ignored: bool }
```

**Acceptance**
- [x] 1 (`rg` empty; `history::tests::history_title_says_whitespace_ignored_while_w_is_on`, `detail_pane::tests::detail_title_says_whitespace_ignored_while_w_is_on`, `blame::tests::blame_title_names_whitespace_only_when_counted` unedited, green): `rg -n 'blame::(put_)?whitespace_tail' crates/` empty; the existing whitespace-tail tests (detail,
      history, blame) pass unedited but for their `use` lines; captures byte-identical.
- [x] 2 (`blame::tests::the_title_tail_follows_the_default_it_is_given`, red on base; `update::place::blame_state::tests::the_view_carries_blames_own_default`): new `blame::tests::the_title_tail_follows_the_default_it_is_given` (`default_ignored: false` +
      `whitespace_ignored: true` → `· whitespace ignored`; `false/false` → none). **Seen red by assertion** with the
      literal still in place (NOTES: the line). New graft `update::place::blame_state::tests::the_view_carries_blames_own_default`
      (`view.default_ignored == BlameOptions::default().ignore_whitespace`). `rg -n 'whitespace_tail\(.*true' crates/graft-ui/src/blame*` empty.
- [x] 3 (`rg` empty; `palette::tests` green unedited): `rg -n 'allow\(clippy::too_many_arguments\)' crates/graft-ui/src/palette*` shows none without a reason;
      `palette` tests green unedited.
- [x] 4 (`nextest list` diff = only the 2 new tests, 6082 → 6084; line-set check, NOTES): move-only check per file; each ≤ 400.

**Touches:** `BlameView` builders (graft `blame_state.rs`; graft-ui `blame/tests.rs`) — grep `BlameView {`;
detail pane and history title tails (17/17b captures `docs/shots/m8/polish-blame-whitespace-counted-120x36.txt`,
`polish-whitespace-*` — byte-identical); `detail_pane/header.rs` is also 20-remainder's (evil-merge words at
`:124–127`) — this item edits only its `use`/call lines, so the merge is textual; `tests/it/blame_place.rs` is
21b-4's (not here); MAP.md graft-ui `## blame`, `HistoryView`, lib.rs list, file layout.

---

## 21b-3 — graft-git, graft-fixtures, graft-ops, graft-core

**Read first:** `docs/reviews/m8-stop-and-look.md` m1, m2, m5, Q2 (:98–135, :188–192); `docs/reviews/m8-doorway-words.md`
Minor 6, Question 3 (:152–156, :182–186); `crates/graft-git/MAP.md:355`; `crates/graft-fixtures/MAP.md:44–45`;
`crates/graft-ops/MAP.md:115` (`in_force`), `:170` (snapshot); `docs/adr/0007-core-holds-plain-data-no-behaviour.md`;
`.scratch/m8-polish-and-ship/issues/13e-stop-review-fixes.md` Minor 8 (the `Reap` rule).

**Items**
1. **M8/stop-read-cadence** — "`next_read` is taken from the `now` before the read (graft-git `run/stop.rs:226,234`;
   `:262,266` in `after_git_ended`), so a slow `ps` makes the reads back-to-back. Test
   `a_slow_read_of_the_group_does_not_spend_gits_grace` (:507-532): about 6.3 s by reading, NOTES claim 2.3 s."
   HEAD: `:233–234`, `:265–266`, test `:507`. Fix: `next_read = Instant::now() + LIST_EVERY` taken **after** the
   read returns, both loops. Measure the test's time before and after (fast tier ≤ 10 s, PROMPT 7.1 `:532`).
2. **M8/13k-sleep-unguarded** — "`stop.rs:510` `sleep 600` holds no `Reap` guard, which breaks 13e's Minor 8 rule."
   Fix: hold a `Reap` guard as the other stop tests do.
3. **stop.rs size** (545 at HEAD, not on a row; same file): move its inline tests under `run/tests/stop/`; production
   ≤ 400, else split on `after_git_ended`.
4. **M8/idle-client-bound** — "`remote/tests.rs:534-537` allows 2 s, but the 1 s `READ_TIMEOUT` (`http.rs:42,383`)
   passes it alone, so the test doesn't pin `Drop`'s shutdown." HEAD: test `http_remote_drop_ends_an_idle_client`
   `remote/tests.rs:520`; `READ_TIMEOUT` `remote/http/mod.rs:46`, set `http/listener.rs:99`. Fix: bound `drop` at
   500 ms.
5. **M8/http-read-timeout** (question, decided here) — "does `shutdown` wake a blocked read on macOS? If it does, the
   1 s per-read timeout … buys nothing for teardown and could fail a slow client under load." **Ruling:** item 4's
   test answers it. Green at 500 ms → shutdown wakes the read → raise `READ_TIMEOUT` to 30 s (it then only bounds a
   hung client; `Drop` bounds teardown). Red at 500 ms → **stop on 4–5**, keep 1 s, report the time `drop` took.
6. **M8/snapshot-filter-blamed** — "`name_the_failing_file` (graft-ops `snapshot/raw.rs:197–228`) returns
   `SnapshotError::Filter` for any lone `checkout-index` failure, with text `{path}: its filter failed`
   (`snapshot.rs:139`). It also writes one Command Log line per file." HEAD: `raw.rs:220`, `snapshot.rs:139–140`.
   **Ruling:** rename the variant `CheckOut`, text `{path}: git could not check it out: {error}` — git's own stderr
   in `{error}` names a filter when one failed, so the words stop guessing. One Command Log line per probe **stays**:
   the Command Log lists every git run (that is its contract); no change.
7. **M8/aborted-refs-moved** (question → probe) — "`took_effect` counts an Aborted entry only when
   `worktree_before.is_some()` (graft-ops `journal.rs:165`), so a multi-remote fetch that moved tracking refs could
   read `nothing written` (`view/journal.rs:192–202`). Whether fetch/push entries carry `worktree_before` is
   unchecked." Probe only: from MAP, find an operation that can end Aborted after moving a ref (a fetch with one
   remote failing, if Graft fetches several; else one ref rejected). Write the test asserting the entry is
   `in_force`. **Green** → it closes the row. **Red** → stop on this item, no fix: NOTES gives the entry's
   recorded fields. `in_force` drives `next_undo` (`u`); changing it is the planner's, and the words are
   20-remainder's (`view/journal.rs`).
8. **M8/adr-0007-unguarded** — "ADR-0007 (graft-core exports no free fns) held only by review; broken once by a
   planner spec (11 :35). An independent check (fast test on graft-core exports) would hold it." Fix: a fast test
   that reads `graft-core/src/**/*.rs` (`CARGO_MANIFEST_DIR`) and fails on any top-level free fn
   (`^pub(\(crate\))? (const )?fn ` at column 0, outside `impl`). Its doc comment states what it can't see (methods
   with logic; fns in inline `mod` blocks). **Run the scan on base first:** a hit is a stop for this item (list
   them; the fix is 25's), not a change to graft-core.

**Signatures**
```rust
// graft-ops src/snapshot.rs
#[error("{path}: git could not check it out: {error}")]
CheckOut { /* the fields Filter has, unchanged */ },          // was Filter
// graft-fixtures src/remote/http/mod.rs
const READ_TIMEOUT: Duration = Duration::from_secs(30);         // only if item 5 is green
// graft-core, in its existing test layout
#[test] fn core_has_no_free_functions();
```

**Acceptance**
- [x] 1 (test `run::tests::stop::the_next_read_waits_a_full_interval_after_a_slow_read`; red on base in each loop
      separately; `a_slow_read…` 6.377 s → 4.384 s): new `run::tests::stop::the_next_read_waits_a_full_interval_after_a_slow_read` (a fake slow `pass`; records
      each read's start and end; every gap end→next start ≥ `LIST_EVERY`, both loops). Seen red on base. NOTES:
      `a_slow_read…` measured before and after (`cargo nextest run -E 'test(a_slow_read)'`, its time line), ≤ 10 s.
- [x] 2 (test `run::tests::stop::signals::a_slow_read_of_the_group_does_not_spend_gits_grace`, guard
      `run/tests/stop/signals.rs:128`; panic run → `pgrep` 0): the test holds a `Reap` guard (NOTES: line). Seen once: a `panic!` put before the stop, run, then
      `pgrep -f 'sleep 600' | wc -l` → 0; panic removed. `rg -n 'sleep 600' crates/graft-git` — each hit's guard line in NOTES.
- [x] 3 (`nextest list` diff + line-set check, NOTES; `stop.rs` 545 → 283): move-only check; `stop.rs` ≤ 400.
- [x] 4 (test `remote::tests::http_remote_drop_ends_an_idle_client`, 20/20 at 500 ms): `http_remote_drop_ends_an_idle_client` bound 500 ms, green 20/20 in a loop (`for i in $(seq 20)`, output to
      a file, NOTES the pass count). Or the stop of item 5.
- [x] 5 (green → 30 s; filter `remote::tests | credential_by_key | m4_walk | m5_walk | m8_walk::clone |
      jobs::clone::tests | remote_ops:: | ^clone::` 298/298; MAP graft-fixtures `:44`): if green: `READ_TIMEOUT` 30 s; `remote::tests` and graft's `credential_by_key`, `m4_walk` (nextest filter
      `HttpRemote` users from 13l A's Touches) green. MAP graft-fixtures `:44` line updated.
- [x] 6 (test `snapshot_raw::a_lone_checkout_failure_without_a_filter_is_not_blamed_on_one`, red on base; filter test
      `undo_bytes::a_failing_required_smudge_filter_refuses_the_operation_before_it_writes` green, asserts git's
      `external filter 'false' failed`; grep empty): new `snapshot_raw::a_lone_checkout_failure_without_a_filter_is_not_blamed_on_one` (no filter configured,
      a checkout failure forced — e.g. an unwritable scratch dir — → `CheckOut`, text has no `filter`). Seen red on
      base. Existing filter test green with the new text if its git stderr names the filter.
      `rg -n 'its filter failed|SnapshotError::Filter' crates/` empty.
- [x] 7 — **STOP, probe red** (`aborted_fetch::a_fetch_that_moved_refs_before_one_remote_failed_is_in_force`, not
      committed; fields in NOTES 21b-3 item 7). Row M8/aborted-refs-moved stays open: the probe test, green (row closes, name it) **or** the stop in NOTES.
- [x] 8 (test `graft-core tests::core_has_no_free_functions`, 0.010 s; red seen with `pub fn x() {}` in `raster.rs:42`): `core_has_no_free_functions` green; seen red once by adding a `pub fn x() {}` to a graft-core file
      (removed); runs in the fast tier < 1 s.

**Touches:** `run::stop` callers (`exec`, 13e–13k stop tests; heavy `--features heavy-tests` clippy clean, not
run); `HttpRemote` users (graft-ops `tests/it/{remote_ops,clone}.rs`, graft `credential_by_key`, `m4_walk/*`,
`m5_walk/fixture.rs`); `SnapshotError` matchers (`rg 'SnapshotError::' crates/` — only graft-ops at filing) and
how graft shows a snapshot error (one status-line test, if any, by text); journal/undo: item 7 changes nothing;
MAP.md graft-git, graft-fixtures, graft-ops `:170`.

---

## 21b-4 — graft: hang-up guard, startup/session/empty-state splits, four tests

**Read first:** `crates/graft/MAP.md` `## event_loop` (:970–1041, `Exit`, `hung_up`, `put_away`), `:21`, `:84`,
`:141` (session), `:283` (fold_count), `:294`; `docs/reviews/m8-stop-and-look.md` m3, m10, Q3; 
`docs/reviews/m8-doorway-words.md` Minor 4; `docs/design-notes/m8-polish-and-ship.md:145–150` (J26–J31);
`.scratch/m8-polish-and-ship/issues/13l-stop-and-look-review-fixes.md` NOTES Part B (:246–256, the hang-up tests).

**Items**
1. **M8/hangup-exemption-too-wide** — "`startup.rs:531` and `:482` turn any error into `Ok` on `Signal(SIGHUP)`.
   They should exempt only `EIO` / `input::hung_up` (rule 3)." HEAD: `:498`
   (`Err(_nowhere_to_go) if exit == Exit::Signal(SIGHUP)`), `:547` (`(Ok(Exit::Signal(SIGHUP)), Err(_nothing_left_to_restore))`).
   Fix: both guards call one predicate. **Stop rule:** if the real hang-up tests go red (a dead pty's write error
   is not `hung_up`), stop; NOTES gives the errno seen. Don't widen the predicate.
2. **M8/look-file-sizes (refresh)** — "`startup.rs` is now 801, not 704. Split after 18b." HEAD 817 (inline tests
   from `:670`). Split after item 1 (seams from MAP: `listen`, `outside`, `wind_down`/`finish_to`/`put_away`,
   `terminal_and_state`).
3. **M8/11-files-over-size** — "`graft/src/empty_state.rs` 441, `graft/src/session.rs` 510 lines (rule ~400); split
   on their seams." HEAD: `empty_state.rs` 838 (grew with 18b/18c), `session.rs` 510.
4. **M8/12b-outside-paste-duplicates-first-runnable** — "`empty_state.rs` `outside_paste` repeats
   `first_runnable`; one call once 12e's split lands." HEAD `outside_paste` `empty_state.rs:403`. Before item 3's split.
5. **M8/12f-find-rs-407** — "`update/palette/find.rs` 407 lines (~7 over); seam: move `landed` to `run.rs`." HEAD
   429; `landed` `:403–429`. Moving it alone leaves ~402: move one more self-contained fn if needed, name it.
   `run.rs` stays ≤ 400.
6. **M8/stash-ago-test-blind** — "`view/tests.rs:5257` asserts `super::ago` alone, so reverting `view/stash.rs:61`
   stays green. Drive `view::stash` instead." Test: one stash 10 s old → its row's age reads `now` (not `now ago`).
7. **M8/fold-refresh-test-blind** — "`update/fold_count/tests.rs:56-60` asserts only the absence of `CountFolded`,
   and also passes if `Ctrl+R` did nothing." Fix: also assert the refresh's own command in `commands`.
8. **M8/blame-place-ends-with** (question, decided) — "should `tests/it/blame_place.rs:160` go back from `contains`
   to `ends_with` now that the tail is off by default?" **Yes**: 17 loosened it only for the tail. If `ends_with`
   fails, stop on this item and quote the drawn title (a tail drawn by default is a defect, not a test to loosen).
9. **M8/12-walk-journey-numbers** — "`tests/it/palette_find.rs` header/walk docs say J26–J28, "J30
   seek-commit-words", "J31 file-history"; note has J29, J30." Per the note (`:145–150`): header `J26–J31`;
   `rerun-recent` J28→**J31**, `seek-commit-sha` J29→**J28**, `seek-commit-words` J30→**J29**, `file-history`
   J31→**J30** (HEAD `:1, :169, :261, :285, :303`). Doc comments only.
10. **M8/12-test-file-size** — "`tests/it/palette_find.rs` ~900 lines; split by journey." HEAD 331: already under.
    Close only: `wc -l` in NOTES.

**Signatures**
```rust
// graft src/event_loop/startup.rs (or the split child holding finish_to / wind_down)
/// A hang-up excuses a failed restore, or a line it can't print, only when the failure is the dead terminal's.
fn excused_by_hangup(exit: &Exit, error: &io::Error) -> bool; // matches!(exit, Exit::Signal(SIGHUP)) && input::hung_up(error)
```
Everything else: no signature change.

**Acceptance**
- [ ] 1: new `excused_by_hangup` table test — (SIGHUP, EIO) true, (SIGHUP, UnexpectedEof) true, (SIGHUP,
      PermissionDenied) false, (Quit, EIO) false. New `finish_to` test: SIGHUP + a non-EIO restore error → `Err`.
      Seen red on base. The `:803` test keeps an EIO `refused` (edit only its error kind if it was another; NOTES).
      `quit_waits::hangup::*` green, fast and `--features heavy-tests` run alone (13l B's commands; times in NOTES).
- [ ] 2, 3: move-only check per file; `startup.rs`, its children, `session.rs`, `empty_state.rs` and children ≤ 400.
- [ ] 4: `outside_paste` calls `first_runnable` once (NOTES quotes the call); the start-screen paste tests
      (`empty_state::tests`, `outside_repo::*`) green unedited.
- [ ] 5: move-only check; `find.rs`, `run.rs` ≤ 400.
- [ ] 6: new `view::tests::a_stash_ten_seconds_old_reads_now` through `view::stash`. **Seen red** by reverting the
      stash row to a private formatter (NOTES: the failing line), restored.
- [ ] 7: the test asserts the refresh's command (name it in NOTES). Seen red with `Ctrl+R`'s handler stubbed to do
      nothing, restored.
- [ ] 8: `blame_place.rs:160` uses `ends_with`; green. Or the stop.
- [ ] 9: `rg -n 'J2[6-9]|J3[01]' crates/graft/tests/it/palette_find.rs` matches the note's numbers (NOTES quotes it).
- [ ] 10: `wc -l` in NOTES.

**Touches:** hang-up path (`main` re-raise, `wind_down`, `exit_wait`, 13g capture `docs/shots/m8-hangup-relaunch-120x40.txt`
unchanged with `GRAFT_UPDATE_SHOTS` unset); 19b's tour launch in `startup`/`empty_state` (merged first — re-read
its NOTES file list); 18b/18c clone code in `empty_state.rs` (moves only); `Session` users (workspace compile);
palette `landed` callers; `view::stash` (20 item 7's `now`); fold-count refresh (17b capture
`docs/shots/m8/polish-fold-120x36.txt` unchanged); MAP.md graft `## event_loop`, session, empty_state, palette.

---

## 21b-5 — graft big moves (move-only)

**Read first:** `crates/graft/MAP.md` the `update::undo`, `update::banner`, `update::tests` rows (`rg -n 'undo|banner|update::tests' crates/graft/MAP.md`);
the scout's file. Don't read whole files: list `fn`/`mod`/`impl` lines and section comments, cut by ranges.

**Items** — M8/doorway-files-over-400: "`update/banner/resume.rs` 511 (less after 11f A), `update/undo.rs` 2050,
`update/tests.rs` 6836, `update/tests/keymap_words/stops.rs` 1142 (less after 11f A)". HEAD: `resume.rs` 493,
`undo.rs` 2050, `update/tests.rs` 7032, `stops.rs` 1038.
1. `update/undo.rs` → children under `update/undo/` by what each part does (the scout names the seams; `consequence.rs`
   and `tests/` exist already). Production files ≤ 400.
2. `update/banner/resume.rs` → ≤ 400 (one seam).
3. `update/tests.rs` → children under `update/tests/` by topic (the existing `keymap_*` children are the precedent);
   each new file ≤ 400 where a topic allows, never splitting one test fn; the parent keeps only shared fixtures.
4. `update/tests/keymap_words/stops.rs` → by stop kind, each ≤ 400.

**Signatures:** none change. `pub(super)` widening only where a sibling needs it.

**Acceptance**
- [ ] 1–4: the move-only check, per file, one commit each; NOTES the before/after `wc -l` table and the
      `nextest list` diff (empty). A topic that can't get under 400 without splitting a test: its size in NOTES.

**Touches:** every test path under `update::tests::*` is renamed by a move → `nextest list` is the check (paths
change, count must not: NOTES compares **counts** and test-fn names, since module paths move); filters that name a
path (`rg -n 'update::tests::' scripts/ .scratch/*/issues/28* TESTING.md docs/` — list each, fix scripts, leave
records); undo/journal behaviour: none (moves); 20-remainder's `undo.rs` edits merged first; MAP.md graft layout lines.

---

## 21b-6 — allow-reason guard (all crates, alone)

Why: PROMPT.md 6.2 (`:456`) — "any noisy lint allowed in one place, with a reason". The row
(**M8/11-allow-no-reason**) found one; a grep at filing shows more `#[allow(` with no `reason =` (e.g. graft-ops
`snapshot.rs:252`, graft-ui `map_screen/render.rs:233, :309`, graft `map/walk.rs:212`, `map/rows.rs:37, :97`,
`graft-capabilities/src/render_md.rs:66`, ~15 `wildcard_imports`, and every test module's
`#![allow(clippy::expect_used, clippy::unwrap_used)]`). A review-held rule broke once; a lint holds it.

**Scope:** add `allow_attributes_without_reason = "warn"` to `[workspace.lints.clippy]` (the gate's `-D warnings`
makes it an error). Then fix each hit with `reason = "…"`: a trailing `// …` comment becomes the reason; test
modules `reason = "a failing test should panic"`; a module-split prelude `reason = "a split of one module: its
parent is its prelude"` (graft's existing words). Remove an allow that no longer fires. No other edit.

**Acceptance**
- [ ] `cargo clippy --all-targets -- -D warnings` and with `--features heavy-tests`: clean. Seen red first: NOTES
      gives the hit count before the fixes (the lint's own output, to a file; count only).
- [ ] `git diff --stat` touches only `Cargo.toml` and `#[allow`/`#![allow`/`#[expect` lines
      (`git diff -U0 | rg '^[+-]' | rg -v 'allow|expect|reason|^(\+\+\+|---)'` empty, NOTES).

**Touches:** every crate's lint table inherits workspace lints (check each `[lints] workspace = true`); none of the
code; open builders (runs alone).

---

## Gates (every part)
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide. 21b-3 and 21b-4 also clippy with `--features heavy-tests`.

## Excluded (rows not taken, why)
- M8/waiting-line-again → 20. M8/journal-refused-covers-failure, M8/evil-merge-not-checked-words → 20-remainder.
- M8/captures-two-places, M8/doorway-marked-caption → 28. M8/adr-0060-wording, M8/mockup-stash-footer → 23.
- M8/doc-warnings (rustdoc in graft, graft-ui, graft-fixtures, graft-graph, graft-term) → 25/27 per its row.
- M8/13g-double-sighup → "keep" (a question about his shells). M8/j-footer-at-48 → usability walk.
- M8/hangup-first-test-blind (unverified): graft maps `EIO` to `Signal(SIGHUP)` on purpose (`running/turn.rs:249–250`);
  the second hang-up test (`hangup.rs:251`) proves the route. No fix without undoing that mapping.
- M7/r0120-m5 (`preview/rewrite.rs:194` `unwrap_or_default`): not routed here; at HEAD `:272` is a map miss
  (`todo.get(row)…full.get(..)`), not the failed read the row names — re-check before anyone files work on it.

## Questions for the stop
- None needing the user. `close` (S, J) vs `back` (Map) for the same toggle stays the usability walk's (row
  M8/stash-footer-close-key).

## NOTES — for the conductor (planner)
- Proposed gap row `M8/files-over-400-unrowed`: many more `.rs` files are over 400 lines than any row names — by
  eye from `rg -c '^' crates --glob '*.rs'` on 2026-10-08, roughly 100 non-test files (e.g. graft-ops
  `operation.rs` 1912, graft `jobs/runner.rs` 1457, `update/mod.rs` 1406, `view/rows.rs` 1203, graft-ui
  `text_input.rs` 1123) and roughly 80 test files (e.g. `view/tests.rs` 5261, `hints/tests.rs` 3092). Not counted
  exactly. 21b splits only the rowed files. → 25 to rank, **broken** (PROMPT 6.2).
- Rows 21b closes on green: M8/11-files-over-size, M8/11-allow-no-reason, M8/12-test-file-size,
  M8/12-walk-journey-numbers, M8/12b-outside-paste-duplicates-first-runnable, M8/adr-0007-unguarded,
  M8/marked-row-no-ctrlp, M8/stash-footer-close-key (code half), M8/stash-ago-test-blind,
  M8/snapshot-filter-blamed, M8/aborted-refs-moved (if green), M8/11f-b2-ready-join, M8/12f-find-rs-407,
  M8/stop-read-cadence, M8/13k-sleep-unguarded, M8/hangup-exemption-too-wide, M8/idle-client-bound,
  M8/blame-ws-default-twice, M8/whitespace-tail-home, M8/fold-refresh-test-blind, M8/look-sizes-2,
  M8/look-file-sizes (refresh), M8/http-read-timeout, M8/blame-place-ends-with, M8/doorway-files-over-400,
  M8/keymap-file-sizes (named files; its "graft-ops/graft files held by 10f–10i" part names none → fold into the
  proposed row).

## NOTES — 21b-1 (builder, Opus, 2026-10-08)

Scout: none — the four files' seams were read directly (`grep` of item lines); MAP's named seams held.

- **1** `rg -n 'join\(' crates/graft-ui/src/map_screen/` → no output. Map Ready arm: `let keys_part = fitted.keys_text(&clauses);` (`map_screen/render/footer.rs`). `place_footer_text` body = `fit_place_footer(..).keys_text(clauses)` (`fitting/place_footer.rs`). Captures for Map/journal: no diff.
- **2** Red on base (fix reverted to `list.insert(at, DOORWAY)`): panicked `hints/tests.rs:2661` (`text.ends_with("⌃P keys")`), bar ended `… H hide · f fetch` (no doorway). To pass, `keys` threaded through private `whole_bar`/`searching_bar`/`led_by`/`pane_focused_bar` (`hints_for` already had it), and `doorway_last` also moves `DOORWAY_BY_PALETTE`. Width note: with no doorway, 240 cells admit one more hint (`f fetch`), so the "nothing else changes" check runs at 400. `a_marked_row_keeps_undo_and_search_before_the_doorway` passes unedited.
- **3** Red on base (close group reverted to `["app.back"]`): panicked `stash_screen/tests.rs:360`, `rows: "⏎ apply  p pop  s save  d drop  b branch  space files  esc close  ? keys"`. Footer rows now: Ready `⏎ apply  p pop  s save  d drop  b branch  space files  S/esc close  ? keys` (60x20: `⏎ apply  p pop  s save  d drop  S/esc close  ? keys`); Loading/Empty/Failed `s save  S/esc close  ? keys`. Re-shot: `m8/doorway-stash-{120x36,60x20}`, `m3-06-stash-list-{80x24,160x48}` (pty walk, then a second clean pass: "settled"), **plus** `m3-stash-list.txt`, `m3-stash-empty.txt` (graft `view::tests::capture_m3_stash` — also stash footers, not in the ticket's list), `m3-contact-sheet.txt` (4 lines, footers only). Second clean nextest pass without `GRAFT_UPDATE_SHOTS`: green. **Stale captions, outside graft-ui (not edited):** `crates/graft/src/update/tests.rs:6964` still says `… space files  esc close  ? keys` (drawn into `doorway-stash-120x36.txt:7`); the m3 headers `m3-stash-list.txt:13`, `m3-stash-empty.txt:9` list `esc close` (written by graft `view/tests.rs`). Needs a graft builder. For 23: design `01-view-mockups.md:392` stash footer → `S/esc close`. Graft tests on `esc close  ? keys`: only journal (`J/esc close`) and the caption above — none assert a stash footer. Colour unverified (rule 5); look is the user's (rule 6).
- **4** `cargo nextest list --workspace | sort` before/after: 6069 lines, `diff` empty. Line-set check (script: every non-blank new line in the old file, less `use`/`mod`/`//!`, `pub(super)` widened): exceptions only `use` re-wraps, `fmt` re-wraps (`whole_bar(` signature, `NEEDS_A_COMMIT`, a test call), and `super::` → `super::super::` in `fitting/stop.rs`, `fitting/place_footer.rs` (one module deeper). Methods on `Hint` widened to `pub(super)` (`at`, `labels`, `rungs`, `floor_rung`); `footer_line` to `pub(super)`. `tests/no_literal_keys.rs` allow entry path `stash_screen.rs` → `stash_screen/view.rs` (the `"1 file"` literal moved). One commit per split.
  `wc -l` before → after: `hints.rs` 906 → 149 (+ `hint.rs` 323, `doorway.rs` 118, `filters.rs` 350); `fitting.rs` 723 → 316 (+ `stop.rs` 223, `place_footer.rs` 102, `tests.rs` 122); `map_screen/render.rs` 462 → 389 (+ `render/footer.rs` 84); `stash_screen.rs` 468 → 383 (+ `view.rs` 92). (Before = after items 1–3; at ticket HEAD 888/697/468/463.)

## NOTES — 21b-2 (builder, Opus, 2026-10-08)

Scout: none — sites found by `grep` (the names the ticket quotes held); seams read from each file's item outline.

- **1** Moved to `graft-ui/src/whitespace_tail.rs` (`pub mod` in lib.rs) with both constants and `put_whitespace_tail`; callers `blame.rs`, `history.rs:299`, `detail_pane/header.rs:351` import the new path; no re-export in `blame`. No direct unit test of the fn existed; the three title tests named in the tick cover it.
- **2** Red with the literal in place: panicked `blame/tests.rs:668` (title lacked `· whitespace ignored`). Wired: `blame.rs:236` `whitespace_tail(view.whitespace_ignored, view.default_ignored)`; graft `blame_state.rs:98` from `BlameOptions::default().ignore_whitespace`. The graft test is an inline `mod tests` in `blame_state.rs` (builds a `Failed` place itself; `place/tests.rs::ready_place` is private and outside the crate set). It cannot go red while the default is `true` — it pins the wiring, not the value.
- **3** Grouped, no reason needed: `draw_row(buf, slot: Rect, row, chosen, wide, theme)` — `x`, `y`, `inner` were one concept (the row's one-cell slot).
- **4** `wc -l` before → after: `blame.rs` 563 (after item 1) → 314 + `blame/line.rs` 262; `graph_column/mod.rs` 444 → 340 + `kinds.rs` 114 (`NodeKind`, `RowPaint`, `pub use`d); `palette.rs` 430 → 326 + `palette/row.rs` 120; `hints/lists.rs` 593 → 377 + `lists/live.rs` 100, `rows.rs` 92, `editors.rs` 51; `context_menu.rs` 870 → 387 + `context_menu/draw.rs` 53, `context_menu/tests.rs` 441 (inline tests moved, dedented). Line-set exceptions: `use` re-wraps only, plus one doc link in `kinds.rs:98` gained a path (`(super::GraphLine::ghost_lanes)`); `#[cfg(test)] use` lines in `blame.rs` (`trailer`) and `graph_column/mod.rs` (`Modifier`) so tests compile unedited; `context_menu/tests.rs` gained `use` lines. Widened: `line.rs` items `pub(super)`, `said_live_on_the_graph` `pub(crate)` (re-exported `pub(super)` from `lists`). One commit per split. Captures: `docs/shots` untouched (status clean after the gates).
- **Touches:** `BlameView {` builders — graft `blame_state.rs`, graft-ui `blame/tests.rs` (grep: only two); history/detail title tails via their tests; `detail_pane/header.rs` call line only; MAP.md graft-ui (lib list, `## blame`, HistoryView, layout line).

## NOTES — 21b-3 (builder, Opus, 2026-10-08)

Status: **done, with item 7 a stop**. No compaction. No scout (ticket: "21b-3 … need none: the rows name the lines").
Commits on the branch: f56ae578 (move tests), 8a90725f (items 1–2), 1a103f9d (split), 4d46de60 (4–5), 452d77de (6),
ef6d2dc0 (8), 8a059fe0 (revert of an over-wide sed in 4, test margin in 1). Gates workspace-wide: fmt, clippy
`--all-targets` (+ `--features heavy-tests`) clean; nextest 6063/6063 (12 skipped); doctests ok.

- **Lines found at HEAD.** Item 1: `stop.rs:233–234` (hooks_first), `:265–266` (after_git_ended) as the ticket said.
  Test `:507`. Item 4: test `remote/tests.rs:520`, bound `:534–537`; `READ_TIMEOUT` `remote/http/mod.rs:46`. Item 6:
  `raw.rs:220`, `snapshot.rs:139–140`. Item 7: `journal.rs:165` (`took_effect`). 20b A did not move any of them.
- **1.** Now `next_read = Instant::now() + LIST_EVERY` after `pass` returns: `run/stop.rs:239`,
  `run/stop/helpers.rs:40`. Test fakes `Members` (150 ms per read) and checks every gap ≥ `LIST_EVERY`: hooks_first
  (3 reads + an unreadable 4th → fallback) and `after_git_ended` called directly (reads before `until`). Red on base
  in each loop on its own (hooks_first fixed alone → after_git_ended still red, gap 99.9 ms). `a_slow_read…`
  (`-E 'test(a_slow_read_of_the_group)'`, Summary line): **6.377 s → 4.384 s**. New test 2.6 s.
  Cause, not symptom: the read cadence was timed from the read's start.
- **2.** Guard `run/tests/stop/signals.rs:128` (`Reap::hook`; the shell writes `$!` to `$PIDFILE`). `panic!` after
  the guard, run, `pgrep -f 'sleep 600' | wc -l` → 0 (0 before too); panic removed. Not seen: the same run without
  the guard (would leak a `sleep 600` I'd have to `pkill`, which could hit the concurrent builder's). `rg -n
  'sleep 600' crates/graft-git` hits and guards: `tests/stop.rs` :40,:50 → `stop_at_the_hook`'s `Reap`; :62 →
  `_hook` :64; :81, :174 → `stop_at_the_hook`; :103, :158 → `stop_once_stopped`; :114 → `hook` :116; :131 → `job`
  :134; :190 → `helper` :194; :215 `OwnSleep` (Drop kills+reaps); `signals.rs:118` → :128; `fixture.rs:97` is a
  comment. **Unverified, not new:** `:131`'s guard is taken after `merge(&m).run(..).expect(..)` — a failed run
  there would leave its `sleep 600`; out of scope, filed here.
- **3.** Two move-only commits. (a) inline tests → `run/tests/stop/signals.rs`: `nextest list` 6059 = 6059; the 4
  fn names identical, module path `run::stop::tests::` → `run::tests::stop::signals::` (unavoidable for a move
  out of the file). Changed lines: `use`s, `super::super::group::members` → `crate::run::group::members`,
  `group_stop`/`hooks_first` → `pub(super)`. (b) split on `after_git_ended` → `run/stop/helpers.rs`
  (`after_git_ended`, `Helpers`, `kill_reaped_group`, `signal_one`): line-set diff = only visibility
  (`pub(in crate::run)` / `pub(super)`), `use`, `mod`, `//!` lines; `nextest list` unchanged. `wc -l`: `stop.rs`
  545 → 409 (after 1's edits, before split) → **283**; `stop/helpers.rs` 142; `tests/stop.rs` 247 → 365;
  `tests/stop/signals.rs` 156. MAP graft-git file-layout lines added. `docs/shots` untouched; no `.snap.new`.
- **4–5.** 20/20 green at 500 ms with `READ_TIMEOUT` still 1 s (`target/drop-loop.txt`, `grep -c '1 passed'` → 20)
  → shutdown wakes the blocked read on macOS → `READ_TIMEOUT` 30 s. Users run green (298/298). Observed, not
  caused: `m5_walk::every_scenario::merge_resolved_with_the_keys` takes 15.2 s at 1 s *and* 15.3 s at 30 s — over
  the 10 s fast-tier bound before this ticket. → proposed row, unverified cause.
  Slip caught and reverted (8a059fe0): my `sed` also set the wedged-hook drop test (`remote/tests.rs:392`) to
  500 ms; back to 2 s, out of scope.
- **6.** Forced failure = a worktree tree naming a missing blob (`mktree --missing`, a partial clone's shape) under
  `core.autocrlf=true`; an unwritable scratch dir fails earlier (`TempIndex`), not at checkout. Red on base:
  `a.txt: its filter failed: … unable to read sha1 file of a.txt (3333…)`. Now `a.txt: git could not check it
  out: …`. The filter test's words carry git's first stderr line, `error: external filter 'false' failed 1` (the
  filter's *command*, not its name `broken`), asserted. `rg 'its filter failed|SnapshotError::Filter' crates/`
  empty; only `docs/reviews/m8-doorway-words.md` (history) still quotes it. `SnapshotError::` matchers outside
  graft-ops: none. Seen in the words: they include the scratch path (`--prefix=/var/folders/…`) — a reader sees
  a temp path; not changed (not in the ruling).
- **7. STOP — probe red.** Probe: `LocalRemote` (Linear), `commit_on_remote`, second remote `gone` =
  `file:///nonexistent/…`, `apply(Fetch { remote: None, prune: false })` → `Err`; `origin/main` **did** move to
  the teammate's tip; the entry is `Aborted` and `in_force` is **false**. Recorded fields: `kind: Fetch`,
  `status: Aborted { reason: "no repository at '/nonexistent/graft-21b-3.git', or no access to it" }`,
  `touches: { refs: [], index: false, worktree: false }`, `head_before: Branch(refs/heads/main)`, `head_after:
  Some(same)`, `refs_before: {}`, `refs_after: {}`, `index_*: None`, `worktree_before: None`, `worktree_after:
  None`, `backup_ref: None`, `commands: ["git fetch --progress --all"]`, `merge_*: None`, `config_*: {}`. So
  `took_effect` (`journal.rs:165`) fails on `worktree_before.is_some()` and would fail `changed_anything` too:
  a fetch records no refs at all (a Committed one doesn't either, `remote_ops.rs:172`). `u` is unaffected
  (`undoable_kind` skips Fetch). Reasoned, **not observed**: the journal screen would mark this row `refused ·
  nothing written` (`graft-ui/src/journal_screen/view.rs:42`) though a ref moved. Fix is the planner's; probe not
  committed (red); code is 50 lines, easy to re-create from the above.
- **8.** Scan on base: no hit (`rg '^pub(\(crate\))? (const )?fn ' crates/graft-core/src` exit 1). Test
  `crates/graft-core/src/tests.rs` (+ `mod tests;` in `lib.rs`): column-0 `pub`/`pub(..)` + `const|async|unsafe|
  extern` + `fn`; plus `the_scan_reads_each_form_of_a_public_fn` pinning the matcher. Red seen: `pub fn x() {}`
  appended to `raster.rs` → fails naming `raster.rs:42`; restored. 0.010 s.
- **Touches:** `run::stop` callers — `exec`'s `stop`/`kill` unchanged signatures, all 17 `run::tests::stop*` +
  `run::` 55 green, heavy clippy clean (not run); `HttpRemote` users — 298/298 above; `SnapshotError` — grep
  above, graft shows it only through `ApplyError::explain` (no graft test pins the text: grep `its filter`
  empty); journal/undo — nothing changed (item 7 stopped); MAP: graft-git (layout ×2), graft-fixtures `:44`,
  graft-ops `:170`, graft-core (guards). KNOWN_GAPS not touched (builders don't).
- **Proposed rows (conductor):** M8/aborted-fetch-not-in-force (**broken**, the probe above, planner's call);
  M8/m5-walk-merge-keys-15s (**unverified** cause: over fast-tier bound, unrelated to 21b-3); M8/sleep-guard-after-run
  (`tests/stop.rs:131`, **unverified** leak path); M8/checkout-words-carry-scratch-path (**unverified** whether a
  user ever sees it).

## 21b-4 additions (conductor 2026-10-08, from `docs/reviews/m8-palette-landing-stop.md` C1, M1–M3 — fixes as the review states them, no new decision)

- Item 5 correction: `update/palette/find.rs` was **429** lines at the split (not 407); at HEAD re-measure. Seam unchanged (move `landed` to `run.rs`).
- [ ] 5b (M2): `update/palette/listing.rs` (424 at `c893abaa`) ≤ 400 — seam: the `IN PROGRESS`/`HERE`/`EVERYWHERE` assembly (`listing.rs:177-216`) apart from spelling and filter. Move-only check; `wc -l` of every `update/palette/*.rs` production file quoted at tick, measured with `wc -l` on the committed file.
- [ ] 5c (M1): `ARCHITECTURE.md:160-165` drops the count: "the commands the loop carries out itself (the `not_a_job` arms in `jobs/runner.rs`): the six that hand the terminal over, plus `Retheme`, `CopyToClipboard`, `SaveLayout`, `FirstLaunchDone`"; `:194-197` says the six *hand over*, without "only". Check: `rg -n 'not_a_job' crates/graft/src/jobs/runner.rs` arm count quoted at tick equals the doc's list.
- [ ] 5d (M3): `update/palette/mod.rs:7-10` module doc → "an `IN PROGRESS` row runs as the banner's key (`banner_press`), any other through `dispatch_key`"; `event_loop/startup.rs:248-249` `Arrival::on_head` doc → "every arrival on a terminal — launch, open, clone, init; off a tty (`run`) none". Doc only; quote both lines at tick.
