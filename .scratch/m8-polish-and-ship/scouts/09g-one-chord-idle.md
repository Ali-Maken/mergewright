# Scout 09g — ONE_CHORD_IDLE, the five C chords, resolver `u` (main @ e80f5a63)

Paths under `crates/graft/src/update/` unless stated. Code, not intent.

## (3) Resolver `u`/`U` arm — the root file, not 10j's
- `resolver.rs:343` `fn act(`; `:379` `KeyCode::Char('u' | 'U') if plain => undo_answered(state)`; `:380` `_ if plain => scroll_key`; `:383` `_ => {}` (non-plain, incl. ⌃Z, swallowed); `:393` `fn undo_answered`.
- `resolver/answers.rs`, `resolver/resolved.rs`: no `undo_answered` / `u` arm (`grep -rn undo_answered` → only resolver.rs:379,393). Submodules declared resolver.rs:41–56; tests at `:435 mod tests` (`resolver/tests.rs` + `resolver/tests/`).
- Overlap with 10j: none on source for `u`. 10j holds `resolver/answers.rs` (`open_failed`), `resolved.rs`, `mod.rs` (`mergetool_returned`), `banner/{resume,pressed}.rs`, `status_read.rs` (10j Files :24–33). `update/mod.rs` is 10j's — 09g must not touch it.

## (2) Does the edge translate Home/End/⌃Z to `g`/`G`/`u` first? No.
- `edge.rs:18` `key`: text field → raw dispatch (:21–24); else `translate` (:30) → `Pass` → `dispatch_key` raw (:31); `Default(d)` → dispatch `d` (:32–33).
- graft-ui `keymap/resolve.rs:139–165`: a pressed chord that is itself one of the action's shipped chords and resolves to it returns `Pass` (:156–157, :163). Home/End/⌃Z are shipped chords of `app.top/bottom/undo`, so they reach the handler **raw**, by design of translate. Loss point is therefore the handler arm, not the edge:
  - Blame: `place/keys.rs:57` `g`, `:58` `G`, `:78` `_ => {}`.
  - Changes: `detail/keys.rs:307` `g` arm only, no `Home`.
  - Resolver ⌃Z: `resolver.rs:383` `_ => {}`.
  - Banner ⌃Z: `banner/focus_keys.rs:147` `u`/`U` arm, `:173` `_ => {}`.
- Could not confirm without running: that no earlier dispatcher in `mod.rs:1083–1123` (credential/palette/find/search) claims these presses at those places.

## (1) The 27 rows — `update/tests/keymap_edge/sweep_tables.rs:136–164`
Tuple is `(ContextId, &str id, &str chord, &str why)` (4 fields; ticket's sketch adds a 4th `IdleWhy` — today the 4th is a free string). Each row's `why` cites its idle line (not re-verified per row except the five C rows above):
- C rows (5): :137 Changes home (detail/keys.rs:307); :138 Resolver ⌃Z (resolver.rs:383); :139 Banner ⌃Z (focus_keys.rs:173); :144 Blame home, :145 Blame end (place/keys.rs:78).
- Picker (6): :140 ⌃Z (picker.rs:155), :141 ↑ (:138), :142 home, :143 end (:155), :160 ⌃P (`R_PICKER`).
- CommitEditor (6): :146 ⌃Z (commit_editor/keys.rs:110), :147 ↑ (:231), :148 home (:229), :149 end (:230), :161 ⌃P (`R_EDITOR`).
- Credential (11): :150–154 `credential.edit_text` delete, ←, →, home, end (credential.rs:379–382); :155 ⌃Z (:326); :156 ↓, :157 ↑ (:385); :158 home (:381); :159 end (:382); :162 ⌃P (`R_CREDENTIAL`).
- Menu (1): :163 `changes.stage` `s`.
- Contradiction for R1: of the 22 non-C rows, **none is a plain printable character** except Menu `s` (:163); all others are ↑, ↓, ←, →, home, end, delete, ⌃Z, ⌃P. `TypesItself` as defined (idle chord is a printable the field inserts) would cover at most 0–1 rows; the "u is typed / j is typed" notes refer to the *other* chord, not the idle one.
- Picker and CommitEditor: whether `ctx.is_text_field()` is true for them — not checked; could not confirm.
