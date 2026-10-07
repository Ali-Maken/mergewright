# G-keymap 2b — the literal-key grep reaches graft and graft-ops (a ratchet)

Status: draft (re-specced by the planner's spec-only pass 2026-10-07 after the builder's stop; was: the 23 word
sites, now split across 10e–10i — see **Ruling** below)
Build with: **Sonnet** — the scanner change exists as a patch (`10b-grep-extension.patch`), the file layout and the
classification rule are decided below; held by the test itself (stale rows fail). Not Fable.
Blocked by: none (10, 10c landed).
Needs: from 10/10c — `graft-ui/tests/no_literal_keys.rs` (`no_user_facing_literal_spells_a_default_key`, `literals`,
`shipped_part`, `hint_at`, `ALLOWED`).
Crate set: graft-ui **tests only**.
**Files:** `crates/graft-ui/tests/no_literal_keys.rs` (scanner fns only: `roots`, `source_path`, the file filter; the
existing `ALLOWED` rows untouched) + new `crates/graft-ui/tests/no_literal_keys/{prose,known_ops,known_stops,
known_resolver,known_undo,known_preview}.rs`. No `src/`, no graft, no graft-ops file.
**Scheduling:** disjoint from 16 (theme/config) — may run now. 09d edits `tests/no_literal_keys.rs`'s `ALLOWED` rows:
**not concurrent with 09d** — merge before 09d starts (09d rebases; its rows are untouched here) or after it merges.

## Read first
- This file's **Ruling** (whole) and the builder NOTES below
- `10b-grep-extension.patch`, `10b-grep-hits.txt` (beside this file)
- `10-keymap-spell-from-the-table.md` §Acceptance "Grep test" (what a hint position is)

