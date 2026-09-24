# Scout 16: take every open block (`Ctrl+O`/`Ctrl+T`), main at 8dcc4d3

## What exists
- **graft-git** `ConflictFile::take(block, take) -> Result<(), NoSuchBlock>` `crates/graft-git/src/conflict/document.rs:66`. It indexes Conflict+Resolved regions and replaces an earlier take. `render` is at `:21`. `write_back(path, file, expected) -> Result<Oid, WriteError>` is at `conflict/disk.rs:76`: it compares the fingerprint and returns `Changed { now }`. `Take` enum `conflict.rs:208`. No `take_all`.
- **graft-ops** `apply_take(repo, path, file: &ConflictFile, block, take) -> Result<ConflictFile, TakeError>` `crates/graft-ops/src/conflict.rs:151`. It works on a clone, then `take`, then `write_back(.., file.fingerprint)`, and sets the new fingerprint. `TakeError {Changed, NoSuchBlock, Write}` `:114`. Called only from `graft/src/jobs/conflict.rs:247`.
- **graft** `update/resolver.rs` (442 lines):
  - `ResolverState.in_flight: Option<(usize, Take)>` `:174`, `queued: VecDeque<(usize, Take)>`, `TAKE_QUEUE_CAP = 64` `:187`.
  - `act` `:295`: `plain = !control && !alt` `:301`. The done refusal (`done_refusal`, `done.rs:42`) runs only when `plain` and the key is `o t b B M n N ⏎` `:305-314`. `o/t/b` go to `take_key` `:318`.
  - **Every non-plain key is silently swallowed** `:329`. `Ctrl+O`/`Ctrl+T` currently do nothing, with no words.
- `take.rs`:
  - `take_key` `:19` tries `whole_file_take` first. `take_side` `:51`: if `in_flight` is set it queues, else it sends `Command::TakeSide(id, TakeRequest{path, file, block, take})` `:101`. The cursor advances at the keypress `:111`.
  - `take_refusal` `:117` refuses at the cap. `pending_file` `:162` replays `in_flight` then `queued`, each as `file.take(block, take)`.
- `answers.rs` `take_landed` `:200`: the notice/side word comes from `in_flight`'s single block `:209-219`. It pops the queue front and sends another `TakeSide` `:228`. The notice is `all_taken` only when 0 are open `:243`.
- `TakeRequest` `update/events.rs:632` is a single `block: usize`.
- **graft-ui** `hints/lists.rs`: `resolver_list` `:255`, `resolver_card_list` `:287`, `resolver_done_list` `:300`. `BOUND_ACTIONS` `graft/src/update/keys.rs:755`, with resolver rows at `:806-820`.
- Keymap: §2.6 row `Ctrl+O`/`Ctrl+T` `docs/design/02-keymap.md:263` (claimed for 16). `O`/`T` `:264` belong to ticket 17 (P6), not 16.

## Contradictions
1. **Signature.** The ticket has `apply_take_all(repo, path, expected: Oid, take)`. The sibling `apply_take` takes `file: &ConflictFile` (`graft-ops/src/conflict.rs:151`) because re-reading loses taken blocks (doc `:140-145`, `events.rs:625-630`). With only an `Oid` there is no document to call `take_all` on. Reading from disk would re-index blocks that are already taken, which contradicts "Blocks already taken are untouched".
2. **Queue shape.** `queued`/`in_flight` hold `(usize, Take)` (`resolver.rs:174`), and `pending_file` replays them by block (`take.rs:164`). A bulk take cannot be queued without changing that type, and so `TakeRequest.block` (`events.rs:640`). `take_landed`'s notice reads a single block (`answers.rs:209`), so "in 3 blocks" has no source today.
3. **Done refusal.** `done_refusal` is gated on `plain` (`resolver.rs:310`), so `Ctrl+O/T` would bypass it unless the builder adds them explicitly.
4. **`AppState::choices` does not exist in code** (grep of `crates/graft/src`: none). Ticket 19 owns it (`19-…md:41,110`), and 19 is not merged. Item 3's "records choices" has nothing to write to.
5. **Blockers.** `Blocked by: 10` (`SideNames`) is present: `update/resolver/names.rs`.
6. **Ticket 28 (queued)** splits `resolver.rs`. `act`/dispatch edits will conflict with it.

## Staging with `:(exclude)` / `add --all` + pathspec
None found. Searched `crates/` for `exclude`, `"--all"`, `"-A"`, `"add"`:
- `snapshot/plumbing.rs:188-199` runs `add -A` with no pathspec, into a **temp** `GIT_INDEX_FILE` copy, not the real index. It has `--literal-pathspecs`, so it would not interpret `:(exclude)` magic anyway.
- `operation/stage.rs:128` and `operation/conflict.rs:180,192` run `add -- <paths>`.
- `remote.rs:266` and `probes.rs:412,426` use `--all`, but not for `add`.

Could not confirm: whether the snapshot's temp-index `add -A` collapses unmerged stages in that copy (it does not touch the real index).
