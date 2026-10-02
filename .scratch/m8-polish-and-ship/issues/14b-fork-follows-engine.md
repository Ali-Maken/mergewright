# G-daily 3a — a branch forked off a rewritten commit follows when clean (engine + preview data)

Status: ready
Build with: Opus (conductor, after scout `scouts/14b-fork-follows.md`; `r` builds its todo via rebase_rows apply/engine.rs:162 → splits off per this ticket's rule; m5 left-behind capture may change)
Blocked by: 14
Needs: none (no signature from another ticket). Order: after `daily-ops-fixes` (12b; same graft-ops rewrite files).
Exposes the types `fork-follows-preview-words` draws.
File set: graft-ops `rewrite/` (plan, dependents, todo, saved, progress), `preview/` (effects, rewrite, chain),
`operation/rewrite.rs`, `apply/engine.rs` (todo hand-off only), graft-ops tests. **Not** graft: `left_behind` stays
filled (see Signatures) so the app compiles and draws as today until 3b.

## Read first
- `docs/adr/0059-a-branch-forked-off-a-rewritten-commit-follows-when-clean.md` (whole, 1 page; Accepted 2026-10-02)
- `docs/adr/0047-rewrite-engine-and-dependent-branches.md` §1, §2, §4
- `docs/design-notes/m8-polish-and-ship.md` "Pass 2c"
- `crates/graft-ops/MAP.md` `## rewrite`, `## preview` (`PreviewPlan` m7/37 fields, `ConflictForecast::StopsAt`,
  `plan_as_run`), `## operation` (`Operation::tips`), `## journal` (`touches_of`)
- Ticket 14 NOTES "Item 2 — what it needs" (branch `worktree-agent-a67428ac4073c7eaf`)

## Scout questions (answer before `Build with` is set; ≤1 page, code not intent)
1. Does `r` (rebase onto) build its todo through `RewriteParams`, or by another path? If another, `r` splits off.
2. Where is `LeftBehind` computed today (file:line), and which inputs does it already hold (merge-base, cap)?
3. Which tests assert `left_behind` for a fork whose replay would be clean (list each, file:line)?
4. Who reads `RewriteProgress` (graft banner and any other), and does any count `update-ref` lines?
5. `plan.json`: where is it read and written, and how does an unknown/missing field behave today?
6. Where do `Operation::tips` / `rewrite::tips` feed `check`'s `Moved` and the journal's `refs_before`/`refs_after`?
7. Every `PreviewPlan` and `RewriteParams` literal site, by crate (the compiler list).

## Scope
In: fork detection and fates; forecast per fork; `label`/`reset` sections in the todo; saved plan; `progress`;
operation tips / journal refs; ghosts and ref moves for followers in `PreviewPlan`. Every kind that reports
`left_behind` today: `Operation::Rewrite` (`R`, `a`-into, `a` on HEAD with dependents, absorb) and `r`.
**If the scout finds `r`'s params don't reach the todo through `RewriteParams`, `r` is split off — say so, don't guess.**
Out: drawing the fates (3b); ADR-0047 §4's tip-fork refusal (unchanged); forks of forks (they stay, `Nested`).

## Signatures
```rust
// graft-ops::preview (effects.rs), re-exported from preview
pub struct Fork {
    pub branch: RefName,   // full
    pub at: Oid,           // the replaced commit it leaves the line at (old id) == LeftBehind.on today
    pub commits: usize,    // its own: at..branch
    pub fate: ForkFate,
}
pub enum ForkFate { Follows { onto: Target }, Stays(WhyStays) }   // onto: the ghost of at's row
pub enum WhyStays {
    WouldConflict { commit: Oid, paths: Vec<PathBuf> },  // one of its own commits
    StopsBelow,                                          // the line's forecast stops at or below at's row
    HasMerge { merge: Oid },
    Pushed { upstream: RefName },
    Elsewhere { worktree: PathBuf },
    Nested { on: RefName },                              // forked off another fork
    DefaultBranch,
    PastCap,
    NotChecked(String),
}
impl WhyStays { pub fn words(&self) -> String; }   // graft-ops' clause, no keys: "would conflict in src/a.rs"
// PreviewPlan gains `pub forks: Vec<Fork>, pub forks_more: usize`.
// `left_behind`/`left_behind_more` stay, filled from the Stays forks only (3b removes them).

// graft-ops::rewrite
pub struct ForkSection {
    pub after_row: usize,     // todo index whose result is the fork's new base
    pub branch: Tip,          // pinned old tip
    pub picks: Vec<TodoLine>, // at..branch, oldest first, all Pick
    pub inside: Vec<Tip>,     // other local branches on at..branch's first-parent chain, bottom up
}
// RewriteParams gains `pub forks: Vec<ForkSection>`; empty → the todo is today's, byte for byte.
// RewriteProgress gains `pub fork: Option<String>` (no refs/heads/): the section git is in.
// plan_as_run: signature unchanged; fills params.forks (followers) and PreviewPlan.forks (all fates).
// rewrite::tips (operation/rewrite.rs): adds every follower's branch + inside.
```

## Acceptance (truth read with git, never with Graft's own reads)
- [ ] Clean fork follows: `main─c1─c2─c3` (`feature-a`, HEAD), `feature-x` = c2 + x1 (other file); stage a change,
      `a` into c1, run. `git rev-parse feature-x~1` == `git rev-parse feature-a~1`; `git diff feature-x~1 feature-x`
      == x1's old diff; subject kept; HEAD home on `feature-a`.
- [ ] Conflicting fork stays: x1 edits the line the amend changes. Preview: `Stays(WouldConflict{commit: x1, ..})`;
      after `y`, `git rev-parse feature-x` unchanged; the line moved.
- [ ] Reword HEAD with a dependent (`RewordHead`) and a fork off the dependent's middle commit: fork follows (git).
- [ ] `u` after a follow: every ref (`for-each-ref refs/heads`) equals before; one journal entry.
- [ ] Oracle: the generated todo hand-run with `git rebase -i` gives the same trees and refs (`rewrite_oracle.rs`).
- [ ] Verified against git: `label`/`reset` run without `--rebase-merges`; abort during a stop after a section leaves
      the fork ref unchanged (`git rev-parse`). If either fails: stop, report, don't work around (ADR-0059 §3).
- [ ] `progress` during a hand-run of Graft's todo with a `break` inside a section: `fork == Some("feature-x")`,
      `step` equals the pick-like rows of `rebase-merge/done`.
- [ ] Stays reasons each tested: pushed (upstream holds x1), elsewhere (worktree), merge in `at..F`, nested, default
      branch, ninth fork `PastCap`, forecast `NotChecked` (git < 2.40 path or cap) → never `Follows`.
- [ ] Fork moved between preview and `y` → `Moved`, nothing written (git).
- [ ] `plan.json` without `forks` (an older plan) reads; a stop resumed after relaunch finishes the section.
- [ ] No fork: todo text byte-equal to today's (existing oracle tests unchanged and green).

## Touches
- m7/37 tests asserting `left_behind` for a **clean** fork now see it follow: list each, update the expectation, say so.
- `PreviewPlan` and `RewriteParams` literal sites (compiler-enumerated, graft tests included: field defaults only,
  no graft logic). `Operation::tips` → `check`'s `Moved` and the app's re-check before `y`. Journal `touches_of`
  (refs_before/after), undo/redo of a rewrite. `progress` readers: graft banner (`branch` index must not count a
  follower's `update-ref`). Saved-plan format (`plan.json`, a contract). `rewrite_oracle.rs`. MAP.md graft-ops.
- Captures showing `left behind:` (`docs/shots/m7/amend-into-left-behind-187x44.txt`, `rewrite-editor-left-behind-…`,
  m5 contact sheet): unchanged by 3a (app untouched); 3b retakes them.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~75k. If the scout finds `r` needs its own params path, split `r` off first.
