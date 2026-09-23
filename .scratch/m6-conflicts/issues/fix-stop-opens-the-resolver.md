# fix-stop-opens-the-resolver: a stop with anything unmerged lands in the resolver on the first conflicted file; `Esc` goes to the banner

Status: draft · **gated on Proposed change P5** (02-keymap §2.6a: "It takes focus the moment an operation stops"). Do not file until approved.
Build with: set by the conductor after a scout (brownfield); expected **Sonnet** — two call sites of an existing function, fully specified.
Needs: `fix-landing-after-a-stop` (`Status::Stopped`, `stop_reason`, the banner the resolver's `Esc` returns to).
Crates: graft.

## Read first
- `docs/design-notes/m6-conflicts.md` — Amendment: Journey J2, "Landing per event", Where I'd differ P5.
- `docs/research/ux-principles.md` §3.1.
- `crates/graft/MAP.md` §`update::resolver` (`open_resolver`, `OpenedFrom`), §`update::operate` (`operated`), §`update::banner` (`resumed`, `first_conflicted`, `unmerged_paths`).
- Scout question: whether the `Conflicted` answer's `paths` arrive in git's status order (`unmerged_paths`'s), or only after the re-read.

## Scope
In:
1. On a `Conflicted` answer (`operate::operated`) and a `Conflicted` again (`banner::resumed`) with ≥1 unmerged path: after `Status::Stopped` and banner focus are set, call `open_resolver(first, OpenedFrom::Banner)`. `first` = the answer's paths in git's order (answers beat re-reads); if the answer's order is not git's, wait for the working-tree read and use `first_conflicted` — say which in NOTES.
2. `Esc` in the resolver → banner focused, Stopped line and its keys on the bottom row (today's `OpenedFrom::Banner` path).
3. Unchanged: a stop with nothing unmerged (banner only); a launch into a live op (banner, Decision 2); a stash-pop conflict (no banner — resolver from the pane, as today).

Out: any new key.

## Acceptance (name the verifying test at tick time)
- [ ] `update::operate::tests::a_conflicted_merge_lands_in_the_resolver` — `m m` (or `m → ⏎` if P1 is not approved): resolver open on `first_conflicted`, `opened_from == Banner`, cursor on block 0.
- [ ] `update::banner::tests::a_rebase_stopping_again_lands_in_the_resolver`.
- [ ] `update::resolver::tests::esc_from_a_landed_resolver_shows_the_banner_keys`.
- [ ] `…::an_edit_stop_with_nothing_unmerged_keeps_the_banner`.
- [ ] Journey `a-stop-to-first-conflict` at 0 keys (acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-stop-lands-in-resolver-120x40.txt`. **What right looks like:** the frame right after the stop is the resolver, title `<file> · file 1 of N`, header still `MERGING`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Small. One context.