## Scope
In:
- Apply the patch (`roots()`, `source_path()`, `graft:` / `graft-ops:` prefixes; by path, no new dependency).
- The file filter skips `*_tests.rs` as well as `tests.rs` and `tests/` (23 hits today are test files).
- The graft/graft-ops allow-list lives in six files under `tests/no_literal_keys/`, each `#[path = …] mod` from the
  root (no rename of the root — cargo does not build files in a test's subdirectory as targets), each exporting
  `pub const ROWS: &[(&str, &str, &str)]`; the test checks `ALLOWED` and all six. Stale check covers every table.
- Regenerate the hit list at HEAD (the attached one is from 80dd116b) and classify every hit:
  - `prose.rs` — **not a key**, permanent, reason per row, one of: `article` (`a write is running`), `count`
    (`1 commit`), `not shown: <assert|panic|debug>` (a message he never sees), `description` (`delete branch`, a
    journal record's words). A literal holding a real key anywhere (`a stash isn't a branch · ⏎ opens it`,
    `in progress — c continue  a abort`) is **not** prose.
  - every other hit → `KNOWN: <ticket>` in its area file, by path:
    | File | Paths | Ticket |
    |---|---|---|
    | `known_ops.rs` | every `graft-ops:` path | 10e |
    | `known_resolver.rs` | `graft:update/resolver*`, `graft:view/resolver*` | 10g |
    | `known_stops.rs` | `graft:update/{banner,commit_editor,operate,finish}*`, `graft:update/mod.rs`, `graft:view/journal/summary.rs` | 10f |
    | `known_preview.rs` | `graft:update/preview*` except `preview/boxes.rs`, `graft:update/remote*`, `graft:update/rewrite_editor*`, `graft:view/rewrite_editor*` | 10i |
    | `known_undo.rs` | every other `graft:` path | 10h |
- The test's doc comment states the ratchet: a new literal key anywhere in the three crates fails at once; a fixed
  literal still listed fails as stale, so each area ticket deletes its own rows.
Out: fixing any site (10e–10i). graft-ui's own `ALLOWED` rows (09d).

## Signatures
None in `src/`. Test-local: `fn roots() -> Vec<(&'static str, PathBuf)>`, `fn source_path(path: &str) -> PathBuf`
(as the patch).

## Acceptance (each tick names its test, at tick time)
- [ ] `no_literal_keys::no_user_facing_literal_spells_a_default_key` green over graft-ui, graft, graft-ops.
- [ ] A planted literal (`"u undoes"` in a scratch graft file, not committed) fails it naming `graft:<path>`; a
      planted `*_tests.rs` file does not — say how checked (two runs, output lines quoted in NOTES).
- [ ] NOTES: hit count at HEAD, rows per file (prose / each `known_*`), and every graft path a `known_*` file holds
      that is **not** in that ticket's Files list (the conductor amends the ticket).
- [ ] No `src/` change: `git diff --stat main -- crates/*/src` empty.

## Touches
- 09d (same root file, `ALLOWED` rows only — scheduling above). 10e–10i each delete their `known_*` rows.
- 20 (words sweep) rewords literals listed here: a reworded literal leaves its row stale — 20 deletes it.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~25k: patch ~3k, filter ~2k, classify ~100 hits ~15k, NOTES ~5k.

## Ruling (planner, spec-only pass 2026-10-07)

**Refusal::explain — none of (a), (b), (c) as written. Take: a typed key vocabulary in graft-ops + (b)'s plumbing.**
graft-ops may depend on `git`, `core` only (ARCHITECTURE.md crate table); `ui` is never above it, so it can't name
`Keymap` or `ActionId`.
- (a) `explain_with(&dyn Fn(&str) -> Option<String>)` + a default-letter `explain()`: **reject.** The `&str` is an
  action id graft-ops can't check, and the keymap-less twin is the lie this work removes, kept on an allow-list.
- (c) `Refusal` values back to `update`, words there: **reject.** The words live with the type (`explain_rungs`'
  width ladder, ADR-0049 amendment; `UndoError::explain`); moving them is a refactor of ~40 callers and job result
  types for no gain.
- **Decided (10e):** graft-ops gets `OpKey` — what the key does (`Commit`, `Abort`, `Pull`, …), never a chord — and
  `trait KeyNames { fn label(&self, OpKey) -> Option<String> }`. Every graft-ops wording that names a key takes
  `keys: &dyn KeyNames`; **no keymap-less variant** (a `Display` that can't take one uses `NoKeys`: keyless words,
  never a wrong key). graft maps `OpKey → ActionId` in one exhaustive `match` and spells from `state.keymap`. A job
  that flattens to `String` on its thread holds `Arc<Keymap>` cloned at dispatch (option (b) — `app` may name
  both); a job whose result already carries the value (`PreviewFailure::Refused`, `RewriteOpened::Refused`) is worded
  in `update` instead, result types unchanged.

**Split** (builder's proposal, with a test home per area so area tickets share no file but MAP.md):
10b grep ratchet (this) → 10e graft-ops words + adapter + callers + test scaffold → 10f stops and outcomes, 10g
resolver, 10h undo/journal/recover + chrome, 10i preview/remote/rewrite (any two at once).

**Word rules for 10e–10i (W1–W6):**
- **W1 — spell where shown.** graft sites spell with `state.keymap` through graft-ui's `spell_keys` / `key_verb` /
  `after`; a word fn with no state gains a trailing `keys: &Keymap`. graft-ops words through `OpKeys(&state.keymap)`.
- **W2 — the row is the one that answers where he presses it.** Pin each site:
  `Keymap::shipped().translate(<ctx>, <default chord>) == <id>`, ctx = `context_of` the state the words show in (or
  the place they send him — NOTES says which). Mismatch or two candidates → stop; NOTES names the handler's
  `file:line`; report, don't pick.
- **W3 — unbound (10 decision 2).** A clause is the text between ` · ` separators or two-space gaps; a clause whose
  key is unbound is dropped whole; alternatives (`then m or r`) keep the bound ones; none bound → clause dropped; a
  line that is only a key clause draws nothing. Never `—`.
- **W4 — text fields (10 R4).** Words drawn while a text field is open spell from `Keymap::shipped()`. A key with no
  row stays in its `known_*` file with reason `text field, no row: <file:line>` (not `KNOWN`).
- **W5 — sequences** (`S p puts them back`, `r then R opens it`): each key from its own place's row.
- **W6 — bytes.** Shipped keymap → every line byte-identical, no capture change, except 10 decision 3's
  normalisations (`Esc` → `esc`, …): re-shoot (`GRAFT_UPDATE_SHOTS=1`), NOTES lists file and token.

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
