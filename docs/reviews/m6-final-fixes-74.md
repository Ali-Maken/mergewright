# Review — m6-conflicts 74: the preview shows the set-aside, and every end lands where the note says

Range `3e2f7fbb..47eea813` (74 merged at `47eea813`; 83 at `44b6ded3` is inside the base). Reviewer: read-only,
detached worktree at `47eea813`, own `CARGO_TARGET_DIR`. Ticket
`.scratch/m6-conflicts/issues/74-set-aside-shows-in-the-preview-and-lands.md` (NOTES read), scout
`.scratch/m6-conflicts/scouts/set-aside-shows-in-the-preview-and-lands.md`, `docs/design-notes/m6-stash-put-back.md`
("Journeys", "Where each event lands him", "New UI"), ADR-0045 §4–§6.

**Counts: Critical 0 · Major 4 · Minor 8.**

## Gates and tiers (run one at a time, in the worktree)

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo nextest run --workspace` | 3448 passed, 1 skipped; **166 s** run (195 s wall with the build). Load average ~5 (a builder was running), so slower than the 50 s logged 2026-09-19 |
| `cargo test --doc --workspace` | green, 2 s |
| Heavy tier `cargo test --workspace --features heavy-tests -- --test-threads=2` | 3484 passed, 0 failed, 1 ignored; 421 s wall |
| Release perf `… --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1` | 13 passed, 73 s. Input-to-frame p99 7.9–9.2 ms, max 11.5 ms (budget 16 ms), across pixel/unicode, 50k refs and the 100k fixture. First paint 15.4–38.6 ms. Peak RSS in the budget runs 31–104 MB |
| Fuzz, benches | not run: the diff touches no fuzzed parser (`fuzz/fuzz_targets/*`) and no benched path (`row_render`, `hot_paths`) |

- 5 slowest fast tests: `diff_fixture::parse_unified_matches_numstat_for_every_shape` 8.7 s,
  `m5_walk::every_scenario::merge_resolved_with_the_keys` 8.1 s, `conflict_ops::whole_file_shapes_resolve_like_git`
  7.8 s, `undo_cancel::cancel_at_every_step_boundary_is_clean` 7.0 s,
  `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.6 s.
- Peak memory (fast suite, `/usr/bin/time -l`, largest process): 1.01 GB. `target/`: 3.9 GB.

## Ticks spot-checked

**22 checked, 22 held.**
- Acceptance tick 1: all 12 named tests exist where the tick says, and all were green in the full run (12/12).
- Tick 2, the walks: the 6 claims compared with the assertions in `tests/it/m6_stash_put_back/set_aside.rs`: `:117-121`,
  `:166-176`, `:226-232`, `:272-280`, `:336-342`, `:455-462` (6/6).
- Tick 3, captures: each of the 4 captures asserts its words before `check_shot` (`:549`, `:561-568`, `:578-583`)
  (4/4).

Step counts: the fence rows (`docs/design-notes/m6-conflicts.md` +5 rows) equal the note's `:40-44`, which are 3, 5, 3,
5 and 8. Each walk counts its presses through `Keys` and asserts `<= allowed(id)`. `u` is 1 (`UNDO_KEYS`, note `:47`).
All held.

## Landings (note "Where each event lands him") against the code

| Event | Code | Held? |
|---|---|---|
| Clean put-back → graph on HEAD, own line + `· your edits are back` | `update/operate.rs:848-861`, `update/clash.rs:378`; done capture shows the cursor on `main` and `Uncommitted changes (3 unstaged)`, which bears out the staged clause | yes (the note's `rebased` is `rebase`: gap row 830) |
| Put-back clash → resolver, notice `… esc, u undoes the rebase/switch too` | `update/clash.rs:239-245`, `:278-289`; landing capture | yes, except `esc u` itself, which is not walked in the app (m1) |
| Last file done → `every file done · your edits are back ✓` | `update/clash.rs:357-362`; walks `:329-333`, `:444-448` | yes (ticket overrides the note's `:52-53` stash words) |
| Stop with edits aside → P5, `S` row, `· edits aside` | `update/banner/stop.rs:347-376`; walk `:472-528` | yes (the words appear only at 187 columns: gap row 827) |
| `c` finishing → clean line or clash landing | `update/banner.rs:742-745`; walk `a_stopped_rebase_then_a_clash_is_eight_keys` | yes |
| `a` → aborted, edits back | `update/banner.rs:845-848` | unit test only (m2) |
| Other put-back failure → sticky `your edits are in stash@{n} · S p puts them back` | `jobs/operate.rs:324-360`, `update/operate.rs:987-1007` | **too broad**: M1 |
| Killed → `… · edits in stash@{n}` | `update/undo.rs:624-647` | yes (unit) |

## Major

### M1. `Failed::edits_in` says the edits are in the stash when they are already in the tree. Worse than gap row 829
- **Where:** `crates/graft/src/jobs/operate.rs:344-360` (`edits_kept_in`). It fires on **any** `ApplyError::Git` whose
  entry carries a `SetAside` put-back while `S` is still listed. `update/operate.rs:987-990` then **replaces** the
  error line with `edits_in_status`. The same happens in `update/banner.rs:746-750` → `:776-788`.
- **What's wrong:** graft-ops knows whether the edits came back, but that is lost before the app words the line. At
  least three paths reach `edits_in = Some` with the edits already in the working tree:
  1. `Kept::Git { back: true }`, where the apply was clean and only the drop failed: `graft-ops/src/apply/set_aside.rs:444`
     → `:288`. The entry's own words say `your edits are back, and the set-aside copy could not be dropped` (`:382-386`).
     The status line says the opposite.
  2. `abandon` after `stash store` succeeds and `reset --hard` fails or is stopped: `set_aside.rs:122-163`, called from
     `apply.rs:240-243`. The 73 review's probe saw this state: "4 `S` listed, tree as it was"
     (`docs/reviews/m6-final-fixes-73.md:23`).
  3. `Kept::Unlisted` when the operation itself had failed: `set_aside.rs:445` → `:288`, arm `(Some(failed), _)`.
     The app reads the list again, finds `S`, and says it is where the edits are.
- **Rule 3 side:** when `edits_in` is `Some`, the operation's own failure (a switch git refused) leaves the status
  line: the job's `error` string (`ApplyError`'s Display, `apply/error.rs:60`) is dropped at
  `update/operate.rs:987-990`. It survives in the entry's `Aborted` reason (`set_aside.rs:245-247`), which `J` shows,
  and the doc comment says so (`update/operate.rs:996-1001`). So it is moved, not swallowed. But when the switch
  itself failed, he is never told at the time. (ADR §5 row 4's `u restores the backup` is in `explain`, `:224-226`,
  but not in the Display the job sends, so it was missing from this line before 74 too. That is pre-existing.)
- **Why it matters:** "you cannot lose work" depends on him trusting where the app says his edits are. The line names
  `S p` as the fix, and that runs over a tree already holding the same edits. By reading git's `stash apply` rules, I
  expect git to refuse over the dirty files. I did not run it, so treat that part as a **question**. The row was filed
  "unverified, not reachable in a walk", but the paths above are there in the code: it is **broken**.
- **Fix:** decide it in graft-ops, where `back` is known. Carry `kept: Option<Kept { at: usize, back: bool }>` on the
  error, and drop the second list read in the job. App words:
  - `back: false`: the current line.
  - `back: true` / `Unlisted`: `your edits are back · a copy stays in stash@{n}`, with no `S p`.
  - `abandon`: `u restores the backup`, as ADR §5 row 4 says.
  - The operation's own failure: first, then the stash clause.

  Re-file row 829 as broken.

### M2. The box says `no conflicts expected` directly under a put-back that will clash
- **Where:** `graft-ops/src/preview/kinds.rs:351-376`: a checkout plan never sets `conflicts`, so it stays `Clean`
  from `blank()` (`preview.rs:379`). `update/preview.rs:1012` words that as `no conflicts expected`. The rebase forecast
  covers only the replay (`kinds.rs:166`). The fixture `set_aside.rs:288-301` is a switch whose put-back **does**
  clash (walked at `:306-345`).
- **What's wrong:** the new row `your edits: 1 file set aside, put back after` sits right above `no conflicts
  expected` (captures `set-aside-preview-*.txt`). Together they read as "your edits will come back cleanly", and then
  the resolver opens. I got this from the code and did not render the switch-clash box itself. To the conductor's
  question: the set-aside row promises nothing false, but the box as a whole does.
- **Why it matters:** the M5 promise is "you see the result before you confirm", and heuristic 5 is error prevention
  (`docs/research/ux-principles.md:55`). The design note calls the preview the prompt (`m6-stash-put-back.md:29`).
- **Fix (spec gap, for the conductor or planner):** forecast the put-back. `git merge-tree` of `S` against the landed
  tree, the same machinery the replay forecast already uses, fed into `ConflictForecast`, so the row reads e.g.
  `your edits: … put back after · may clash: base.txt`. Or, at minimum, scope the forecast line to the operation.

### M3. In a set-aside clash, `yours` names the side that discards his edits (design question)
- **Where:** `crates/graft-ui/src/conflict_resolver/names.rs:38` (ours `yours` for `SetAside`),
  `crates/graft/src/update/resolver/names.rs:76-80` (theirs `your edits`). The walk `set_aside.rs:441-460`: `o` (bar
  `o yours`) writes the teammate's `theirs\n` into G over his edit (assert `:460`, "yours (HEAD's) taken for G").
- **What's wrong:** the panes read `yours · main` | `set aside · your edits` (landing capture). Both sides say "your".
  After a rebase, the `yours` side can hold the teammate's content. Pressing `o yours` throws his uncommitted edit
  away. In the same journey, `o` meant `upstream` a moment earlier (rebase roles).
- **Why it matters:** heuristic 2 (`ux-principles.md:52`, "ours/theirs flip in rebase", use the user's words). It is a
  data-loss-shaped mistake, even if recoverable (`u` undoes the drop, and `S p` then brings the edit back; 2+ steps).
- **Not the builder's defect:** the note prescribes these words (`m6-stash-put-back.md:110`). **Fix (conductor):**
  name the committed side by what it is (`branch · main`, or `on main`) and keep `your edits` for theirs. Consider
  the same for 72b's pop (`yours` | `stashed`).

### M4. Readability: boolean names don't read as questions, and `Status::Done::keys` no longer holds keys (PROMPT 6.2)
- **Where:** `update/clash.rs:384` `set_aside_entry(entry) -> bool`, `update/clash.rs:394` `set_aside(state) -> bool`
  (reads as a verb, and it is also the concept's name), `update/banner/stop.rs:366` `edits_aside(state) -> bool`.
  `update/state.rs:974-977`: `keys` is documented as "the next key … or what else is true after the tick", and now
  carries `your edits are back`.
- **Why it matters:** 6.2 says booleans read as questions (`is_merge`, `has_upstream`). A field named `keys` that holds
  a sentence will be misread by the next reader of `Done` (e.g. `undo.rs:229` overwrites `keys` with banner keys).
- **Fix:** `is_set_aside_entry`, `is_set_aside_clash`, `has_edits_aside`. Give the clause its own field
  (`Done { what, also, keys }`), or rename `keys` to `after`.

## Minor

- **m1. The notice's promise `esc, u undoes the switch too` is not walked in the app.** The pop has
  `esc_u_puts_a_clashed_pop_back` (`tests/it/m6_stash_put_back.rs:461`). The set-aside's undo is proven only in
  graft-ops (`graft-ops/tests/it/set_aside.rs:990`). Add `esc u` after `. ⏎ y` on the clash fixture, asserting branch,
  tree, index split and stash list as before.
- **m2. `a` on a stopped rebase with edits aside has a unit test only** (`banner/tests/resume_words.rs`,
  `aborting_a_stopped_rebase_says_your_edits_are_back`). The note lists it as a landing (`:57`). A walk
  `p r r esc a` asserting the original tip, the edits back and an empty stash list would cover it.
- **m3. Two tick placements for one fact.** `rebase … ✓ · your edits are back` (`update/operate.rs:858-861`, clause
  in `keys`) against `every file done · your edits are back ✓` (`update/clash.rs:359-362`, clause in `what`). Both are
  as the ticket wrote them. Pick one shape (heuristic 4).
- **m4. The staged clause under 83's meaning, for a rename.** `graft-ops/src/operation.rs:281-285`: a staged rename
  counts as 1 and comes back as a staged new file plus an unstaged deletion. The row says `1 staged comes back
  unstaged`, which is half true, and `files` counts the rename as 1 file although two paths change. Modifications,
  deletions, type changes and staged new files word truly. Textual, rare.
- **m5. The 80×24 preview assertion pins no rung.** `set_aside.rs:544` checks `contains("your edits: 3 files set
  aside")`, which every rung satisfies. The capture shows the full row. Assert the exact rung `fitting_line` picks
  at 80.
- **m6. A generic with one real use.** `jobs/operate.rs:344` `edits_kept_in<E>` is generic only for the test (6.2: no
  generics without two real uses). Use `graft_git::stash::StashError`. M1's fix removes the function anyway.
- **m7. Oversize files grew.** `update/operate.rs` is 2714 lines (1152 before `mod tests`) and gained ~30 production
  and ~85 inline test lines. `update/undo.rs` is 1634 lines and gained ~80. Both were already oversize, so the new
  tests belong in `operate/tests/` and `undo/tests/`.
- **m8. The builder's question on `kept_open`'s `Clashed` arm** (`update/banner.rs:~910`): it is unreachable.
  `finish::refusal` returns `None` for `Clashed` (`update/finish/answers.rs:175`), the finish summary is merge-only
  (`update/finish.rs:1-5`), and a set-aside never wraps a merge (`graft-ops/src/operation.rs:834`). Keep it (it is a
  correct refresh), but replace its comment with why it is unreachable. Don't restructure the types for it.

## The builder's four findings (gap rows M6/74)

| Row | As filed | Verdict |
|---|---|---|
| 827, stop line drops ` · edits aside` at 120 | question | As filed. The note's ladder orders the drop, and the `S` row and graph stash row still show the set-aside |
| 828, `S p` needs `j` when not `stash@{0}` | question | As filed. `n > 0` needs a stash made between step 3 and step 6, i.e. during a stopped rebase |
| 829, `back: true` says `your edits are in stash@{n}` | unverified | **Worse than filed: broken, three paths** (M1) |
| 830, `rebase` against the note's `rebased` | broken (textual) | As filed |

## Other checks
- Crate rules: graft-ui takes counts, not `graft_ops::SetAside` (`preview_box.rs:316-324`). Held.
- No git on the UI thread: the stash re-read runs on the job thread (`jobs/operate.rs:324-325`), and
  `interrupted_notice` reads `state.refs` (`update/undo.rs:630-640`).
- `unwrap`/`expect` outside tests: none added. `unwrap_or_default` at `update/undo.rs:646` is commented as not a
  failure, and it isn't (the notice is recomputed every frame, `view/frame/bars.rs:306`).
- Escapes: the new words come from constants and counts, and the rows pass through `sanitize_line`
  (`update/preview.rs`, `plan_lines`).
- Bar keys: `t set aside` is bound (`update/keys/bound.rs:102`); `every_key_the_bar_draws_is_answered` covers
  `SetAside`.
