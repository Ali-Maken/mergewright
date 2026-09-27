# 34 graft-detail-keys-split — `jobs/detail.rs` and `update/keys.rs` under 400 lines; `escape_key`'s dead rung

Status: draft
Blocked by: **20l** (the last queued graft ticket that points into `update/keys.rs` by line, `:263`) and **20m** (cites
`jobs/detail.rs:380-393` by line; added 2026-09-27, planner). Place after 20m, before 03b. **Must land before 21**
(history place adds a job beside blame's and an `Esc` rung). 30c, 33, 20k, 20m, 03b, 20j edit neither file (their
Crates lines, checked 2026-09-27); 33's `update/keys.rs:479` is a note for the conductor only.
Crates: graft only (`jobs/detail.rs`, `jobs/detail/*.rs` new, `update/keys.rs`, `update/keys/*.rs` new, their tests,
graft `MAP.md`). No graft-ui, no graft-git. Needing another crate → commit at the seam, report `blocked: needs <crate>`,
stop.
Build with: **Sonnet**. Pure moves along seams named below; the one non-move (Minor 3) is decided. No seam needs
judgement: each follows a line MAP.md already draws.
Scout: none. MAP.md names every item moved; the review names the lines (`docs/reviews/m7-32-20i.md`, at `cb065156`).
Line numbers move with 30c–20m: re-read each site before editing.
Needs: none.
Split by: planner, 2026-09-27, from review m7-32-20i Major 1 (gap M7/32i-major1) and Minor 3 (M7/32i-m3).

## Why
PROMPT.md 6.2: files under about 400 lines. `jobs/detail.rs` is 893, `update/keys.rs` 559, and neither has a gap row.
21 would add to both. No behaviour changes; every public path stays (`pub use` at the old path where MAP names it).

