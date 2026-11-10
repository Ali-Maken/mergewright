# Scout 12: fixture identity (main @ 121c167c)

## Identity strings and where set
- `crates/mergewright-fixtures/src/git.rs:323` `IDENTITY_NAME = "Graft Fixture"`, `:325` `IDENTITY_EMAIL = "fixture@example.invalid"`.
- `crates/mergewright-fixtures/src/stream.rs:31` `IDENTITY = "Graft Fixture <fixture@example.invalid>"` (separate literal, not built from git.rs).
- `crates/mergewright-fixtures/src/synthetic/reflog.rs:238` inline literal `Graft Fixture <fixture@example.invalid>`.
- Users of the constants: `lost_work.rs:302-303`, `synthetic/history.rs`, `tests/it/ease_walk/fixture.rs:31` (`ME` = IDENTITY_EMAIL, the "you" email), `:241`.
- **No constants exist for `Graft Test` / `Graft Perf`**: inline literals in tests. Counts (crates+scripts): `Graft Fixture` 17 files/25 lines; `Graft Test` 30 files/44 lines (e.g. `tests/it/app_harness.rs:98,115`, `src/jobs/finish/tests.rs:12`); `Graft Perf` 2 files/3 lines (`tests/it/rename_names.rs:107`, `mergewright-git/examples/stash_only_cost.rs:171,173`).
- Emails seen: `fixture@example.invalid` (×11 configs), `test@example.invalid` (paired with `Graft Test`), `test@graft.invalid` ×3 (`mergewright-ops/tests/it/remote_ops.rs:400,927`, `aborted_fetch.rs:448`). Plus unrelated `practice@`, `sara@`, `t@example.com`, `t@t`.
- No other `"Graft <Word>"` identity literal in crates.

## Hard-coded hex (7–40) string literals in `.rs`
- 180 literals across 68 files; top: `mergewright-ui/src/rewrite_editor/tests.rs` 18, `detail_pane/tests.rs` 11, `mergewright/src/update/preview/tests.rs` 10, `view/rewrite_editor/right/tests.rs` 9, `mergewright-core/src/journal/merge_state.rs` 8, `journal.rs` 7. 29 of the 68 files mention fixtures/corpus.
- Samples checked are invented ids (`rewrite_editor/tests.rs:50-69` "4c3b2a1"…, `diff_cases.rs:342` "a1b2c3d4…", `captures.rs:334-342` "1e43bf9" inside a masking test). **Could not confirm** how many are fixture-derived oids; none of the samples were.

## Cache key (`crates/mergewright-fixtures/src/digest.rs`)
- Key = FNV over `include_str!` of generator sources + `LAYOUT_VERSION "1"` (:38) + `git --version` (:48). Corpus list :17-34 includes `stream.rs` and `git.rs` → changing those constants changes corpus, synthetic (:56-63) and team (:80-87) keys.
- **Gap:** `synthetic/reflog.rs` (and `synthetic/history.rs`, `scale.rs`) are not in the SYNTHETIC list — only `synthetic.rs` (:57). An edit only to reflog.rs's inline identity would not change the synthetic key. `lost_work.rs` is in no list (0 hits); whether it is cached at all: could not confirm.

## Tie-breaks / hashes on oid
- `mergewright-git/src/history/node.rs:136-140` topo Key: time, then `other.id.cmp(&self.id)` → equal-time commits reorder when oids change.
- `mergewright/src/map/walk.rs:92` branches tie-break on oid.
- `mergewright-graph/src/identity.rs:53` `BranchIdentity::of_short_sha` (FNV over first 4 oid bytes), used `fold.rs:241`, `row.rs:149` for unnamed lanes → lane colour/slot can change.
- No author-hash colour found (`identity.rs` hashes branch names / oids only).

## Initials in captures
- Yes. `mergewright-ui/src/graph_screen/columns.rs:18` `initials()`. 69 files under `docs/shots/` contain `GF` (e.g. `GF 6y 68c1`). `Graft Fixture` appears 81× and `fixture@example.invalid` 61× in docs/shots; `Graft Test` 2×.

## Contradictions with ticket
- D1 "email local parts `graft` → `maker`": no email has local part `graft`. `graft` is a **domain** (`test@graft.invalid`, 3 sites); fixture emails are `fixture@example.invalid`.
- Signatures: no `FIXTURE_AUTHOR/TEST_AUTHOR/PERF_AUTHOR` exist; names are `IDENTITY_NAME`/`IDENTITY` (fixtures) and inline literals elsewhere.
- Initials change GF→FM, TG→TM? (`Graft Test`→"GT"/`Test Maker`→"TM"): same width, but masking must include initials (D3 says so).
- Oid-ordered tie-breaks (node.rs:140, walk.rs:92) and oid-hashed lane identity (identity.rs:53) mean `differ=0` is not guaranteed.
