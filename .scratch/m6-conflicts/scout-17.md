# Scout 17 — resolver file keys (against main 721547f6, the 16 merge)

## What exists
- `ResolverState` is in `crates/graft/src/update/resolver.rs` (fields ~140–200): `job`, `reading`, `base_job`, `staging: Option<JobId>` (:160, ⏎'s Stage and whole-file o/t), `restarting`, `notice`, `files` (:172), `seen`, `done`, `in_flight: Option<(TakeAt, Take)>`, `queued: VecDeque`, `land_last`, `dropped`. No `settling`.
- `TakeAt { Block(usize), Every }` — `update/events.rs:648`.
- `apply_take_all(repo, path, &ConflictFile, Take) -> Result<ConflictFile, TakeError>` — `graft-ops/src/conflict.rs:177`; `Changed` → job sets `reread` (`jobs/conflict.rs:316`).
- `take_all_key` — `update/resolver/take.rs:92`: whole-file card refused `this card is one choice · …`; `pending.open()==0` → notice `nothing open in <file>` (:125); else `take_side(TakeAt::Every)`. Queue cap refusal `still writing the last take` (take.rs:177, resolver.rs:250).
- `take_landed` — `answers.rs:255`: pops next queued, notice `took <side> in N blocks · ⏎ marks <file> done`.
- ⏎ → `mark_resolved` launches `Operation::Stage` via `operate::launch`, stores id in `staging` (`resolved.rs:47–53`). Answer → `update/mod.rs:468` → `resolved_advance(state, cmds, id, next_id)` (`resolved.rs:58`): ignores id ≠ `staging`; next not-done in list order, wrapping; notice `<file> done ✓ · ▸ <next>` (:103); last file closes to banner.
- `select_file(state, cmds, index, next_id) -> bool` — `files.rs:166`; refuses via `switch_refusal`. `file_cursor(state)` is a **function** (`files.rs:142`), not a field.
- `resolver_key` → `act` (`resolver.rs:282,312`): o/t/b, ^O/^T, B, n/N, ⏎, e, d, M; everything else plain → `scroll_key` (`scroll.rs:128`): `j`/`↓` +1 line, `k`/`↑` −1, PgDn/PgUp page, `g`/Home top, `G`/End bottom. `O`/`T` unbound today (fall to scroll_key, no-op).
- Hints: `resolver_list` — `graft-ui/src/graph_screen/hints/lists.rs:259`; order `o t b ^O ^T [B] e n N ⏎ resolved M [d] esc`.
- `BOUND_ACTIONS` — `crates/graft/src/update/keys.rs:755` (^O rows :820, `⏎ resolved` :829).
- Keymap §2.6 already has `O`/`T` (`02-keymap.md:264`) and `j/k ↓/↑` (:268); §2.1 exception named (:29–32). Claimed, rows present.
- Tests: `update/resolver/tests/file_list.rs:813` presses `j` (scroll today) — will change meaning.

## Contradictions
1. Signatures say `graft/src/update/resolver/mod.rs`; no such file — it is `update/resolver.rs`.
2. Needs line says `ResolverState.files`/`file_cursor`; `file_cursor` is a fn (`files.rs:142`), not a field.
3. Hint bar word: ticket says `j/k file` after `⏎ done`; code says `⏎ resolved` (`lists.rs:273`, `keys.rs:829`).
4. `BOUND_ACTIONS` is in `graft/src/update/keys.rs:755`, not graft-ui `graph_screen::hints` as Read-first implies (crate set still covers it via `graft`).
5. "No open blocks left → only Stage": `take_all_key` sets `nothing open in <file>` and returns (take.rs:125); new path must bypass.
6. `resolved_advance` advances only on `staging == id` (resolved.rs:66); whole-file o/t already uses `staging`, so O on card advances with no new code — consistent.
- Could not confirm: `(file_list.rs:813)` test's expectation; TESTING.md scenarios using ↑/↓.
