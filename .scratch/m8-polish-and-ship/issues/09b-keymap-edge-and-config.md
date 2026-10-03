# G-keymap 1b — remaps translated at the app's edge, read from `[keys]`

Status: ready (split from 09 on 2026-10-04, after scout `scouts/09-keymap-table.md`)
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
- [ ] Behavioural sweep green over every `ContextId` with a state (test named at tick).
- [ ] Every-row-answered green; each exemption has a reason (test named at tick).
- [ ] Typed-text test per field, the empty state's prompt included (tests named at tick).
- [ ] J8 through `load_from` (test named at tick); person step stays a TESTING.md row (accept-m8's).
- [ ] Outside remap test (named at tick).
- [ ] `suspend_on_ctrl_z` still read: `keys_suspend_on_ctrl_z_reaches_app_state` (startup.rs:640) green.
- [ ] Default config: every existing capture assert green, none re-blessed (the status line is absent with no problems).
- [ ] Capture `docs/shots/m8-keymap-config-problem-120x36.txt` (pty, `[keys.graph] merge = "M"`): graph as normal,
      bottom row `config: 1 problem · ? shows them`. **Colour unverified** (rule 5). ≤ 200 KB.
- [ ] `crates/graft/MAP.md`: `Keys::remaps`, `keymap_from`, `context_of`, `dispatch_key`, the edge, `config_problems`.

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
