# land-on-result — every write that moves HEAD lands the cursor on the new HEAD; a fast-forward pull sets edits aside

Status: ready (respecified 2026-09-26 against `scouts/03.md`, HEAD ed624165)
Blocked by: none
Crates: graft (`update/operate.rs` `launch`/`operated`, `update/banner/stop.rs`, `update/remote/in_flight.rs`, the test
harness's `settled`); graft-ops (`operation.rs` `moves_head`/`sets_aside`/`touches`/journal record, facts, `apply.rs`
Pull path, `apply/set_aside.rs`).
Source: KNOWN_GAPS :773, :805; ADR-0045 Consequences :132. Note "Pass 2a" R7, "Where I'd differ" 4.
Build with: Opus. One landing rule wired into an existing mechanism, and an ADR-0045 extension whose one ordering
choice is decided below; both are judged by real-repo tests, neither is structural beyond Pull → not Fable.
Scout: done (`.scratch/m7-power-tools/scouts/03.md`). No second scout.
Needs: none. `rewrite-stops` and `branch-picker` need `Operation::moves_head` and the Done landing.

## What changed from the first spec
| First spec said | Code says (scout) | Now |
|---|---|---|
| `land_on_result(state, kind, new_head)` | `land_on_head` reads HEAD from refreshed refs (`stop.rs:148`); `resumed_done` has no kind (`banner.rs:877-891`) | **No new landing function, no oid.** Both paths call the existing `finished_lands_on_head` (`stop.rs:90-104`). Whether to land is decided at **launch** from `Operation::moves_head()`, carried to Done. The note's `land_on_result` line is superseded (conductor: note/interfaces). |
| Done lands "as `--continue` does" | Done never touches the cursor (`operate.rs:897-905`) | Done arms the same landing when the launch said so. |
| `sets_aside` true "when the pulled paths overlap" | no overlap test anywhere; Rebase/Checkout: `Some` on any changed tracked file (`operation.rs:1086-1105`); `RepoFacts` has no incoming paths; the paths are unknown until the fetch runs | Pull's **fetch runs before the facts read** that decides; `RepoFacts` gains `incoming`; `sets_aside` for Pull tests overlap. Rebase/Checkout unchanged. |
| preview line `your edits: N files set aside, put back after` | `p` launches directly (`remote.rs:348-368`); no Pull preview | **No preview line.** Said after, on the finish line. A Pull preview is a new step → Proposed change below, not built. |
| walk: `p`, then `y` if previewed | — | `p` only: 1 key. |

Kept: the kind list's intent (UX §3.1), undo/redo/Recover excluded, the four graft-ops tests, one entry, clash → resolver.
Added (forced): the fetch-before-decide order, a characterisation test pinning a failed fetch's journal, the harness
`settled` waiting on an owed landing (more walks now owe one), the finish-line words for Pull.

## Decided
- **The landing rule** (UX §3.1): a **finished** write whose `moves_head()` is true lands the cursor on the new HEAD's
  row, detail pane following, whatever row it was on.
  - `moves_head()` true: Commit, Amend, Merge, Rebase, Rewrite, CherryPick, Revert, Pull, Checkout,
    Reset with `ResetTarget::Head`, BranchFromStash, BranchCreate with `checkout: true` (it switches HEAD, as Checkout).
  - False: everything else, including Reset `ResetTarget::Branch` (`branch -f`, HEAD stays), Fetch, Push, Stash*,
    Tag*, BranchDelete, Recover (journal-screen owns its landing), and undo/redo (not `Operation`s; their own landing).
- **Mechanism:** `operate::launch` stores `op.moves_head()` in the value held by `AppState::operating`. `operated`'s
  Done arm, when it is true, calls `finished_lands_on_head(state, &commands)` with the refresh's commands, exactly as
  `resumed_done` does. The existing refs+walk wait and `ShowRows` (`stop.rs:109-170`) do the rest; nothing blocks.
  - If the refresh a kind issues has no `ReadHistory`, that kind silently won't land: each kind's test catches it.
  - Known quirk, not fixed here: the walk half is spent by **any** started walk (`stop.rs:130`). Report if a test hits it.
