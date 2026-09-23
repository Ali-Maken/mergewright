# fix-take-every-open-block: `Ctrl+O` / `Ctrl+T` take ours / theirs in every open block of this file, in one write

Status: draft · **gated on Proposed change P3** (02-keymap §2.6 gains two rows). Do not file until approved — and until someone checks Zed's terminal does not claim `Ctrl+T`.
Build with: set by the conductor after a scout (brownfield in graft); expected **Sonnet** — `take_all` is `take` in a loop; the write path is `apply_take`'s.
Needs: `after_take` / the take queue from `fix-take-advances-to-next-block` if P2 is approved (a queued single take must not interleave with a bulk one).
Crates: graft-git, graft-ops, graft-ui (`hints` Resolver table), graft.

## Read first
- `docs/design-notes/m6-conflicts.md` — Amendment: Prior art row "Whole-file take (text)", Journey J5, Interfaces (P3 block), Where I'd differ P3.
- `docs/research/conflict-resolution.md` §1.5 (the lazygit maintainer's caveat: "every block" is not `checkout --theirs`).
- `crates/graft-git/MAP.md` §conflict (`take`, `write_back`); `crates/graft-ops/MAP.md` §conflict (`apply_take`, `TakeError`); `crates/graft/MAP.md` §`update::resolver`; `crates/graft-ui/MAP.md` §`graph_screen::hints` (`Resolver`).
- `docs/design/02-keymap.md` §2.1 ("Bulk variants use a modifier").

## Scope
In:
1. `ConflictFile::take_all(take)`: every `Conflict` region becomes `Resolved { take }`; returns the count; already-taken blocks untouched.
2. `apply_take_all`: read → fingerprint check → `take_all` → `write_back` once (temp+rename). `Changed` never clobbers — the same contract as `apply_take`.
3. Resolver: `Ctrl+O`/`Ctrl+T` → `Command` for it; waits for (or drops, with words) a queued single take first; after it lands the cursor stays and the notice reads `took theirs in 3 blocks · ⏎ marks <file> resolved`. Zero open → `nothing open in <file>`, no write. Refused on a whole-file card (the card's own `o`/`t` are already whole-file).
4. Hints: `^O all ours`, `^T all theirs` in the Resolver table, after `b both`.

Out: `Ctrl+B` (tmux's prefix); staging (still `⏎`); any other file.

## Signatures
```rust
// graft-git/src/conflict.rs
impl ConflictFile { pub fn take_all(&mut self, take: Take) -> usize; }
// graft-ops/src/conflict.rs
pub fn apply_take_all(repo: &RepoHandle, path: &Path, expected: Oid, take: Take) -> Result<ConflictFile, TakeError>;
```

## Acceptance (name the verifying test at tick time)
- [ ] graft-git `conflict::tests::take_all_equals_taking_each_block` — for every fixture file and each `Take`, `take_all` renders byte-identical to `take(i)` over every open `i`.
- [ ] graft-ops `conflict::tests::take_all_never_clobbers_a_changed_file`.
- [ ] graft `update::resolver::tests::ctrl_t_takes_theirs_everywhere_then_enter_resolves` — `three.txt`: `Ctrl+T ⏎` → bytes equal per-block theirs (not `checkout --theirs` where they differ), `ls-files -u` no longer lists it.
- [ ] Journey `three-conflicts-all-theirs` at 2 keys (acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-take-all-120x40.txt`. **What right looks like:** all three headers read `took theirs`; the notice reads `took theirs in 3 blocks · ⏎ marks three.txt resolved`; the bar shows `^T all theirs`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Small–medium across four crates. One context.
