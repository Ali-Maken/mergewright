# G-keymap 1b — remaps translated at the app's edge, read from `[keys]`

Status: done (2026-10-04, branch `m8-09b`; builder compacted once — see NOTES)
Build with: **Opus** — the approach and every rule are fixed by 09; correctness is judged by the behavioural sweep
below, not by eye. Not Fable: fails "can't be judged from the diff or the tests". Scout: 09's covers this code
(`scouts/09-keymap-table.md`); a fresh one only if `update/mod.rs` moved since eeaf6a19.
Blocked by: 09
Needs: from 09 — `ContextId` (+ `scope`, `is_text_field`), `KeyChord::{from_press, to_press}`, `Keymap::{default,
with_overrides, translate}`, `Translated`, `KeyWarning::words`; 09's NOTES (`NOT_YET_ANSWERED`).

## Read first
- `docs/adr/0055-keys-are-one-table-and-remaps-translate-at-the-edge.md` §3–4 and Consequences
- `.scratch/m8-polish-and-ship/issues/09-keymap-table-and-translate.md` Scope 4–5 and its NOTES
- `docs/design/02-keymap.md` §2.1 bullet "Every binding is remappable" (:174)
- `docs/design-notes/m8-polish-and-ship.md`: Journeys "Where he lands", row "Startup with `[keys]`/`[theme]` problems"
- `.scratch/m8-polish-and-ship/scouts/09-keymap-table.md` (whole)
- `crates/graft/MAP.md`: `config` (`Keys`, `load`, `ConfigWarning`), `AppState::text_field_open`, `update::keys` files,
  `places::drive`, `empty_state` (`outside_key`), `modal_owns_bottom_row`, `acknowledge`
- Source, after the maps: `update/mod.rs:1101–1131`, `config/load.rs:177–230`, `config/types.rs:40`,
  `event_loop/places.rs:279`, `event_loop/startup.rs:550`, `update/state.rs:847–860`

