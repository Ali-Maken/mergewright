# daily-wording — words that name the wrong key, plan shorthand, and status that reads as an error when nothing is wrong

Status: ready
Blocked by: none
Crates: graft-ops (`apply/remote.rs`, `mistake_radar/rules.rs`), graft (`update/commit_editor.rs`,
`view/commit_editor.rs`, `update/commit_graph.rs`, `update/name_prompt.rs`), graft-ui (`commit_editor.rs`,
`preview_box.rs`, the name prompt's border).
Source: note "Pass 2a" R7. The KNOWN_GAPS rows, quoted, because builders don't read KNOWN_GAPS:
- **:774** `push rejected: … · p pull first · P force with lease` sends him to `p`, but pull is `--ff-only`, so a diverged
  remote answers `diverged · m merge it  r rebase onto it`.
- **:775** "commit or stash first" names words, not keys (`apply/remote.rs`).
- **:776** The commit editor opens with `subject empty` already showing (the refusal is computed every frame).
- **:777** Every non-default local branch without an upstream gets the radar's `RiskyState` `no upstream (<b>)`.
- **:778** The commit-graph offer appears on any repo without the file, whatever its size.
- **:779** Plan shorthand reaches the screen: `… diverged — merge or rebase (M5); shell: git pull --rebase`.
- **:856** The name prompt draws its keys twice, in its border and on the bar.
- **:860** (question) Other refused boxes still offer `←→ choose` over a single `[ Close ]` (87 fixed the stash box
  only).

Build with: Sonnet. Words and conditions; each fix is named below.
Scout: `scout` first (ADR-0035). Confirm each site; for :860, list every refusal box and what its bar shows.
Needs: none. Sequence it apart from other graft/graft-ops tickets (crate sets).

## Decided (exact words)
- :774 → `push rejected: origin/<b> has N commits you don't · p pull, then m or r · P force with lease`. N comes from the
  existing ahead/behind read, if it is at hand; if it isn't, drop the count: `origin/<b> has commits you don't`.
- :775 → `commit or stash first · c commit  S stash`.
- :776 → `subject empty` shows only after `Commit` was pressed (`⏎` in the subject / `Ctrl+Enter`) with an empty
  subject. It clears on the first typed character. Nothing on open.
- :777 → no radar finding for a missing upstream on any branch. The rule is removed, with its tests. The Mistake Radar
  row it fed is gone, and the push box still says where it will push.
- :778 → the offer shows only when this repo's last measured open sort took ≥ 300 ms. Use the existing sort timing if
  one is recorded; if none is, the builder records it (one `Instant` around the sort) and names it. Below that, no
  offer. The once-per-repo declined flag is unchanged.
- :779 → `pull stopped: origin/<b> and <b> have both moved · m merge it  r rebase onto it`. No `(M5)`, no shell command.
- :856 → the name prompt's keys on the bar only (as the preview and the confirm box do).
- :860 → every refused box with a single `[ Close ]` shows only `esc back` on its bar (87's rule). The scout lists each
  box; the builder fixes each one found. NOTES lists them.

## Read first
- Note "Pass 2a" R7; CLAUDE.md "A status field must never read as an error when nothing is wrong".
- `docs/design/02-keymap.md` §2.1 (trailing parens carry a key), §2.3c, §2.10.
- `crates/graft-ops/MAP.md` `apply` (remote), `mistake_radar`; `crates/graft/MAP.md` `update::commit_editor`,
  `update::commit_graph`, `update::name_prompt`, `update::remote::in_flight`; `crates/graft-ui/MAP.md`
  `preview_box`, `commit_editor`.

## Scout brief (≤ 1 page)
1. For each row: the file:line that produces the words now.
2. :777: every test and capture that shows `no upstream`.
3. :778: is an open-sort duration recorded anywhere?
4. :860: every call that draws a refused box, and its bar text.

## Signatures
```rust
// no new pub API. mistake_radar: the no-upstream rule removed (name it in NOTES).
```

## Acceptance
- [ ] One test per row, red first:
  - `rejected_push_names_pull_then_m_or_r` (:774);
  - `commit_or_stash_names_keys` (:775);
  - `a_fresh_commit_editor_shows_no_refusal` and `commit_with_empty_subject_says_subject_empty` (:776);
  - `a_branch_without_upstream_has_no_finding` (:777);
  - `a_fast_open_gets_no_commit_graph_offer` and `a_slow_open_gets_the_offer` (:778);
  - `diverged_pull_names_m_and_r_without_plan_shorthand` (:779);
  - `the_name_prompt_draws_keys_once` (:856);
  - `every_single_close_box_offers_only_esc` (:860).
- [ ] **Captures:** each changed capture is re-taken and listed in NOTES (e.g. `m4-01-push-box-plain-160x48` loses
      `. explain` on the bar; `m5-hints-uncommitted-160x48` loses the offer). New:
      `docs/shots/m7/commit-editor-fresh-120x40.txt`. **Right:** no `subject empty`. Colour unverified.
- [ ] Hint-bar tests green. MAP.md for the three crates.

## Touches (check each, report how)
- **Mistake Radar:** removing a rule changes the finding counts on the radar screen and in the header. Check: radar
  tests and captures; list the changes.
- **The push box, pull stop line, commit editor, name prompt:** each capture re-taken or proved byte-identical.
- **Keymap:** none. Words only. §2.1's trailing-parens rule is respected in every new string.
- **Other crates:** none.

## Rules / Gates
As rewrite-engine.

## Size
Medium (~45k).