- **Stop / Conflicted / Clashed / Failed:** unchanged. Status words unchanged except the Pull clause below.
- **Fast-forward pull over edits git would refuse** (ADR-0045 §4 extended):
  - **Order (the one hard choice):** `apply` runs Pull's fetch invocation first, **then** reads facts (with remotes,
    and `incoming`), then check → `sets_aside` → ADR-0045 §4 steps 1–6 with the plan = `merge --ff-only --no-edit
    <tracking ref>` alone. Why: the overlap is unknowable before the fetch, and §4 needs `S` on the Pending line
    before the store; opening the entry after the fetch keeps §4's order and the journal format untouched. Rejected:
    set aside on any changed file (a daily clean pull would silently lose his staging, with no preview to warn);
    retry after git's refusal (two entries per key); a second Pending line mid-entry (journal format change).
  - **The fetch's journal record stays as today.** First write `a_pull_whose_fetch_fails_journals_as_today` and
    `a_pull_journals_as_today` against the unchanged code (green), and keep them green. If today's entry opens before
    the fetch, keep a failed fetch's record identical (open-and-fail with no set-aside). Say in NOTES which it was.
  - `Some` iff: `incoming` is `Some`, `fast_forward`, and `paths` ∩ changed tracked paths (staged or unstaged, a
    rename's original path included) ≠ ∅. `SetAside.files` = **all** changed tracked files (§4 stashes all), as Rebase.
  - Not a fast-forward (diverged), up to date, or no overlap → `None`: today's behaviour and words, staging kept.
  - Untracked files are never set aside (§4); git's own refusal over one stays (§4 "fails at 5": put back, its words).
  - `touches` and the journal record add `refs/stash` when set aside, as Rebase's arm (`operation.rs:1464`).
  - `incoming` is read only for Pull, only when tracked files are changed: `merge-base --is-ancestor` + `diff
    --name-only -z --no-renames HEAD <tip>` (2 spawns, `stop` checked between). Clean tree: 0 spawns.
- **Where he lands** (the note's table, Pull rows):
  - Finish, set aside: the graph, `▸` on the new HEAD, `pulled origin/main · your edits are back`; when
    `SetAside.staged` > 0, `… · N no longer staged` (the preview's staged warning, said after). Width: the
    `your edits are back` clause is the part kept whole (`fit_status_line`).
  - Put-back clash: the resolver (`clash::clashed`, as `operated`'s Clashed arm `operate.rs:956`).
  - Failure after the store: the ADR-0045 §5-amended `EditsAt` clause (`your edits are in stash@{n} · S p …` etc.).
  - Journey: pull over overlapping edits = `p`, **1 key**, 0 decisions (today: refused → the shell; a "Times I left"
    row). No best-tool count: the prior-art study has no pull row — say so, don't invent one.

## Proposed change (for the user; not built)
- **Give `p` a preview only when edits would be set aside:** `p` → `your edits: N files set aside, put back after`,
  the §8 put-back forecast, the staged warning → `y`. Buys: M5's "see it before you confirm" for the one pull that
  rewrites his tree. Costs: +1 key on that pull; the preview must fetch first (a remote spawn inside a preview, new);
  a new screen state. Until approved: said after (above).

## Read first
- This spec. `.scratch/m7-power-tools/scouts/03.md` (all).
- Design note `docs/design-notes/m7-power-tools.md`: "Where he lands" table (:94-105); R7 rows :764-765; "Where I'd
  differ" 4 (:990-992).
- `docs/research/ux-principles.md` §3.1.
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §4, §5, §6, §7, and the 2026-09-26 amendment (§5, §8).
- `crates/graft-ops/MAP.md` `operation`: `SetAside`, `sets_aside`, `RepoFacts`, `plan` (pull line), `touches`,
  `Invocation.remote`; `read_facts`; the `apply` set-aside wrap.
- `crates/graft/MAP.md`: `AppState::operating`, `land_on_head`/`head_landing`; `update::operate`; `update::remote`
  (`pull_key`, `remote_done`, `remote_failed`); `edits_back`.
- Then source: `operate.rs:880-960`, `banner/stop.rs:80-170`, `banner.rs:870-895`, `remote.rs:340-370`,
  `remote/in_flight.rs`, `graft-ops/src/apply.rs:220-365`, `apply/set_aside.rs:60-120`, `operation.rs:1080-1110`,
  `:1455-1470`.

## Signatures
```rust
// graft-ops
impl Operation {
    /// True when finishing this write moves HEAD (its ref or its commit). A table test covers every variant.
    pub fn moves_head(&self) -> bool;
    // unchanged signature; gains the Pull arm above:
    // pub fn sets_aside(&self, facts: &RepoFacts) -> Option<SetAside>;
}
pub struct Incoming { pub tip: Oid, pub fast_forward: bool, pub paths: Vec<PathBuf> }
// RepoFacts gains `pub incoming: Option<Incoming>` — Some only for Pull, read after its fetch, only over changed tracked files.
pub(crate) fn read_incoming(repo: &Repo, git: &Git, head: Oid, tip: Oid, stop: &dyn Stop)
    -> Result<Incoming, FactsError>;

// graft (app)
// The value held in `AppState::operating` gains `lands_on_head: bool`, set in `operate::launch` from `op.moves_head()`.
// `finished_lands_on_head` (banner/stop.rs:90) — existing, signature unchanged; now also called from `operated`'s Done.
```
If `AppState::operating` cannot carry the flag to the arm that handles Pull's Done (it goes through `remote_done`),
carry it the same way the arm gets the operation's words; say how in NOTES. Never land by guessing the kind.

## Acceptance (each tick names its test)
- [ ] **graft-ops:**
  - `moves_head_covers_every_operation` (table, every variant, both Reset targets, BranchCreate both ways);
  - `a_pull_journals_as_today`, `a_pull_whose_fetch_fails_journals_as_today` (written first, green before and after);
  - `a_ff_pull_over_overlapping_edits_sets_them_aside` (red first, :805);
  - `a_ff_pull_whose_put_back_clashes_stops_as_a_clash`;
  - `a_ff_pull_over_unrelated_edits_is_unchanged` (no stash created, index byte-equal: staging kept);
  - `a_diverged_pull_over_edits_sets_nothing_aside` (today's refusal words);
  - `the_ff_pull_entry_is_one_entry_and_u_restores_edits` (staged split back via the backup);
  - `the_ff_pull_set_aside_round_trips` on ADR-0045 §7's tree table, and a stop before each step 1–6 leaves the
    edits in the tree or the stash list (reads both).
- [ ] **App:**
  - `after_m_m_the_cursor_is_on_the_merge` (red first, :773);
  - `after_a_commit_the_cursor_is_on_it`; `after_a_cherry_pick_the_cursor_is_on_the_new_commit`;
    `after_a_checkout_the_cursor_is_on_the_new_head`; `after_a_pull_the_cursor_is_on_the_new_head`;
  - `a_branch_reset_leaves_the_cursor`; `a_fetch_leaves_the_cursor`;
  - `landing_waits_for_an_unlaid_row`; `the_detail_pane_follows_the_landing`; `undo_keeps_its_own_landing`;
  - `a_ff_pull_clash_lands_on_the_resolver`; `a_ff_pull_over_edits_says_they_are_back` (both clauses, 80 cols);
  - `settled_waits_for_an_owed_landing` (harness).
- [ ] **Walk** `pull_ff_over_edits` (m7 harness): `p` (asserts **1 key**); edits back byte-equal (unstaged), HEAD
      fast-forwarded, `▸` on the new HEAD, the status line above.
- [ ] **Captures:** new `docs/shots/m7/landed-after-merge-120x40.txt` — **right:** `▸` on the merge commit's row,
      detail pane on it, the status line in today's words. New `docs/shots/m7/pull-over-edits-120x40.txt` — **right:**
      `▸` on the pulled tip's row, `pulled origin/main · your edits are back`. Changed ones re-taken (Touches).
- [ ] MAP.md (graft, graft-ops). TESTING.md rows for the walk and both captures. ADR-0045 amendment text drafted in
      NOTES for the conductor (§4 kinds + Pull's order; Consequences :132 withdrawn).

## Touches (check each; report how)
- **Captures with `▸` (212 files, scout Q5):** only a capture taken after a finished `moves_head` write with the
  cursor elsewhere can move. Check: (1) before any change, re-take every capture on the base (`GRAFT_UPDATE_SHOTS=1`,
  then a pass without it, all `settled`) — any diff there is pre-existing: stop and report it, don't mix it in;
  (2) after, re-take the same way; (3) script the classification of `git diff docs/shots` into
  `.scratch/m7-power-tools/03-shots.txt`: per file, "cursor row + detail pane only" or "other". Any "other" → stop,
  report. Read the file's summary lines, not the diffs (context guard). NOTES lists each moved capture.
- **Walks that wait on `!land_on_head`** (`m5_walk/run.rs:608`, `conflict_interim/resume_words.rs:29`,
  `landing_shots.rs:199`, `resolver/shell_begun.rs:346,571,822`, `finish.rs:336`): unchanged meaning; green.
  **Other walks** now owe a landing after writes: `settled` must be false while one is owed (test above).
- **`done_lands_on_the_merge_commit`** (`it/finish.rs:328`), `banner/tests.rs:1788`: the continue path; green.
- **The commit flow:** after `c` the cursor leaves the working-tree row (`land_on_head` clears `on_working_tree`,
  `stop.rs:143-170`). Note-decided (R7). Check: the m3 commit walks; list any capture that moved in NOTES.
- **The pull/push `in_flight` machine** (`Esc` during the fetch, `cancelled`, `WouldOverwrite` words
  `in_flight.rs:205`): Esc during the fetch leaves no set-aside. Check: m4/m6 pull tests green; a cancelled-fetch test.
- **Set-aside (ADR-0045):** Rebase/Checkout/Recover/Rewrite unchanged. Check: every set-aside, stash-clash and
  `live_clash` test green.
- **Journal/undo:** Pull records `refs/stash` when set aside; the no-op-pull `undoable` rule (MAP graft-ops :106)
  unchanged. Check: the m4 undo tests; journal-screen (a later ticket) reads Pull rows — name any field change in NOTES.
- **`rewrite-stops`, `branch-picker`** call `moves_head`/the Done landing: signature fixed here.
- **Keymap:** none. **Design note:** `land_on_result` line and R7 :765's "preview's set-aside line" superseded
  (conductor). **Prior-art study:** no pull row (conductor, if wanted).

## Rules / Gates
As rewrite-engine. Workspace-wide gates. Captures ≤ 200 KB.

## Size
~70k. Build as two, in order (both hold graft + graft-ops, so not concurrent):
- **-a:** landing (`moves_head`, launch flag, Done arm, harness `settled`, captures re-take).
- **-b:** ff-pull set-aside (fetch order, `incoming`, `sets_aside` arm, journal, finish/clash/failure words, walk).

## User decisions (conductor, 2026-09-26)
- After `c`: cursor lands on the new commit (confirmed; the knock-on stands).
- Pull preview (Proposed change): **rejected**. `p` stays one key; no preview when edits are set aside. Build without it.