## Scope
In (crate set: `graft` only):
1. **Config.** `Keys` keeps `suspend_on_ctrl_z` (a bool at `[keys]`'s top level) and gains `remaps`: each sub-table
   entry `[keys.<ctx>] <verb> = "<chord>" | ["<chord>", …]` becomes `("<ctx>.<verb>", chords)`, in file order (say in
   NOTES whether the `toml` table keeps file order; if not, sort by id and say so). The typed parse must accept the
   sub-tables (the scout could not confirm `deny_unknown_fields` — check). `SCHEMA`/`unknown_keys` (load.rs:195/216)
   stop warning `keys.<table>`; any other non-bool entry straight under `[keys]` still warns as unknown. A sub-table
   value that is neither a string nor a list of strings → a `ConfigWarning` naming `keys.<ctx>.<verb>`. An unknown
   `<ctx>` is passed through; the keymap says `UnknownAction`.
2. **One keymap per process.** `keymap_from(&Loaded)` builds it and the problem words: existing `ConfigWarning`s in
   their current words, then each `KeyWarning::words`. `AppState` gains `keymap`, `config_problems`; startup
   (startup.rs:550, where `suspend_on_ctrl_z` is copied today) fills both; `drive` passes the same keymap to the
   outside screen. `AppState::new`/test fixtures: `Keymap::default()`, no problems.
3. **The edge (in a repo).** First thing in `core_event`'s `Event::Key` arm (update/mod.rs:1101), ahead of
   `credential_key`:
   - `state.text_field_open()` → the press goes on untranslated (that predicate already names every field the scout
     listed: search, place prompt, J find, commit editor, stash prompt, credential, name prompt, shown picker, rewrite
     field, finish off its file list). Typed text is never remapped (§2.1).
   - else `KeyChord::from_press` (`None` → untranslated), then `translate(context_of(state), chord)`: `Pass` → as
     pressed; `Default(d)` → `d.to_press()`; `Unbound` → **exactly what a key no dispatcher answers does today** (if
     `acknowledge` runs on such a key, it runs here too; nothing else, no status).
   - Everything after the translate moves, unchanged, into `dispatch_key` — the entry 11's palette calls to run an
     action by its **default** chord (through `translate` a moved default would be `Unbound`).
   - The harness `hand_back` check (:1118) is a test-harness hook, not a binding: it reads the raw press. Say in NOTES
     what it is and that no binding can reach it.
   - Mouse is not translated: the `.` menu acts through `apply_context_action`, not keys (scout). The comment at
     `mouse.rs:91` ("handed back down key_event") does not match the code: report in NOTES, don't edit.
4. **`context_of(&AppState) -> ContextId`**, in the order the chain claims keys (scout's chain): credential prompt →
   `Credential`; drag → `Graph`; confirm box → `Confirm`; preview → `Preview`; rewrite editor → `Rewrite`; finish →
   `Finish`; resolver → `Resolver`; picker shown → `Picker`; banner focused → `Banner`; blame / history place shown →
   `Blame` / `History`; `.` menu → `Menu`; commit editor → `CommitEditor`; Journal / Stash / Map overlay with the graph
   focused → that overlay; detail pane focused → `Changes` on the uncommitted row, else `Diff`; else `Graph`. A text
   field over a place → that place (the field is passed before `context_of` matters). Never `Tour`; `Palette` is
   11's to add. Where the chain and this order disagree, follow the chain and say so in NOTES.
5. **The edge (outside a repo).** `places.rs:279`, before `outside_key`: a prompt open (`Prompt::Open`, and 18's clone
   prompt later) → untranslated; else `translate(ContextId::EmptyState, …)`, same three answers.
6. **Startup status.** Problems → the bottom row reads `config: 1 problem · ? shows them` / `config: N problems · ?
   shows them`, muted, in place of the hints, until the first key or click (the lifetime `acknowledge` gives a `Done`);
   a failure wins over it. Zero → nothing drawn (CLAUDE.md status rule). It replaces today's display of
   `ConfigWarning`s on the first repository screen; tests pinning the old display are updated and named in NOTES.
   The outside screen shows none (unchanged). Until 11 lands, `? shows them` is not yet true: the conductor holds a
   KNOWN_GAPS row (broken, closed by 11).
7. **Tests.**
   - **Behavioural sweep** (ADR-0055 consequence): a keymap remapping every movable action to its own free chord (09's
     sweep set). For every `ContextId` with a state (all but `Tour`, `Palette`; `EmptyState` through `outside_key`),
     a fixture; for every row of `in_context(ctx)` and of `Global`: the free chord pressed in one copy and the default
     pressed in another under the default keymap give the same result; every default chord under the remapped keymap
     gives what an unanswered key gives. "Same result" = one projection the builder names (emitted commands + drawn
     frame text, or a state dump without clocks), the same for every case.
   - **Every row answered** (replaces the old criterion: `bound_actions` has no context and can't drive it — scout):
     default keymap, each context's fixture, each row's default changes the projection, or is listed in
     `NOT_YET_ANSWERED` (id → ticket, from 09's NOTES; 11/18/19 delete their own entries) or `NO_EFFECT_IN_FIXTURE`
     (id, why — e.g. `L` with nothing truncated). Each entry a reason; no bare exemptions.
   - **Typed text**: with `graph.merge = "x"`, typing `x` in each field `text_field_open` names, and in the empty
     state's open prompt, puts `x` in the field.
   - **J8 through `load_from`**: a temp config with `suspend_on_ctrl_z = true` and `[keys.graph] merge = "M"` →
     `config_problems.len() == 1`, the bottom row reads `config: 1 problem · ? shows them`, no unknown-key warning for
     `keys.graph`, `M` on a branch row opens the merge preview, `m` acts as an unanswered key, `suspend_on_ctrl_z`
     read true.
   - **Outside**: `[keys.empty_state] open = "O"` → `O` opens the path prompt, `o` does nothing, `o` typed in the
     prompt is an `o`.
   - `context_of`: one test per reachable `ContextId`.
Out:
- Hint/label spelling from the table (10). `?` and the palette, `YOUR CONFIG` (11). Config problems on the empty
  state. README config section (23).

## Signatures
```rust
// graft::config
pub struct Keys { pub suspend_on_ctrl_z: bool, pub remaps: Vec<(String, Vec<String>)> }
pub(crate) fn keymap_from(loaded: &Loaded) -> (graft_ui::keymap::Keymap, Vec<String>);   // (keymap, problem words)
// graft::update
pub(crate) fn context_of(state: &AppState) -> graft_ui::keymap::ContextId;
pub(crate) fn dispatch_key(state: &mut AppState, key: graft_core::event::KeyPress) -> /* the Key arm's type today */;
// AppState gains `pub(crate) keymap: Keymap`, `pub(crate) config_problems: Vec<String>`
```
Return type of `dispatch_key`: whatever `core_event`'s `Event::Key` arm yields today; name it in MAP.md.

## Acceptance
- [x] Behavioural sweep green over every `ContextId` with a state — `update::tests::keymap_edge::the_sweep_moves_every_action_and_every_context_follows`
      (18 repo contexts with a fixture; asserts > 300 rows swept). `EmptyState`: `empty_state::tests::open_moved_to_capital_o_outside_a_repository`
      + `the_default_keymap_outside_is_every_key_as_pressed` (one remap and the default identity, not a full sweep — see NOTES).
- [x] Every-row-answered green; each exemption has a reason — `update::tests::keymap_edge::every_row_is_answered_in_its_context`
      (`NOT_YET_ANSWERED` 5, `NO_EFFECT_IN_FIXTURE` 18; the test also fails on an exempt row that *is* answered).
- [x] Typed-text test per field, the empty state's prompt included — `update::tests::keymap_edge::x_typed_into_every_field_is_an_x_with_merge_on_x`
      (10 fields), `empty_state::tests::x_typed_into_the_open_prompt_is_an_x_with_merge_on_x`.
- [x] J8 through `load_from` — `update::tests::keymap_edge::j8_a_keys_table_beside_suspend_moves_merge_and_says_one_problem`;
      real binary over a pty: `it::keymap_config::a_keys_table_in_the_config_file_reaches_the_first_screen`. Person step stays accept-m8's.
- [x] Outside remap test — `empty_state::tests::open_moved_to_capital_o_outside_a_repository`.
- [x] `suspend_on_ctrl_z` still read: `event_loop::startup::tests::keys_suspend_on_ctrl_z_reaches_app_state` green (full run).
- [x] Default config: every existing capture assert green, none re-blessed — full workspace `cargo nextest run`; `git diff --stat main -- docs/shots`
      shows only the new file. Notice gone/absent: `update::tests::keymap_edge::the_config_notice_goes_at_the_first_key_and_needs_a_problem`.
- [x] Capture `docs/shots/m8-keymap-config-problem-120x36.txt` (4.7 KB) — written by the J8 test above. **Not pty**: see NOTES.
      **Colour unverified** (rule 5); muted token not checked. Looks-right is the user's call.
- [x] `crates/graft/MAP.md`: `Keys::remaps`, `keymap_from`, `ConfigWarning::NotAKey`, `update::edge` (+ `context_of`, `SharedKeymap`),
      `dispatch_key`, `before_any_key`, `config_problems`/`config_notice`/`seed_keys`, `outside_edge`, `places::drive`.

## Touches
- `core_event` Key arm and every dispatcher behind it (input only; no dispatcher edited). Harness `hand_back`. Mouse
  (not translated; checked by its existing tests). Palette (11) calls `dispatch_key`.
- Config: `types.rs` `Keys`, `load.rs` typed parse + `SCHEMA` + `unknown_keys`; startup copy (:550) and its test (:640).
- `places.rs`, `empty_state.rs` (outside edge); 18's clone prompt must join the "prompt open" check.
- Bottom-row rungs (`modal_owns_bottom_row`, `acknowledge`): the new line's lifetime; captures under default config
  unchanged. Journal/undo: untouched (keys only). MAP.md graft.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~120k: config ~15k, edge + `context_of` ~25k, outside + status ~15k, fixtures and the two sweeps ~50k.
If the fixtures alone pass ~50k, land 1–6 with J8, typed-text and outside tests, commit at the seam, and report
`blocked: sweeps` — the conductor gives the two sweeps to an Opus follow-up.

> Conductor (2026-10-04), inputs from 09 (see 09 NOTES): name prompt has no ContextId; `translate` passes every key in text-field contexts (09 builder decided, spec silent); spelling exemptions `release`, `type`, `tab` go to ticket 10; gap M8/ctrl-shift-c-legacy.

## NOTES (builder, 2026-10-04)

**Compacted: yes, once**, mid-build (after the edge and the sweep fixtures; before the every-row exemptions, the
typed-text/J8/outside/notice tests, capture, MAP, gates). Nothing of scope remains; all of it landed after the compaction.
Scout: none fresh — 09's two scouts covered the code; `update/mod.rs`'s Key arm was where the scout said.

- **Order of remaps.** `toml` is built without `preserve_order` (`crates/graft/Cargo.toml:55`), so tables come out
  sorted: `Keys::remaps` is sorted by id, not file order. A collision's winner follows that order. Said in `Keys`' doc.
- **Typed parse.** No `deny_unknown_fields` (load.rs:178 says why); serde ignores `[keys.<ctx>]`. `remaps` is
  `serde(skip)`, filled from the raw table by `config/keys.rs::remaps`. `SCHEMA`'s `keys` row removed and
  `unknown_keys` skips `keys`; `keys.rs` warns a non-table straight under `[keys]` as unknown (`keys.<name>`),
  a non-text value as the new `ConfigWarning::NotAKey { path }`.
- **`SharedKeymap`** (`update/edge.rs`): `AppState` derives `Clone, Debug, Eq`; `graft_ui::keymap::Keymap` derives none,
  so `AppState.keymap` is an `Arc` wrapper (equal = same Arc or same chords per row). **Proposed:** derive
  `Clone, Debug, PartialEq, Eq` on `Keymap` in graft-ui (not my crate) and drop the wrapper's impls.
- **`dispatch_key` signature** gains `commands`: `(&mut AppState, &mut Vec<Command>, KeyPress)` → `()` — the Key arm
  mutates state and pushes commands; it has no return value. 11's palette calls it.
- **`hand_back`** (`update/mod.rs:1108`): a test-harness hook that ends the run with `Quit::HandedBack(key)` on listed
  key codes. It reads the **raw** press (`dispatch(next, commands, key, raw)`), so no remap reaches or dodges it; no
  binding names it.
- **`mouse.rs:91`** comment says the menu's letters are "handed back down `key_event`"; the code acts through
  `apply_context_action`. Not edited (ticket said report only).
- **Unbound = what F20 does** (`edge::unanswered`): `keys::before_any_key` (extracted from `key_event`: dismiss an unseen
  failure / `acknowledge`), drop a badge drag, close the `.` menu in `Menu`. **Deviation:** a shown blame/history place
  runs `keep_in_view` on *any* key it gets (`place_key`'s fall-through); the edge's unbound key does not reach it. In a
  settled place that is no visible change; on a place not yet scrolled to its cursor, the first unbound key would not
  scroll it. The sweep's Blame/History fixtures are settled by one F20 first, and say why. Cause, not symptom: the
  alternative is calling into the place from the edge for a no-op in every real state.
- **Text fields pass every key** (09's decision kept): the edge checks `text_field_open()` before translating, so it
  never matters whether `translate` would. The empty state's open prompt does the same in `outside_edge`.
- **Config warnings moved** from `Session` notes to `AppState::config_problems`: shown on F2's status screen
  (`view::status::notes`, between git notes and session notes) and counted on the bottom row. Test renamed
  `session::tests::config_warnings_become_notes_and_are_sanitised` → `config_warnings_are_not_session_notes` (asserts
  notes empty). Sanitising now happens in `view::status::notes` and `bars::footer_notice` (`sanitize_line`); **no test
  pins sanitising of a config problem on F2** — unverified.
  `places::drive` lost its `quiet`/`told` logic: the first repository place gets the problems, later ones none.
- **Notice priority** (`view/frame/bars.rs::footer_notice`): after `interrupted_notice`, before the hints; a bottom-row
  problem (failure) is checked first and wins. Cleared by `acknowledge` (first key or click).
- **`context_of` vs the chain:** followed the chain. One nuance: J/Stash/Map count only with the graph focused
  (`keys::pane_and_overlay_key`), so a focused detail pane over an overlay is Changes/Diff.
- **Visibility changes for tests:** `update::commit_editor::tests` and `config::tests` are `pub(crate)` (fixtures shared
  with `keymap_edge`); `config::tests::loaded_from_a_file` writes the J8 file because `update`'s purity scan
  (`update_cannot_reach_the_world`) forbids `std::fs` anywhere under `update/`, tests included.
- **AppState grew** (3 fields): two existing tests tripped clippy's 16 KB local-array lint (`live_c_tests.rs:456`,
  `bottom_row/unseen.rs:147`); now boxed slices, same assertions.
- **Capture not pty.** A pty run reads the real clock, so ages drift and the shot could never compare. The shot is
  drawn by the real `view` from the J8 test's state (file → `load_from` → `keymap_from` → `seed_keys`) at 120×36 with
  the fixtures' fixed clock. The startup wiring through the real binary is asserted separately (pty test above), not
  captured. Colour unverified.
- **NO_EFFECT_IN_FIXTURE reasons**: 16 checked against the handler (named in each entry); `diff.word_diff` and
  `blame.heatmap` flip state that draws only as colour, which the text frame cannot show (rule 5) — **unverified** that
  they draw at all, only that the state flips (`detail/keys.rs:392`, `place/keys.rs:59`).
- **Outside sweep:** only one remap + default identity over 8 keys, not every EmptyState row moved; the full sweep is
  repo-only. Gap for the conductor if wanted.
- Touches checked: full workspace nextest 5474/5474; `git diff main -- docs/shots`: only the new shot is mine
  (`m8/fork-follows-rewrite-editor-187x44.txt` differs because main moved with 14d after this branch).