## Seam 1: `jobs/detail.rs` (893) → readers in the parent, three children
The parent keeps what MAP calls "the detail pane's reads": `read_files`, `read_file_diff`, `read_contained`, the module
doc, and re-exports. Do **all three** moves (headroom for 21's job); the parent should land well under 400.
1. `jobs/detail/blame.rs` — the blame job: `read_blame`, `blame_failure`, `path_bytes`, and any private helper only
   they call (the came-from diff). `blame_tests.rs` already sits in `jobs/detail/`; point it at the child, bodies
   unchanged. MAP names `jobs::detail::read_blame` / `blame_failure` (`:577`): re-export there.
2. `jobs/detail/working_tree.rs` — the changes pane's list: `assemble_working_tree`, `UnmergedShapes`, `shapes_of`, the
   section builders, `diff_spec_for`.
3. `jobs/detail/assemble.rs` — the pure commit-side assembly: `assemble_files` (path tree, `✉` row), `assemble_diff`
   (sanitise, pairs, word spans, syntax, CRLF → `␍`).
Seam rule: **pure assembly vs the reads that run git** (deep module: the pure half is testable without a repo). If an
item sits on both sides, it goes where its callers are; say which in NOTES.
Inline `#[cfg(test)] mod tests` in `detail.rs`: tests follow their code into `jobs/detail/<child>/tests.rs` (or one
`jobs/detail/tests.rs` if they're all the parent's), bodies unchanged.

## Seam 2: `update/keys.rs` (559) → the dispatch chain in the parent
The parent keeps the **order a key is offered to handlers**: `key_event`, `place_claim`, `pane_and_overlay_key`,
`graph_key`. Reading the parent answers "who gets this key first". Moves, in this order until `keys.rs` ≤ 400, then
stop:
1. `update/keys/escape.rs` — `escape_key` and every rung it owns (one key's whole ladder, top to bottom, in one file;
   Minor 3 lands here).
2. `update/keys/acknowledge.rs` — `acknowledge` and `dismiss_unseen` (what a key does to the status line before it
   acts). `mouse::mouse_event` calls `acknowledge`: re-export at `update::keys::acknowledge`.
3. `update/keys/refresh.rs` — `refresh_key` (`Ctrl+R`).
4. `update/keys/commit.rs` — `commit_key`.
Existing children (`bound`, `predicates`, `actions`, `view_change`) stay as they are. Visibility: moved fns become
`pub(super)` unless MAP gives them wider; re-export anything MAP names at its old path.

## Minor 3 (folded): `escape_key`'s last `show_again` rung
`update/keys.rs:553-558` (at `cb065156`) calls `place::show_again`; `detail::keys::place_key` (`detail/keys.rs:84`) has
already taken `Esc` whenever `esc_goes_back` is true (review Minor 3 names the lines). **Ruling:** remove the call.
In its place, one line and a check:
```rust
// Going back to a hidden trail is `detail::keys::place_key`'s: `detail_key` runs for every focus before this ladder.
debug_assert!(!place::esc_goes_back(next), "Esc with a hidden trail reached escape_key");
```
Before removing, confirm by line that `show_again` does nothing when `esc_goes_back` is false (so removing it changes
no outcome); quote the line in NOTES. If it does something, stop and report: that is not dead code.

## Not folded
- **Minor 5** (`^`'s "can't tell" words, `answer.rs:139-143`, `:177`): a type change across `jobs`, `update/place` and
  the view, not a move. The review asks for nothing now. Stays a gap; 21 decides if it reuses `BlameAnswer`.
- **Q1** (landing after `Esc`): folded into 20k (behaviour; this ticket changes none).
- **Minors 1, 2, 4** (graft-git): 32b.

## Signatures
None new or changed. Every `pub`/`pub(crate)` item keeps its path (re-exports). Moved private fns keep their shape;
only visibility widens to `pub(super)` where a sibling now calls them.

## Acceptance (name the verifying test or command at tick time)
- [ ] `wc -l` in NOTES for `jobs/detail.rs`, `update/keys.rs` and every file created: each ≤ 400.
- [ ] `git diff -M --stat` and `git diff --color-moved=zebra` show moves. NOTES lists every **non-move** line (a `use`,
      a visibility, a re-export, the `mod` lines, Minor 3) and nothing else.
- [ ] No caller edited outside the two files and their children: `git diff --stat` names only them, their tests,
      `blame_tests.rs`'s `use`, and MAP.md. Any other file → say why in NOTES.
- [ ] Minor 3: `grep -n 'show_again' crates/graft/src` in NOTES: no call from `update/keys*`. **Mutation:** gate
      `place_key`'s `Esc` arm off (`if false`), run `test(esc_back)`: red on the `debug_assert` (say which tests),
      revert. This shows the check sits on the path.
- [ ] Every existing test green, unchanged in body; `test(esc_back) | test(blame_tests) | test(bar_draws)` counts in
      NOTES.
- [ ] Captures byte-identical: `check_shot` green; `git status docs/shots` clean.
- [ ] MAP.md (edited, read back): `jobs` `detail` line (`:843`) names the three children; `update::keys` files line
      (`:382`) names the new children; `esc_goes_back` (`:570`) and the key-chain line (`:586`) no longer say
      `escape_key`'s last rung shows the trail (it's `place_key`'s, with the `debug_assert`); `walk_is_reading` (`:47`)
      checked still true.
- [ ] Gates workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`,
      `cargo test --doc`.

## Touches (check each; NOTES says how)
- **Callers of moved items** (`jobs::detail::*` from `jobs/runner.rs`/`kinds.rs`, `jobs::finish` (reuses the
  assembly), `update/place`; `update::keys::*` from `update/mod.rs`, `mouse.rs`, `update/tests*`): grep; paths held by
  re-export, so none edited.
- **`tests/it/seam_rule.rs`** (forbids write `GitInvocation`s outside `jobs/operate` and `jobs/undo`): run it; say
  whether it scans by path and still covers the new files.
- **`bound_actions` / `every_key_the_bar_draws_is_answered`:** unchanged; green.
- **ARCHITECTURE.md** module rules: read before moving; no new crate edge.
- **Keymap, journal, undo, graft-ui, graft-git, captures:** none. The grep/test for each in NOTES.
- **For the conductor:** closes M7/32i-major1 and M7/32i-m3. M7/32i-m5 stays open (→ 21).

## Size
~40k. One context. Commit after seam 1 with gates green, then seam 2 and Minor 3.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. `codebase-design` allowed (moving
modules).
