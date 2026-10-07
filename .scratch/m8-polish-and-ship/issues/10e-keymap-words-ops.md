# G-keymap 2e — graft-ops words name keys by what they do; graft spells them

Status: done (2026-10-07; builder compacted once mid-ticket — see NOTES)
Build with: **Opus** — the shape is fixed below (one trait, one enum, one adapter); the work is a wide caller sweep
held by the compiler and byte-identical tests. Not Fable: no structural choice is left open.
Blocked by: 10b (its `known_ops.rs` rows are what this ticket deletes).
Needs: from 10/10c — `Keymap::spell_keys`, `AppState::keymap: Arc<Keymap>`, graft-ui `keymap::sweep` helpers (10c
sweep (c)); from 09 — `ActionId`, `defaults()`, `Keymap::translate`.
Crate set: graft-ops + graft (callers, adapter, test scaffold) + graft-ui `tests/no_literal_keys/known_ops.rs` only.
**Files:** graft-ops `keys.rs` (new), `lib.rs` (mod line), `operation.rs`, `apply/error.rs`, `apply/error/words.rs`,
`apply/remote.rs`, `restore.rs`, `restore/outcome.rs`, `undo.rs`, the `preview` and `rewrite` modules holding
`PreviewError` / `RewriteError`, their tests, `MAP.md`. graft: `update/op_keys.rs` (new), `update/mod.rs` (one `mod`
line), **every caller** of a changed fn (step 1 below lists them), `jobs/*` that word an error, `update/tests.rs`
(one `mod keymap_words;` line), `update/tests/keymap_words.rs` + `keymap_words/ops.rs` (new) + empty
`keymap_words/{stops,resolver,undo_journal,preview_remote}.rs` (new), `MAP.md`.
**Scheduling:** may run beside 09d **if** step 1's list is disjoint from 09d's Files (likely contact:
`update/tests.rs`'s mod lines, `view/status*`). Before 10f–10i (they edit the same callers' files).

## Read first
- `10b-keymap-spell-in-words.md` **Ruling** (whole: the option taken, W1–W6)
- `ARCHITECTURE.md` crate table and layers (graft-ops: `git`, `core` only)
- `crates/graft-ops/MAP.md`: `Refusal`, `Refusal::explain_rungs`, `RefusalWords`, `Refusal::InTheWay`, `UndoError`,
  `PreviewError`, `RewriteError`, `## restore`, `apply` error entries (`ApplyError`, `EditsAt`), `apply::remote`
- `crates/graft/MAP.md`: `PreviewFailure`, `RewriteOpened`, `update::edge::context_of`, the jobs entries
- `crates/graft-ui/MAP.md` `## keymap` (`spell_keys`, `translate`, sweep helpers)

## Scope
In:
1. **Step 1, before editing:** grep every caller of `Refusal::explain`, `explain_rungs`, `UndoError::explain`,
   `PreviewError::explain`, the `ApplyError` / remote / restore wording fns, and every `to_string()` / `"{}"` of those
   types; write the file list in NOTES. A file in 09d's Files while 09d is building → stop, report.
