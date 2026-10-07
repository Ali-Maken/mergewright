# G-keymap 2e — graft-ops words name keys by what they do; graft spells them

Status: draft (planner spec-only pass 2026-10-07; split from 10b — its **Ruling**)
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
- [ ] `update::op_keys::tests::every_op_key_is_a_row_and_spells_todays_letter` — each `OpKey::ALL` maps to a row of
      `defaults()`, W2-pinned (ctx stated per row), and `OpKeys(&Keymap::shipped())` spells the letter graft-ops
      wrote before (table of literals).
- [ ] `update::tests::keymap_words::ops::each_ops_word_spells_the_remapped_key` — one row per site above (count
      stated): under the sweep keymap the line holds the remapped label and no default label of a moved action as a
      whole token; under shipped, byte-identical to the literal.
- [ ] `…::ops::an_unbound_key_drops_its_clause` — `graph.merge = "none"`: `… · p pull, then r · P force with lease`;
      stash unbound: the `S stashes … first` clause gone; no `—`.
- [ ] No graft-ops → graft-ui edge: the existing `cargo metadata` edge test green (name at tick) and
      `cargo tree -p graft-ops` names no `graft-ui` (output line in NOTES).
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green; `known_ops.rs` holds no `KNOWN` row.
- [ ] Shipped keymap: every capture assert green; `git diff --stat main -- docs/shots` empty (W6).
- [ ] MAP.md graft-ops (`keys`, every changed signature) and graft (`op_keys`, jobs that now take `Arc<Keymap>`,
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
