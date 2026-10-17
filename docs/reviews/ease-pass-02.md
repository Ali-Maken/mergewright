# Review: ease-pass group 02 — clash-reads

Range `9cb03ce8..4fb8f4bb` (branch tip `17cbbcc7`), graft-git only. Ticket
`.scratch/ease-pass/issues/02-clash-reads.md`, scout
`.scratch/ease-pass/scouts/02-clash-reads.md`. Reviewed read-only in a
detached worktree of `4fb8f4bb` with its own target dir (removed afterwards).

**Verdict:** sound reads, well tested, honest about what they can't count.
No Critical findings. One Major (readability: function size, PROMPT.md 6.2).
Nothing here is wired to a caller yet (Mp/P/Fr build later), so no UI-thread,
data-loss or escape-sequence exposure in this range.

## Gates (run by me, at `4fb8f4bb`)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run` (workspace) | 6455/6455 pass, 16 skipped, 4 slow |
| `cargo test --doc` | pass |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | 6524 pass, **2 fail**, 18 ignored — both `held_key::held_j_stops_on_release_pixel{,_free}` (mergewright `it`), the known raw-Kitty red (`KNOWN_GAPS.md` M7/46-kitty-no-deflate, M8/heavy-tier-reds). Not this group: no app code changed. Every binary ran (the failing one was last). The new proptest ran at 1000 cases, green. |
| Release perf run (`frames::`, `heavy::budgets_hold`) | **21 pass**, 0 fail, 1 ignored, 110.5 s. Pixel 50k-refs p99 8.9 ms (1 frame over budget, still within the test's allowance); Unicode p99 1.6–2.6 ms; JournalFind p99 10.2 ms; first paint 15.8–109 ms on the 100k fixtures; peak RSS 33–230 MB. |

**Measurements.** Fast tier 664.8 s (first run), 587.4 s (rerun) — **not
comparable** with the ~50–70 s baseline: load average was 7–13 on 10 cores
the whole time (another builder running). The machine was busy, so read these
as an upper bound, not a regression. The new tests cost ~1.0 s total
(slowest `a_clean_merge_…` 0.23 s). Five slowest, all pre-existing graft-ui:
`hints::tests::the_bar_never_draws_a_fragment_of_itself` 77.6 s,
`a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn` 71.7 s,
`the_bare_keys_are_a_tail_not_a_scatter` 68.2 s,
`no_hint_loses_ground_as_the_room_for_it_grows` 68.0 s,
`a_words_only_bar_draws_no_bare_key_and_ends_with_its_doorway` 41.9 s.
Peak RSS (`time -l`, top process) 1.69 GB fast tier, 1.70 GB heavy tier.
`target/` (this worktree, debug + release) 5.4 GB before the release build.

## Acceptance ticks — 6 checked, 6 held

| Tick | Checked how | Held? |
|---|---|---|
| `merge_forecast` fixtures (clean, 2 places, modify/delete, unrelated, old git) | Read all five tests (`tests/it/merge_forecast.rs:77-176`); each asserts the stated value (old-git compares against `forecast_merge`'s own `NotCheckable` reason, so the text is pinned, not copied). Ran them: green. | yes |
| Stopped `Stop` → `Err(Stopped)` | `merge_forecast.rs:179-188`; green. (See Minor 2 on what it covers.) | yes |
| Property `take → reopen → render == original` | `conflict/reopen_tests.rs:85-102`: every block index per case, `t` drawn per case; also asserts the whole `ConflictFile` equals the parsed one. Green at 64 and 1000 cases. | yes |
| `reopen` refuses open block and index past end | `reopen_tests.rs:8-26`; also checks a refused reopen changes nothing. Green. | yes |
| `line_origins` table test | `diff/origins.rs:122-162`: yours-only, theirs-only, both, neither (hand edit) and a base line; green. | yes |
| Existing `forecast_merge` tests unchanged and passing | `git diff 9cb03ce8 4fb8f4bb -- crates/graft-git/tests/it/history_reads.rs`: only five helpers made `pub(super)` (lines 17, 47, 57, 65, 80), no test body touched; `src/history_reads/tests.rs` untouched. All 5 named tests green in my heavy run. | yes |

Spec compliance: signatures match the ticket except `path`/`changed` are
`PathBuf`, not `Vec<u8>` — allowed by the ticket ("`changed_paths`' own element
type … one path type for both fields") and recorded in MAP.md:184-185. Order of
reads matches the ticket's 5 steps. `forecast_merge`, `Forecast`,
`merge_tree_step` untouched (diff shows additions only). `reopen` keeps the
block whole (`document.rs:89-104`). MAP.md `NoSuchBlock` line updated as asked.

Feature pipeline: this group is reads only, no journey of its own; the note
(`docs/design-notes/ease-pass.md`) has Need (:13), Prior art (:21), Journeys
(:43). Step counts are checked when Mp/P/Fr land, not here.

## Critical

None.

## Major

### 1. `merge_forecast` is 72 lines, over the ~60-line limit; `forecast.rs` is at 395 of 400
`crates/graft-git/src/history_reads/forecast.rs:191-262`. PROMPT.md 6.2 (line
436): functions under about 60 lines; split once comments are needed to
separate sections. This one has five sections (version gate, base, merge-tree
run, record parse, changed + places) and the 23-line `match` at :223-245 nests
`let … else` three deep. The file is now 395 lines; the next addition breaks
the 400 cap. The tree-id parse is also copied a third time (:226 here, :380 in
`merge_tree_step`), and the 2.38 reason string a second time (:200 = :79).

*Why it matters:* it is the function Mp's preview will be debugged through;
the record-parsing shape is the part most likely to change with git versions.

*Fix:* pull :216-245 into `fn read_records(raw: RawOutput) -> Result<(Option<Oid>,
Vec<&[u8]>), String>` (the `String` is the not-checkable reason) and a shared
`fn tree_id(field) -> Result<Oid, String>` used by both merge-tree readers;
a `const OLD_GIT_MERGE: &str` for the reason. Moving `merge_forecast` and its
helpers to `history_reads/forecast/merge.rs` keeps both files well under 400.
Marginal size, cheap fix; filed Major because the brief ranks readability so.

## Minor

### 1. `line_origins` matches by text set: common lines are misattributed
`crates/graft-git/src/diff/origins.rs:40-49`. `yours.contains(text)` /
`theirs.contains(text)` test membership in the set of every line each side
added anywhere in the file. So any result line whose text either side added
*somewhere* is claimed: a blank line, `}`, `end`, `return nil` added by both
sides in unrelated places reads `Both`; a blank line the user typed by hand in
a resolution reads `Yours` if yours added any blank line. The doc comment says
"text, not position" so this is the stated design, and the table test
(:122-162) uses only distinct texts, so it cannot show the case. For M17's
"one sample with from yours/from theirs", the sample Fr picks may be exactly
such a line.
*Fix:* either the Fr caller picks its sample from lines with distinctive text
(non-blank, not punctuation-only), or match as a multiset (each side's added
text claimable once). Add one table row with a blank line added on both sides
to pin whichever is chosen.

### 2. The stop test covers only the first spawn; `changed_paths` ignores stop
`tests/it/merge_forecast.rs:179-188` raises the stop before the call, so
`compare_span`'s `merge-base` (`compare.rs:43-53`) refuses and nothing after
it is exercised. `changed_paths` runs with `.run(&())`
(`history_reads.rs:314`, documented "never stopped: a single small spawn"),
called at `forecast.rs:245` after merge-tree. A stop raised while merge-tree
runs still lets `diff-tree` run to completion on a large merge. The
`cat-file` reads (:249) do take the stop. Bounded and documented, so Minor.
*Fix:* give `changed_paths` a `stop` (its other callers: `graft-ops/src/preview/landing.rs:89,110`, `preview/stash.rs:115,139`) or note in
`merge_forecast`'s doc that the diff-tree read is not stoppable.

### 3. One `cat-file blob` spawn per clashing file, uncapped
`forecast.rs:247-256`, `places_in` :281-315. N clashing files → N spawns, each
reading up to the 64 MiB runner cap. A merge clashing in a few hundred files
(a vendored-dir or lockfile-heavy merge) costs hundreds of spawns before the
preview answers. The ticket left this to the builder; the choice is not
recorded anywhere (no NOTES). Off the UI thread once Mp calls it from a job,
so a latency cost, not a freeze.
*Fix:* one `cat-file --batch` for all paths, or count places for the first
`CHANGED_CAP` clashes and leave the rest `None`.

### 4. `compare_span` runs a `rev-list --count` whose answer is thrown away
`forecast.rs:203-206` keeps only `split`; `compare_span` also runs
`count_between(split, theirs)` (`compare.rs:55`), a history walk. The ticket
prescribed `compare_span`, so this follows spec; it is one wasted spawn per
preview.
*Fix:* a stoppable `merge_base_stopped(repo, a, b, stop) -> Option<Oid>` (the
first half of `compare_span`) and use it here.

### 5. Question (rule 7 — not verified): criss-cross merges and `changed`
`changed` is `diff-tree <one merge-base> theirs` (`forecast.rs:245`, base from
`git merge-base` without `--all`, `compare.rs:44-48`), while `merge-tree`
merges against a virtual base built from all merge bases. With several merge
bases, `changed` may list a file the merge does not bring in, or miss one. I
did not build a criss-cross fixture to see whether the two disagree in
practice. *If confirmed:* compute `changed` from the written tree instead
(`diff-tree ours <tree>`), which is what the merge actually changes for the
user.

### 6. The ticket's NOTES are missing
The ticket asked twice to "say which in NOTES" (path type; where the merge
base came from). `.scratch/ease-pass/issues/02-clash-reads.md` at `4fb8f4bb`
has no NOTES section. MAP.md records `PathBuf`; the merge-base source (from
`CompareSpan`) and the per-file `cat-file` choice (Minor 3) are recorded
nowhere but the code.
*Fix:* add the three lines to the ticket.

## Checked and fine
- `unreachable!` in `reopen` (`document.rs:99-101`): same placeholder-move
  pattern as `take` (:75-79) and `take_all` (:114-117); guarded by the
  `filter` two lines above. No `unwrap`/`expect` outside tests.
- Errors: `parse(..).ok()` at `forecast.rs:311` turns a parse failure into
  `None` — commented, and is the ticket's contract (rule 3 satisfied).
  Non-zero `cat-file`, truncated output and NUL-containing blobs → `None`,
  each commented.
- Conflicted-record parse splits at the **first** tab (`forecast.rs:270-271`):
  the `<mode> <oid> <stage>` prefix holds no tab, so a path containing a tab
  survives. Paths stay raw bytes (`-z`, `bytes_to_pathbuf`).
- `<tree>:<path>` as `GitArg::Ref` has precedent (`trace/whole.rs:122`,
  `trace/function.rs:99`); it cannot be read as an option.
- Architecture: graft-git only, no new dependency, `origins` sits in `diff/`
  beside `pairs`; MAP.md entries added for every new public item.