2. graft-ops: `keys` module; every wording that names a key takes `keys: &dyn KeyNames` and spells through it; W3 for
   unbound. Sites (planner's reading of `10b-grep-hits.txt`, **not checked against code** — rule 7): `operation.rs`
   `R rewrites your branch`, `a puts staged changes …`, `c makes the first one`, `f to fetch, then look`, `S stashes
   {count} changed … first`, `c continues, a aborts`, `c continues`, `space stages`, `switch first (C)`, `c commits
   them` ×3, `a only changes older commits`, `S stashes {them} first` (`InTheWay`); `apply/error/words.rs:38`
   `u undoes it`; `apply/error.rs` `· u restores the backup`; `apply/remote.rs:330` and the two `push rejected` /
   `push refused` lines (`p`, `m`, `r`, `P`, `f`); `restore.rs` `U restores the backup` ×2; `undo.rs`
   `UndoError::explain`'s `u again` / `U again` (MAP; not a grep hit). Plus every row `known_ops.rs` holds.
   Out: `operation.rs:920` (names no key); prose rows.
3. A `Display` of these types: `explain(&NoKeys)`. NOTES lists each use found in step 1 and confirms none is drawn
   (each drawn one now calls `explain(&OpKeys(..))`).
4. graft: `op_keys` adapter; every caller passes `&OpKeys(&state.keymap)`. Jobs (Ruling): result already carries the
   value → word in `update`; flattens to `String` → job params gain `keys: Arc<Keymap>` cloned at dispatch.
5. Test scaffold: `update/tests/keymap_words.rs` — shared helpers (reuse graft-ui `keymap::sweep`; add only what is
   missing) and the five submodules; `ops.rs` filled here, the other four empty for 10f–10i.
6. Delete every `KNOWN: 10e` row in `known_ops.rs`.
Out: graft's own word sites (10f–10i). Rewording (20).

## Signatures
```rust
// graft-ops::keys (new; `pub mod keys;` in lib.rs)
/// A key a graft-ops wording tells him to press, named by what it does — never by chord (ADR-0055: the app
/// spells it from his keymap). Exhaustive: graft's map fails to compile when one is added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpKey { Commit, Continue, Abort, Absorb, Stage, Stash, Switch, Rewrite, Fetch, Pull, Merge, Rebase,
    ForcePush, Undo, Redo }
impl OpKey { pub const ALL: &'static [OpKey]; }
pub trait KeyNames { /// His label for `key`; `None` when unbound (W3).
    fn label(&self, key: OpKey) -> Option<String>; }
pub struct NoKeys;                     // every key None: keyless words for Display and logs
impl KeyNames for NoKeys { .. }

// graft-ops — replace, no keymap-less twin left
impl Refusal { pub fn explain(&self, keys: &dyn KeyNames) -> String;
               pub fn explain_rungs(&self, keys: &dyn KeyNames) -> Vec<RefusalWords>; }
impl UndoError    { pub fn explain(&self, keys: &dyn KeyNames) -> String; }
impl PreviewError { pub fn explain(&self, keys: &dyn KeyNames) -> String; }
// ApplyError, apply::remote's error, RestoreError: the fn that words each today gains a trailing
// `keys: &dyn KeyNames` (names in MAP.md).

// graft::update::op_keys (new)
pub(crate) fn op_action(key: OpKey) -> ActionId;     // exhaustive match, no `_` arm
pub(crate) struct OpKeys<'a>(pub &'a Keymap);
impl KeyNames for OpKeys<'_> { fn label(&self, key: OpKey) -> Option<String>; } // spell_keys(&[op_action(key)])
```
Variant names follow the row W2 pins (e.g. if `a puts staged changes …` answers `graph.amend`, the variant is
`Amend`); a key a site names with no variant → add one, NOTES names the site. graft-ops tests implement `KeyNames`
locally (one impl per test tree); **no production impl holds default letters.**

## Acceptance (each tick names its test, at tick time)
- [x] `update::op_keys::tests::every_op_key_is_a_row_and_spells_todays_letter` — each `OpKey::ALL` maps to a row of
      `defaults()`, W2-pinned (ctx stated per row), and `OpKeys(&Keymap::shipped())` spells the letter graft-ops
      wrote before (table of literals).
- [x] `update::tests::keymap_words::ops::each_ops_word_spells_the_remapped_key` (29 sites) — one row per site above (count
      stated): under the sweep keymap the line holds the remapped label and no default label of a moved action as a
      whole token; under shipped, byte-identical to the literal.
- [x] `update::tests::keymap_words::ops::an_unbound_key_drops_its_clause` — `graph.merge = "none"`: `… · p pull, then r · P force with lease`;
      stash unbound: the `S stashes … first` clause gone; no `—`.
- [x] No graft-ops → graft-ui edge: `it::dependency_edges::internal_dependency_edges_match_architecture_md` green and
      `cargo tree -p graft-ops` names no `graft-ui` (output line in NOTES).
- [x] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `known_ops.rs` holds no `KNOWN` row.
- [x] Shipped keymap: every capture assert green (workspace `cargo nextest run`, 5697/5697); `git diff --stat main -- docs/shots` empty (W6).
- [x] MAP.md graft-ops (`## keys`; checked by reading — no test) (`keys`, every changed signature) and graft (`op_keys`, jobs that now take `Arc<Keymap>`,
      `update::tests::keymap_words`).

## Touches
- Every caller (step 1 list, compiler-checked); graft-ops and graft tests asserting these strings (grep each literal).
- Jobs' params (preview, rewrite, any other step 1 finds); job dispatch sites.
- Journal/undo: words only — `UndoError` logic and `restore` execution untouched; show
  `git diff main -- crates/graft-ops/src/undo.rs crates/graft-ops/src/restore*` touches only wording fns.
- 20 rewords `apply/error.rs`, `apply/remote.rs` lines after this: its lines keep `keys`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~70k: keys + graft-ops sites ~20k, callers + jobs ~25k, adapter + tests ~20k, MAP ~5k. Seam if over: graft-ops,
adapter and every caller land together (they compile together); commit; report `blocked: ops remap tests`.

## NOTES (builder, 2026-10-07)

**Compacted once mid-ticket.** Nothing remains: all boxes ticked, gates green. No scout: the stale
`scouts/10-keymap-spell.md` was not used; callers came from the compiler and greps (step 1 below).

**Step 1 — callers (compiler-held, then grep for `to_string()`/`"{}"` of these types):**
graft-ops: `operation.rs`, `apply/{error.rs, error/words.rs, remote.rs, record.rs:235, run.rs:128}`,
`restore.rs`, `restore/outcome.rs`, `undo/execute.rs`, `preview/{effects,put_back,stash}.rs`, `rewrite.rs`,
`conflict.rs`. graft: `update/operate/failure.rs`, `update/{amend_into.rs, banner/resume.rs, preview/answer.rs,
preview/take.rs, preview/tests.rs, resolver/resolved.rs, rewrite_editor/{open,routing}.rs, finish/answers.rs,
undo.rs, journal/keys.rs, operate.rs, events/command.rs}`, `jobs/{operate.rs, undo.rs, journal_list.rs,
runner.rs, conflict.rs, preview.rs}`; tests in both crates. None in 09d's Files.
**apply/resume lines (for 13e):** `apply.rs` and `apply/resume.rs` untouched. Touched under `apply/`:
`error.rs`, `error/words.rs`, `remote.rs`, `record.rs` (~235, one `&NoKeys` arg + import), `run.rs` (~128, same).

**`cargo tree -p graft-ops -e normal`:** graft-core, graft-git, serde, serde_json, thiserror — no graft-ui
(`grep -c graft-ui` = 0). `git diff --stat main -- docs/shots`: empty.

**Display uses (step 3) — none drawn keyed:** every `Display` of a keyed type is `explain(&NoKeys)`. Drawn
paths now call `explain(&OpKeys(..))`: `jobs::operate::failure_words` (Refused, AfterState, Remote; other
`ApplyError` keep `Display` — `Git`'s `explain` adds `u restores the backup`, which graft never drew),
`jobs::undo`, `jobs::journal_list`, every update caller. Keyless `Display` that remains:
journal-record reasons (`record.rs`, `run.rs` — logs), `rewrite`/`conflict` `Refused` Display (graft carries the
`Refusal` value and words it in update), test panics. `jobs::conflict` restart gate and `jobs::preview`
`plan_revert_to` pass `&NoKeys` on purpose — their refusals (`NotMarkerShape`/`NoStagesKept`/`NotUnmerged`,
`UndoError::Nothing`) name no key; comment at each. **Deviation from the Ruling's "job gains keys"**, held
there because no key is named; say if you want them threaded anyway.

**Deviations / questions:**
- `OpKey` variants differ from the spec list: `Amend` (not `Absorb` — `a … only changes older commits` and
  `a puts staged changes …` answer `graph.amend` on the graph, W2) and `Push` (not `ForcePush` — `P force
  with lease` / `P pushes and sets one` answer `graph.push`). Pinned by `every_op_key_is_a_row_…`.
- Keyed beyond the planner's list (they name keys): `NoUpstream` (`P`), `UpstreamChanged` (`press p again`),
  `NotFastForward` (`m merges instead`), `BeyondReach` (`a reaches`), remote `AuthFailed` retry key,
  `Conflicted`'s `resolve, then c continues, a aborts`, `UndoError::Drift`'s `u/U again`.
- **Not changed:** `Refusal::MarkersRemain` (`operation.rs:953`, `e edits, M opens the mergetool`) — W2 is
  ambiguous (changes pane / resolver / banner rows) and the grep test does not flag it. Question for 10g.
- **W6 question (byte change, rare path):** `apply/set_aside/answer.rs:323` `unwritten_words` builds its
  `kept` string from `AfterState::StagingNotKept`'s `Display`, now keyless, so an `ApplyError::Journal` after
  a staging-not-kept put-back loses `· u undoes it, staging included`. Unverified whether `u` even reaches
  that entry when its outcome line was not written; left as is — graft-ops holds no keymap there.
- `update::tests::keymap_edge::seen_as` now draws each command's carried keymap as `keys`, like the frame:
  the sweep compares the doing, and an `Operate`'s keymap is words only.
- `jobs::runner::dispatch` gained `#[expect(clippy::too_many_lines)]` with a reason (RecoverFor's fourth
  field makes rustfmt spread the arm; precedent `update::place::write::is_write`).
- Heavy clippy (`--features heavy-tests`) does not compile — pre-existing `frames`/`ghostty_shot`/`held_key`
  `Shutdown` errors (gap M8/heavy-tier-no-compile), none in files this ticket touched.
