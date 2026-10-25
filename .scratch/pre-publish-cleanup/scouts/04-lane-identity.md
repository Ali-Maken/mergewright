# Scout 04: lane identity, pane focus, preview seeds

## Part A: pane focus
- `update/branch_pane.rs:194 focused(state)`: `sources` (:212) takes `view::focus(history)` (cursor row's lane identity), then `BranchList::drawn_under(focus)` (`refs/branch_list.rs:111`, map `by_identity`, "first branch in name order whose identity is X"). Trunk via `standing_for(local)` (:117). Identity of a name = `RefName::shorthand` (`branch_list.rs:133 identity_of_name`).
- `focused` is None unless history, walk facts and refs have all landed (:212-218). Local-only (task doc says "local branch"); not confirmed where `drawn_under` rejects remotes (`candidate` at :135 does no filtering).
- Stacked fixture exists: `tests/it/graph_rows_tests.rs:94-120` (wallet-release, feature/receipts, ops/ci, main; upstreams faked via `update-ref` + config). Tests in `graph_rows_tests/stacked_roles.rs:53,76` check lane roles, not focus. No existing test walks `j` over every row and reads `focused()`: grep of focused callers in tests not exhaustive, could not confirm none.
- Team fixture (`mergewright-fixtures/src/team.rs`) builds forks/merges synthetically; I did not find which branches stack there. Could not confirm.
- Spawn seam: only `mergewright-ops/examples/branch_pane_cost.rs:22,72` counting lines in `$MWR_SPAWN_LOG`, fed by a `git` shim on PATH set up in `scripts/branch-pane-cost.sh:31-56`. It drives ops, no app/graph dep. No in-process spawn counter found (grep MWR_SPAWN_LOG, spawn_count, SpawnLog in crates: only that example). So "through the app path" needs a script.

## Part B: preview seeds
- Main graph: `history/stream.rs:44-59` reads `handle.upstream(name)` (gix config, `mergewright-git/src/repo/refs.rs:245`; no process) for every `refs/heads/*`, passes `seeds_with_upstreams` (`history/seeds.rs:37`). `follows_of` (:169) prefers configured upstream (`followed` map), else same short name.
- Preview: `update/preview/answer.rs:84` calls `history::preview_seeds(walk, refs)` (`seeds.rs:107`). Builds tips from `RefBadges::branches()/remotes()` and passes `&HashMap::new()` as `followed` at `seeds.rs:125` and `:146`. So the same-short-name rule only. Matches the ticket.
- `RefBadges` has no upstream ref: `RefLabel::Branch{upstream: Upstream}` (`refs/mod.rs:161`); `Upstream::InSync{remote}` (:179) holds the remote name only, `Apart(AheadBehind)` (:184) and `None` hold none. Ticket's `refs/mod.rs:171` is now :179.
- The refs read already has the full ref: `refs/mod.rs:202-209` builds `upstreams: HashMap<RefName,RefName>` from `handle.upstream`, passed to `badges()` (:225-290, `up` at :259). Not `%(upstream)`; gix config, no spawn. It is dropped after `badges` (stored only as `remote_of(up)` at :271, and `result.live.tracks(short)` at :264). Ticket says "refs read parses `%(upstream)`": contradiction, it doesn't.
- Remote labels in badges are omitted when level with a tracking local (`merged`, `refs/mod.rs:270`, Remote variant doc :166), so `remotes()` lacks those; the `tips` map in `preview_seeds` then lacks them too (existing behaviour, not checked against main graph).
- Existing seed test with differently-named upstream: `history/seed_tests.rs:88` / `tests/it/graph_rows_tests/stacked_roles.rs:84-88 a_remote_seed_follows_the_branch_that_names_it_as_upstream` (main graph side only). No preview-side test.
- Fixtures with different-named upstream: none found. All `branch.<b>.merge` writes use the same name (`graph_rows_tests.rs:121`, `ease_walk/fixture.rs:124`, `shapes/refs.rs:139`, `m7_walk/picker.rs:44`: feature-a -> refs/heads/main is the only differing one, local not remote).
- Preview opened via `preview_produced` (`update/preview/answer.rs:70`); the key to open a preview that draws a remote lane: could not confirm.

## Seeds are rebuilt only here
`preview_seeds` has one caller (answer.rs:84) plus `history/mod.rs:46` re-export.
