# G-keymap 2b — status, refusal and error words spell keys from the live table

Status: ready (split from 10 by the planner's spec-only pass 2026-10-04; scout `scouts/10-keymap-spell.md`)
Build with: **Sonnet** — every site is listed, the rule for each is decided below, and each is held by a remap test;
no design judgement left. Not Fable.
Blocked by: 10, 10c
Needs: from 10 — `Keymap::{spell, spell_keys}`, `AppState::keymap: Arc<Keymap>`, the graft-ui grep test (extended
here). From 09 — `ActionId`, `defaults()`.
Crate set: graft (`update` word sites below) + graft-ops (word sites below). Not graft-ui.

## Read first
- `.scratch/m8-polish-and-ship/issues/10-keymap-spell-from-the-table.md` §Decisions (2, 3) and §Signatures
- `.scratch/m8-polish-and-ship/scouts/10-keymap-spell.md` §"Status/refusal words" and §"graft-ops"
- `crates/graft-ui/MAP.md` `## keymap`
- `crates/graft/MAP.md` for each `update` module named below; `crates/graft-ops/MAP.md` for `operation`,
  `apply::remote`, `apply::error`

## Scope
In:
- graft `update` words that name a key, each spelled with `state.keymap` (or a `&Keymap` parameter where the word fn
  has no state): `update/remote.rs:285,349`; `quit.rs:103`; `clash.rs:297-298`; `resolver.rs:399`;
  `operate/diff_cursor.rs:180,297`; `preview.rs:130`; `preview/answer.rs:338`; `preview/take.rs:83`;
  `preview/line_rows.rs:228`; `place/answer.rs:181,184`; `rewrite_editor/words.rs:14`; `undo.rs:756`;
  `undo/consequence.rs:415` (`U`; follow :81's `{key}` form).
- graft-ops words (`operation.rs:908,924,1047,1055,1154`; `apply/remote.rs:330`; `apply/error/words.rs:38`).
  **graft-ops never imports graft-ui** (crate rules). Rule per site:
  - a word fn → gains one `&str` parameter per key it names, named after the action (`pull_key: &str`); graft passes
    the spelled label;
  - a `Display`/`words()` on an error type that can't take a parameter → the key clause leaves graft-ops; graft
    appends it (` · <key> <verb>`) where it shows that error. Same bytes under defaults.
  NOTES names which rule each site took.
- An unbound key (10 decision 2): the clause naming it is left out, never `—`. E.g. `c commit  S stash` with stash
  unbound → `c commit`.
- Extend 10's grep test to graft's and graft-ops' user-facing string literals; allow-list entries carry a reason.
Out:
- `operation.rs:920` ("press the key again", names no key). Rewording anything (20). Docs (10 decision 8).

## Signatures
No new public types. graft-ops word fns gain `&str` key parameters (names in NOTES and MAP.md); no graft-ops
dependency added (check `cargo tree -p graft-ops` names no graft-ui).

## Acceptance
- [ ] Remap test, table-driven, one row per site above: under a keymap moving that site's action(s) to a distinctive
      free chord, the produced line holds the new label and not the default as a whole token; under defaults the
      line is byte-identical to today's (assert the literal). Test name at tick; row count stated.
- [ ] Unbound test: at least the `c commit  S stash` and `p pull first` sites with their key `none` drop the clause
      (test name at tick).
- [ ] Grep test (10's) now covers graft and graft-ops and is green (test name at tick).
- [ ] Every capture assert green; `git diff --stat main -- docs/shots` empty (no capture changes in this ticket).
- [ ] MAP.md graft and graft-ops: changed fn signatures.

## Touches
- Every caller of a changed graft-ops word fn (grep each); tests asserting these strings (grep each literal).
- 20 (words sweep) edits the same strings after this. 11 and 16 run in graft too: the conductor checks file sets
  (this ticket touches no `config/`, no `update/palette`, no `update/mod.rs` unless a site lives there).
- Journal/undo: words only; `undo.rs`/`consequence.rs` logic untouched — say how checked.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~45k: 23 sites ~20k, remap test ~15k, grep extension ~5k, MAP ~5k.
