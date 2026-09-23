# fix-take-every-open-block: `Ctrl+O` / `Ctrl+T` take yours / theirs in every open block of this file, in one write

Status: draft · **P3 approved** (user, 2026-09-23: "keep all 5"). The conductor writes the new §2.6 row into `02-keymap.md`. Still open: whether Zed's terminal claims `Ctrl+T`. The acceptance capture taken from his terminal checks it (**unverified** until then).
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Sonnet**: `take_all` is `take` in a loop, and the write path is `apply_take`'s.
Needs: `fix-take-advances-to-next-block` (the take queue: a queued single take must not interleave with a bulk one); `fix-resolver-line-numbers` (`SideNames`, for the notice).
Crates: graft-git, graft-ops, graft-ui (`hints` Resolver table), graft.

## Read first
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P3; Interfaces (the P3 block).
  - Amendment 2: Journeys J5, J8, J9; Keymap rows owed (§2.6).
- `docs/research/conflict-resolution.md` §1.5 (the lazygit maintainer's caveat: "every block" is not `checkout --theirs`).
- `crates/graft-git/MAP.md` §conflict (`take`, `write_back`); `crates/graft-ops/MAP.md` §conflict (`apply_take`, `TakeError`); `crates/graft/MAP.md` §`update::resolver`; `crates/graft-ui/MAP.md` §`graph_screen::hints` (`Resolver`).
- `docs/design/02-keymap.md` §2.1 ("Bulk variants use a modifier").

## Scope
In:
1. `ConflictFile::take_all(take)`: every `Conflict` region becomes `Resolved { take }`, and the count is returned. Blocks already taken are untouched.
2. `apply_take_all`: read → fingerprint check → `take_all` → `write_back` once (temp file + rename). `Changed` never clobbers: the same contract as `apply_take`.
3. **Resolver.**
   - `Ctrl+O`/`Ctrl+T` → the command for it. It queues behind any take in flight (P2's queue).
   - After it lands, the cursor stays and the notice reads `took theirs · feature in 3 blocks · ⏎ marks three.txt done`.
   - Zero open → `nothing open in <file>`, and no write.
   - On a whole-file or binary card: refused with `this card is one choice · o or t`.
   - The answer records `AppState::choices` for every block it took.
4. **Hints.** The Resolver table gains `^O all yours  ^T all theirs` after `b both`, with the short rung `^O/^T all`. Update `BOUND_ACTIONS`.

Out: `Ctrl+B` (tmux's prefix); staging (still `⏎`, or P6's `O`/`T`); any other file.

## Signatures
```rust
// graft-git/src/conflict.rs
impl ConflictFile { pub fn take_all(&mut self, take: Take) -> usize; }
// graft-ops/src/conflict.rs
pub fn apply_take_all(repo: &RepoHandle, path: &Path, expected: Oid, take: Take) -> Result<ConflictFile, TakeError>;
```

## Acceptance (name the verifying test when you tick)
- [ ] graft-git `conflict::tests::take_all_equals_taking_each_block`: for every fixture file and each `Take`, `take_all` renders byte-identical to `take(i)` over every open `i`.
- [ ] graft-ops `conflict::tests::take_all_never_clobbers_a_changed_file`.
- [ ] graft `update::resolver::tests::ctrl_t_takes_theirs_everywhere_then_enter_resolves`: on `three.txt`, `Ctrl+T ⏎` → the bytes equal per-block theirs (not `checkout --theirs` where the two differ), and `ls-files -u` no longer lists the file.
- [ ] graft `…::ctrl_t_waits_behind_a_queued_take`.
- [ ] Journey `three-conflicts-all-theirs` at 2 keys (acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-take-all-120x40.txt`. **What right looks like:**
  - All three headers read `took theirs · feature`.
  - The notice reads `took theirs · feature in 3 blocks · ⏎ marks three.txt done`.
  - The bar shows `^T all theirs` or `^O/^T all`.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium across four crates. One context.
