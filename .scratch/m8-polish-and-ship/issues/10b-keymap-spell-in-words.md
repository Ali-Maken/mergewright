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

## NOTES (builder, 2026-10-07) — stopped early: scope ~5x the spec, needs a planner ruling

Status: **blocked on scope**. No source changed. Did not compact. Scout used (`scouts/10-keymap-spell.md`).

**Scout check.** All 23 listed sites verified against code at HEAD 80dd116b: line numbers held for every one
(`consequence.rs:415` is inside `#[cfg(test)]` and already spells via `{key}` at :81 — nothing to do there).
The scout's *site list* did not suffice: it listed 23 of ~100.

**Why stopped.** Extended 10's grep test to scan `crates/graft/src` and `crates/graft-ops/src` (patch:
`10b-grep-extension.patch`, a by-path scan with `graft:`/`graft-ops:` allow-list prefixes, no new dependency).
It reports **138 non-test hits** (`10b-grep-hits.txt`): ~37 false (article `a`, counts `1 file`), **~100 real
literal keys** — e.g. `banner/pressed.rs "c continue  a abort"`, `resolver/whole_file.rs` (7× `o keeps … t …`),
`operate/failure.rs "e edit  M mergetool"`, `"S stash {count} changed"`, `view/credential.rs`, `view/status.rs
"esc dismiss"`, `undo.rs "U redoes"/"J journal"/"⇥ banner"`, `search.rs "n/N next/prev  Esc …"`, graft-ops
`restore.rs "U restores the backup"`, `apply/remote.rs` p/m/r/P/f clauses, `operation.rs` C/R/space/a/c clauses.
Acceptance 3 ("grep test covers graft + graft-ops and is green") cannot be met by the 23 sites: it needs ~100
more fixed or ~100 KNOWN allow-list rows. Also needs: skip `*_tests.rs` files (the scanner only skips
`tests.rs`/`tests/`; 23 hits come from `*_tests.rs`).

**Design question for the planner (graft-ops).** The `operation.rs` sites live in `Refusal::explain(&self)`,
called at ~40 sites incl. job threads (`jobs/preview.rs:70`, `jobs/rewrite.rs`) that hold no keymap. Rule 2
("graft appends the clause where it shows the error") needs the variant at the show site; job paths flatten to
`String` first. Options: (a) `Refusal::explain_with(&self, key: &dyn Fn(&str) -> Option<String>)` + `explain()`
keeping default letters (allow-listed) for keymap-less callers; (b) pass `Arc<Keymap>` into the jobs that word
refusals; (c) keep `Refusal` values to `update` and word there. Pick one before building.

**Proposed split.** 10b-1: grep extension with `*_tests.rs` skip + allow-list every current hit as KNOWN per
file group (green, ratchets). 10b-2: the 23 graft `update` sites listed above (mechanical). 10b-3: graft-ops
refusal/error words after the ruling. 10b-4..n: the ~80 extra graft sites by area (banner, resolver, undo/journal,
failure/outcome, view/*). Leave 09d's 5 KNOWN graft-ui rows alone.
