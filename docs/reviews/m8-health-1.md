# M8 review — health part 1 (21b-6, 33, 34, 35, 36, 37 base)

Reviewed at **`05e4e3adb82e4159a4e0566a65e146598098670e`** (root `main`), read-only. Merges:
21b-6 `046585d5`, 33 `a44530fa`, 34 `7988b9e2`, 35 `e96e226a`, 36 `61e2a582`, 37 `ccffa49c` (base only; § Addition is building, not reviewed).

**Not run:** cargo, any gate, the heavy tier, the release perf run (conductor's instruction: no cargo). Every "held" below is from reading
the tree and from scripts over `git show` output, not from running a test. Nextest counts and "clippy clean" claims are unverified by me.

**Method.**
- Move-only: rebuilt the 21b step-2 line-set check myself (trimmed, non-blank lines, sorted multiset, `comm`) for each split commit
  against its parent: 34 `13858647`, 35 `28401b56`, 36 `74ab64ae`. Then checked that nothing after each split commit touched the moved files.
- Ratchet: recounted every tracked `.rs` under `crates/*/{src,tests}` at the hash (`git show | wc -l`) and diffed against `over_400.txt`.
- Allows: scanned all 296 `allow(` attributes (incl. `cfg_attr`) in `crates/` for a `reason`, and read the less common reasons in context.
- 37: checked ~45 MAP item lines across all six MAPs against their declarations.
- `code-review` skill: both axes (standards and spec) applied by me in one context. This environment has no sub-agent tool.

## Summary

| Ticket | Ticks checked | Held | Notes |
|---|---|---|---|
| 21b-6 | 2/2 | 2 (tick 1 only statically: 296/296 allows carry a reason; clippy not run) | diff is allow-only, reproduced |
| 33 | 5/5 | 5 | list equals reality at the hash: 286 rows, 87 production / 199 test |
| 34 | 3 NOTES ticks + 4 blank spec boxes | 2/2 checkable (line-set, over_400) | blank boxes hide nothing (see m5) |
| 35 | 3/4 (nextest not checkable) | 3 | line-set reproduced; max file 324 |
| 36 | 4/6 | 4 | line-set reproduced; **purity guard lost coverage (M1)** |
| 37 | 4/5 | 4 | ~45 MAP lines checked, all held; one quoted line number off (Q1) |

**Counts:** Critical 0 · Major 2 · Minor 7 · Question 2. No false tick found.

---

## Critical

None.

## Major

### M1. After 36, the purity guard no longer scans `update`'s own body
- **Where it breaks:** `crates/graft/src/update/tests/purity.rs:76-81` cuts every `mod.rs` at the **first** `#[cfg(test)]`.
  It assumes that marker is the trailing `mod tests;`. 36 added `#[cfg(test)] use graft_core::job::JobId;` at
  `crates/graft/src/update/mod.rs:91-92`.
- **Effect:** the scan now drops `mod.rs:92-271`. That range holds `pub fn update` (`:173`), `opening`, `settle` and the
  re-export block. Before 36, the only `#[cfg(test)]` was on line 1419 of 1420 (`git show 74ab64ae^:…/update/mod.rs`).
- **Today:** lines 92-271 contain none of the forbidden words (I grepped the list at `purity.rs:101-117`, plus `atomic`/`FirstLaunch`).
  So there is no impurity now. There is a silent hole: an `std::time::Instant` added to `update` would pass.
- **Ticks:** the 36 tick (`36:45`) is not false: the guard passes and does scan the five new children (159 → 164 files). But the
  check named for that tick could not see this.
- **Why it matters:** the guard is the only enforcement of the pure-core rule (ARCHITECTURE.md; PROMPT 6.1). It is supposed to be
  a check that "cannot help noticing" (`purity.rs:65-66`).
- **Fix (cause):**
  - Strip at the `mod tests` declaration instead of the first marker: `rsplit_once`, or match `#[cfg(test)]\nmod tests;`.
  - Or remove the test-only import: the tests name `graft_core::job::JobId` directly, and `mod.rs:90-92` goes.
  - Either way, add a table case to `the_scan_skips_test_modules_only`, or a new test: a `mod.rs` with a mid-file
    `#[cfg(test)] use …` followed by `std::time` must still fail.

### M2. 35's `super::` re-import makes 47 call paths point at the wrong module (readability, PROMPT 6.2)
- **What:** `crates/graft/src/jobs/runner/mod.rs:10-14` privately re-imports 19 sibling `jobs` modules into `runner`. This lets
  the moved bodies keep writing `super::<module>::…`.
  - Example: `crates/graft/src/jobs/runner/reads.rs:129` `super::conflict::open(…)`. It reads as a `runner` child, but it is
    `crate::jobs::conflict`.
  - Counts: 11 sites each in `reads.rs`, `lists.rs` and `queue.rs`; 14 in `operations.rs`.
- **Why it matters:** someone navigating looks for `runner/conflict.rs`, which does not exist. The only explanation is a one-line
  `//` comment in a different file. This is clever code where plain code would do.
- **Cause:** it was a fair trade for the move-only line-set check (rewriting the paths would have shown up as changed lines).
  The follow-up is mechanical.
- **Fix:** one commit that rewrites `super::X::` → `crate::jobs::X::` in the four children and deletes `mod.rs:10-14`. No
  behaviour change; the gates judge it.

## Minor

### m1. The ratchet checks counts, not the list's shape
- `crates/graft/tests/it/file_size.rs:78-90` collects rows into a `BTreeMap`. A duplicated path silently keeps the last count,
  which breaks the "list stays exact" promise (ticket 33:38).
- `over_400.txt:1` says "sorted by path", and the ticket asks for it (33:24), but nothing checks it. The file is in locale order,
  not byte order: under `LC_ALL=C sort -c` it fails at the row `778 crates/graft-ops/src/journal/tests.rs`.
- **Fix:** `listed` rejects a duplicate path and a row out of byte order.

### m2. `map_coverage` (a) matches a name anywhere, so it cannot see same-named items or a missing own line
- **(a) is name-only.** `crates/graft/tests/it/map_coverage.rs:197-219` accepts any backticked word on any `- ` line, and any
  word in a heading.
  - Same-named items in different modules count as one. git has three `pub fn read`: `status.rs:328`, `conflict/disk.rs:41`,
    `shallow.rs:30`. One MAP line satisfies all three; today all three happen to be listed (MAP :85, :251, :283).
  - An item named only inside another item's signature or "also …" tail passes. Example: core MAP:94
    "also `as_bytes`, `shorthand`, `is_graft_internal`".
- **Spec says more than the guard does.** Ticket 37:26 says "the guard enforces items 1–4". It does not enforce point 1 (one
  heading per public module, in source order) or point 2 (one line per item).
  - The MAPs do follow point 1 today: headings match `lib.rs` order for core, term, graph and git, checked.
- **(d) is very loose in the base.** `declared` (`:152-164`) takes the first identifier of *any* source line (a call, a field,
  `let`). The § Addition supersedes it.
- **Fix:**
  - The Addition should delete `declared` outright rather than layer on top of it.
  - 38 (or a later ticket) should qualify (a) by module (`heading` + name) so collisions count separately.
  - Either drop "enforces 1–4" from the format text, or add a heading-order check.

### m3. Stale comments into code the splits moved (textual)
- `crates/graft/src/update/resolver.rs:393` says `update/mod.rs` reads `HandedOver`. That arm is now `update/answer.rs:248`.
- `crates/graft/src/update/tests/keymap_words/stops/places.rs:143`: `mergetool_returned` is now `update/answers.rs:20`.
- `crates/graft/tests/it/m7_walk/rewrite_editor.rs:236`: `operation.rs:1278` → `operation/queries.rs:85` (`moves_head`).
- `crates/graft/tests/it/m5_walk/fixture.rs:318`: `operation.rs:574-575` no longer exists at that line.
- **Fix:** repoint, or drop line numbers in favour of item names.

### m4. The MAP lines for the splits are change-log shaped
- 34, 35 and 36 each asked for "one line per child" (34:31, 35:33, 36:33). Each wrote one long line instead:
  - `crates/graft-ops/MAP.md:13`: 676 characters, carries `m8/34` and "422 lines".
  - `crates/graft/MAP.md:1074`: `m8/35`.
  - `crates/graft/MAP.md:161`: `m8/36`, and its pre-existing `m2 review M8`.
- These break the very format 37 defines (ticket ids, measurements, over 240 characters). They are not under the guard yet.
- **Fix:** none now. 41 and 42 rewrite these MAPs, and their guard will catch them. Make sure those tickets know these lines exist.

### m5. Ticket text (34, 36)
- **34 header:** `34-health-split-operation.md:3` still reads `Status: draft`, but NOTES says done.
- **34's blank boxes.** The spec boxes `:42-45` are blank. NOTES re-ticks them in its own words. Judged; they hide nothing:
  - `:42` move-only: I reproduced the line-set check. The only differences are the NOTES-listed `mod`/`use`/`//!` lines, 11
    `pub(super)` widenings, the split `impl Operation {`/`}` and fmt re-wraps. Nothing changed in `operation*` after `13858647`.
  - `:43` "same count": the restated tick reports 6219 → 6222, with the diff being 33's three `file_size` tests from the
    mid-branch merge. It is honest, but it is not the box's literal check.
  - `:44`: the counts in NOTES match the tree (183/277/341/317/422/247/183). `refusal.rs` is one `pub enum Refusal`
    (`:11-422`), a legitimate stop.
  - `:45`: the rows are right; the list equals reality.
- **36 count:** NOTES `36:75` says `mod.rs` is 271 lines. At `74ab64ae` it is 273, and it is still 273 at the hash.

### m6. 21b-6 left "remove an allow that no longer fires" undone
- Disclosed at `21b-hygiene-round-2.md:383` and filed as gap `M8/21b6-stale-allows` (KNOWN_GAPS:1221, unverified). Correctly marked.
- **Suggestion (cause, not symptom):** clippy's `allow_attributes = "warn"` forces `#[expect(…, reason)]`. rustc's
  `unfulfilled_lint_expectations` then flags every stale one mechanically.
- **Cost:** a 296-site edit. Some `cfg_attr` allows that fire only under one cfg would need care.

### m7. In 35, `dispatch_*` lives in `queue.rs`, not `dispatch.rs` (naming)
- `crates/graft/src/jobs/runner/queue.rs:104` `dispatch_detail` and `:169` `dispatch_history` sit beside `submit`/`announce`.
- `runner/dispatch.rs` holds `impl Dispatch`.
- A reader who looks for "dispatch" in `dispatch.rs` finds half of it. The MAP line explains it, but the file name doesn't.
- **Fix:** at next touch, rename the pair (e.g. `route_detail`), or move them next to `impl Dispatch` if that file stays under 400.

## Questions

### Q1. 37's quoted ticket-id failure names the wrong line
- 37 NOTES (`37-health-map-libraries.md:67`) says it appended the id to the `` `Order` `` line, and quotes the failure as
  `graft-git/MAP.md:129: ticket id`.
- At the hash, `Order` is MAP:130 and :129 is `CHUNK`. The MAP has a single commit (`604d104a`), so the line didn't move later.
- Was the quote reconstructed rather than copied?
- The behaviour itself is pinned independently by `map_coverage::ticket_id_pattern_matches_ids_and_nothing_else`
  (`map_coverage.rs:323`), so the tick stands.

### Q2. Does 34's target cover the pre-existing children?
- 34:36 says "`operation.rs` and each child ≤ 400".
- Two pre-existing children are far over: `operation/history.rs` 1016 and `operation/tests.rs` 2006. Both are on the ratchet.
- I read "each child" as the new ones. Confirm, or file them.

---

## Spot checks that held (no finding)

**Move-only.**
- 34, 35 and 36 line-set diffs are empty apart from `mod`/`use`/`//!` lines, `pub(super)` widenings, `impl` header/closer
  splits and fmt re-wraps of widened signatures.
- 34's `pub use types::*` re-exports no item that wasn't `pub` before: the only new visibilities are `pub(super)`.

**Ratchet.**
- Rules 1–4 match ticket 33:36-39.
- The messages match the NOTES quotes: `file_size.rs:108`, `:110`, `:113`, `:126`.
- Rule 5's test/production split matches the ticket.
- At the hash, `over_400.txt` (286 rows) is byte-identical to a fresh `wc -l` of tracked files over 400.

**Allows.**
- 296/296 carry a reason.
- Sampled in context: `too_many_arguments` ×6. Each fn has exactly one caller: `map.rs:268`, `rows.rs:124`, `rows.rs:51`,
  `map_screen/render.rs:150`, `:203`, `apply/record.rs:175`. So "a struct for one caller" is true.
- Also sampled: the `dead_code` cfg_attrs in `blame/knobs.rs:22-61`; the cast reasons in `raster/emphasis.rs:497`; and
  `wildcard_imports` ×30 (all split preludes).
- No filler reasons. The two `reason = "..."` hits are prose comments (`graft-term/src/lib.rs:16`, `graft-fixtures/src/lib.rs:58`).
- 21b-6's diff touches only allow/cfg_attr lines, `Cargo.toml:62` and the ticket.

**37 MAP lines** (~45 checked, all held):
- capabilities: all 13 lines (variants, signatures, re-export names).
- git: `FileEntry`, `DiffLine`, `Hunk`, `BinaryNote`, `Truncated`, `FileDiff`, `ParseError`, `Pair`, `KnownCommits`,
  `parse_unified`, `parse_file_list`, `RepoHandle`, `::open`, `::git_path`, `::worktree_name`, `::commit_tips`/`ref_snapshot`/
  `peeled_references`/`upstream`, status `read`, `parse_porcelain_v2`, `TRACKED_CAP`, `tracked_paths`,
  `submodules_not_set_up`, `SubmoduleStatus`, `RefListing`, `SwitchRefs`.
- graph: `Kin`, `Keep`, `Ancestry::of`/`kin`/`lit`, `Fold::plan`/`marker`/`drawn_columns`/`folded_count`/`view`, `DrawnRow`.
- core: `Commit::date` (committer date, confirmed `commit.rs:70-72`), `subject`, `CommitHandle`, `::index`, `Parents::first`,
  `approximate_bytes`.
- term: `InputMux::on_stdin`/`start`/`pause`/`waiter`/`answer_fences`, `ReplyWaiter::read`, `Inputs`, `FenceListener`,
  `Environment::probe_budget`/`is_probeable`.
- fixtures: `Shape::ALL`/`STOPPED`/`STEPPED`/`TRACED`/`WIDE`, `CORPUS_NOW`, `PINNED_NOW`, `Commits`, `Graft` trait methods,
  `TeamSpec::MONOREPO`/`SMALL`.
- Before → after counts match NOTES for all six MAPs.
- No ticket ids, review ids or measurements remain in the six checked MAPs (grep).

**ARCHITECTURE.md.**
- 36's conductor addition holds: `rg 'jobs/runner\.rs' ARCHITECTURE.md` gives no hits; `:161` and `:196` name
  `jobs/runner/dispatch.rs`.
- `ARCHITECTURE.md:38` (`operation.rs`, one enum) is still true.

**Rules.**
- No new `unwrap`/`expect` outside tests.
- No swallowed errors in the new test code.

---

## KNOWN_GAPS rows these tickets should close or change (not edited)

**Close:**
- `M8/11-allow-no-reason` (KNOWN_GAPS:1158): `graft-ui/src/palette.rs` has no `allow` left, and the workspace lint (`Cargo.toml:62`) now enforces reasons.
- `M8/palette-mod-bare-allow` (:1235): `crates/graft/src/update/palette/mod.rs:47` now carries a reason.

**Reword (33; disposition in `m8-architecture.md:159`):**
- `M8/files-over-400-unrowed` (:1225) still says "Not counted exactly". At the hash it should read: tracked by the `file_size`
  test, 286 files (87 production, 199 test).

**Subsumed by the ratchet: close, or fold into the row above** (re-measure each against `over_400.txt` first):
- `M8/stop-files-over-400` (:1301): `jobs/pool.rs` 327, `apply.rs` 345, `apply/resume.rs` 306, and graft-git `exec.rs` no longer exists.
- `M8/look-file-sizes` (:1125): `empty_state.rs` is 333.
- `M7/39-apply-size` (:1081): `apply.rs` is 345 and `operation.rs` is split. `undo.rs` (833) stays on the list.
- `M7/30j-size` (:992): the `jobs/runner.rs` part is done. `update/detail/ops.rs` (783) and `update/remote.rs` (577) are on the list.
- `M8/test-files-over-400` (:1248), `M8/bars-m1-files-grew` (:1244), `M6/36` sizes (:738; `finish_summary.rs` is now 270).

**Repoint (paths moved by 34/35/36; the rows stay open):**
- `M7/r1621-m7` (:985, `jobs/runner.rs:361`) and `M7/30j-subdispatch` (:991, `:328`/`:363`) → `jobs/runner/queue.rs`.
- `update/mod.rs:<n>` refs at :577, :652, :659, :724, :726, :728, :879, :975, :1282.
- `operation.rs:<n>` refs at :691, :766, :818, :872, :960, :1072.

**37 rows, after the Addition lands:**
- `M8/37-map-format-contradiction` (:1223) is answered by the Addition's ruling.
- `M8/37-map-lines-unjudged` (:1224): this review checked ~45 more lines and all held. It is still a sample, so the row stays
  unverified, with the count updated.
- `m8-architecture.md` Minor 6 (test-only `pub` in git) is the Addition's.

---

SKILLS: code-review (both axes applied by me in one context; no sub-agents available)
