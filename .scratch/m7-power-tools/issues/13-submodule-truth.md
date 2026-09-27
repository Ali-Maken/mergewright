# submodule-truth — a submodule row says what git says: pointer moved, changed inside, untracked inside, not set up

Status: done (13a + 13b), except the fuzz corpus box (see 13b NOTES)
Blocked by: none
Crates: graft-git (`status` parse of porcelain v2's submodule field), graft-ops (`submodule_log`), graft-ui (the row and
the diff pane rows), graft (mapping).
Source: note Decision 3; "Pass 2a" R6 (Submodules); `docs/research/branch-compare-submodules.md` §B (git's own states),
takeaways 7, 8, 10 ("correctness over richness"; test a submodule-only change, Sublime #1084, and a moved+dirty
submodule, lazygit #3641).
Build with: Opus. The value is correctness against git's own states; real-repo tests with real submodules judge it.
Scout: `scout` first (ADR-0035).
Needs: none.

## Decided
- **Status:** porcelain v2's `<sub>` field (`N...` = not a submodule; `S<c><m><u>`) parses into `SubmoduleStatus
  {pointer_moved: c=='C', changed_inside: m=='M', untracked_inside: u=='U'}`. `StatusEntry.submodule` becomes
  `Option<SubmoduleStatus>`.
  - `parse_porcelain_v2` stays a fuzz target; the fuzz corpus gains `S` records.
- **Not set up:** a gitlink in the index whose directory has no `.git` → `not set up`.
- **The uncommitted row** (detail pane): `▣ <path>  submodule · pointer moved <old7> → <new7> · changed inside ·
  untracked inside`.
  - Each fact only when true. A submodule with none shows **no row** (the status rule).
  - Not initialised: `▣ <path>  submodule · not set up`.
- **The diff pane on a submodule path:** the header `<path> · submodule · <old7> → <new7>`, then from
  `git diff --submodule=log`, one row per commit: `+ <sha7> <subject>` (pulled in) or `− <sha7> <subject>` (rewound).
  - Capped at 50, then `+N more`.
  - The submodule not set up, or the commits missing: `the submodule's commits aren't here · not set up`.
  - Content diff (`=diff`) is not offered.
- **Distinctions:** `▣` + the word `submodule` vs a file row; `+`/`−` per commit. Colour unverified.

## Read first
- Note Decision 3, "Pass 2a" R6.
- `docs/research/branch-compare-submodules.md` §B (git's states) and takeaways 7–11.
- `crates/graft-git/MAP.md` `## status` (`StatusEntry.submodule`, `parse_porcelain_v2`, `read`); `crates/graft-ops/MAP.md`
  facts; `crates/graft-ui/MAP.md` `FileStatusView`, `detail_pane::files`; `crates/graft/MAP.md` the detail pane mapping.

## Scout brief (≤ 1 page)
1. `StatusEntry.submodule`'s type today and how it's set (bool from `S…`?). Every reader of it.
2. What the detail pane draws for a gitlink path today (`TypeChanged`? nothing?), and what the diff pane shows.
3. `graft-fixtures`: can it make a repo with a submodule (a local file URL, `protocol.file.allow=always`)?

## Signatures
```rust
// graft-git
pub struct SubmoduleStatus { pub pointer_moved: bool, pub changed_inside: bool, pub untracked_inside: bool }
// StatusEntry { …, pub submodule: Option<SubmoduleStatus> }
// graft-ops
pub struct SubmoduleLog { pub pulled_in: Vec<(Oid, String)>, pub rewound: Vec<(Oid, String)> }
pub fn submodule_log(repo: &Repo, git: &Git, path: &Path, from: Oid, to: Oid, stop: &dyn Stop)
    -> Result<SubmoduleLog, FactsError>;
```

## Acceptance
- [x] **graft-git:** — **13b**
  - `parse_reads_each_submodule_flag` (the 8 combinations); — `graft-git/src/status/tests.rs`, loops `0..8`; plus
    `a_malformed_submodule_field_is_an_error`, `a_gitlink_without_a_git_dir_is_not_set_up`,
    `a_repository_without_submodules_has_none_not_set_up`, `untracked_inside_a_submodule_counts_as_unstaged`;
  - `a_plain_file_has_no_submodule_status`; — same file.
- [ ] the fuzz corpus gains `S` records — **not done, cannot be done as written**: `fuzz/corpus/` is gitignored
  (exists only in the main checkout), so no file can be committed. The five `S` records to seed are in 13b NOTES;
  the conductor decides (seed locally, or un-ignore a seed dir).
- [x] **Real repos** (graft tests, real submodule fixture) — **13b**, all in `crates/graft/tests/it/submodule_truth.rs`:
  - `a_submodule_only_change_shows_pointer_moved` (Sublime #1084);
  - `a_moved_and_dirty_submodule_shows_both` (lazygit #3641);
  - `untracked_inside_is_its_own_fact`;
  - `a_clean_submodule_shows_no_row`;
  - `a_submodule_not_set_up_says_so`;
  - `submodule_log_lists_pulled_in_and_rewound`; — **13a: passes**, `graft-ops/tests/it/submodule_log.rs`, held to
    git's own `git diff --submodule=log` in the same test (the oracle);
  - `submodule_log_honours_stop`. — **13a: passes**, same file.
- [x] **UI** snapshot tests: the row words at 187 and 80 columns; the diff pane rows. — **13b**,
  `graft-ui/src/detail_pane/tests.rs`: `a_submodule_row_says_its_facts_at_187_columns`,
  `a_submodule_row_drops_its_ids_before_a_fact_at_80_columns`, `each_submodule_fact_is_its_own_word`,
  `the_diff_pane_on_a_submodule_lists_its_commits`, `the_diff_pane_caps_a_submodules_commits_at_50`,
  `the_diff_pane_says_when_a_submodules_commits_are_not_here`.
- [x] **Captures** (colour unverified) — **13b**, through `check_shot`:
  - `docs/shots/m7/submodule-row-120x40.txt` (`a_moved_and_dirty_submodule_shows_both`). **Right** as written
    **cannot hold at 120**: the Below layout gives the file list 30 columns, so the row draws `▣ libs/parser
    submodule ·…`, cut. The test asserts the drawn prefix and the full words from the row's own view. Finding in 13b
    NOTES (needs a 187 capture or a shorter ladder).
  - `docs/shots/m7/submodule-diff-120x40.txt` (`the_diff_pane_lists_the_commits_a_move_pulled_in`, held to git's own
    `log --left-right`). Shows `+ efdd295 parser 3`, `+ 8512312 parser 2`, `− 5bdba05 side 1`.
- [x] MAP.md (graft-git, graft-ui, graft; graft-ops by 13a). TESTING.md row "M7 submodule rows". — **13b**, docs.

## Touches (check each, report how)
- **Every reader of `StatusEntry.submodule`** (a type change): grep; each listed; the workspace builds.
- **Staging (`space`) and discard (`d`) on a submodule path:** unchanged behaviour. Check: add
  `space_on_a_submodule_stages_the_pointer` if absent, or name the existing test.
- **Working-tree counts and the clean check:** a submodule with only `untracked_inside` — does it count as dirty
  today? Keep today's answer; NOTES says what it is.
- **Set-aside and stash:** untouched. Check: the stash tests green.
- **Keymap, journal:** none.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~50k).
