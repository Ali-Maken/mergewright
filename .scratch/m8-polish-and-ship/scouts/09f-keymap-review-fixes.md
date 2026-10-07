# Scout 09f — keymap review fixes (at 469c6c70)

Paths: graft-ui = `crates/graft-ui/src/`, graft = `crates/graft/src/`.

## Part A — file splits
- `keymap/resolve.rs` 514 lines. Fns: resolution — `default` :40, `with_overrides` :74, `base_map` :106, `follow` :127, `translate` :184, `shipped` :217, `chords` :325, `reachable` :333, `in_context` :343, `acting` :357, `resolve` :366, `holder` :378, `best` :383, `has` :394, `rank` :403, `index_of` :451, `rows_of` :464, `borrowed_rows` :473, `reachable_rows` :478. Spell — `spell` :226, `spell_keys` :238, `spell_every` :249, `key_verb` :258, `key_verbs` :267, `after` :278, `live_labels` :290, `relabel` :297, `live_chords` :312, `join` (pub(crate)) :420, `word_prefix` :440. Apply — `apply` :137 (method), `parse_list` :492.
- Contradiction: ticket lists `relabel` in spell.rs, but it is a `&mut self` step of construction (called from build path); `apply` is an `impl Keymap` method at :137 — moving it needs a second `impl` block in apply.rs. Not a blocker; flag only.
- `preview_box.rs` 445 lines; `#[cfg(test)]` at :444 — the tests are a 1–2 line `mod` stub, not inline. Moving them saves ~2 lines → still ~443; the `Again` split is therefore required, not conditional. Could not confirm where `Again` lives (no `Again` fn in the grep; types between :120–:265).

## Part B — scope/borrows
- `keymap.rs:146–160` `scope()` → `&'static [ContextId]`; Map/Journal/Stash = `[self, Graph, Global]` (:151–153).
- `keymap.rs:179–193` `borrows()` → `&'static [ActionId]`; Map|Stash → `LIST_BORROWS`, Journal → `JOURNAL_BORROWS`. `stands_down()` should mirror `&'static [ActionId]`.
- `reachable` resolve.rs:333 via `reachable_rows` :478 / `borrowed_rows` :473 — the filter point.
- Not checked: sweep acting rows per overlay, `SWALLOWED_BY_DESIGN` shape. Builder must measure.

## Part C — one-chord handlers (all match **raw `key.code`**)
- Blame `update/place/keys.rs:57–58`: raw `Char('g')`/`Char('G')`; no `Home`/`End` arm, falls to `_ => {}` :78.
- Changes `update/detail/keys.rs:307`: raw `Char('g') if plain`; PageUp/Down raw arms above.
- Resolver `update/resolver.rs:379`: `Char('u'|'U') if plain`; :383 is the `q` arm area, not ⌃Z. `banner/focus_keys.rs:147` `u|U` arm, :173 `_ => {}` silent arm.
- Why `Home`/⌃Z miss: could not confirm whether `edge` translate fails to map them at these contexts or translates but the handler never sees it. Read `update/edge.rs` before choosing the fix (ticket's stop condition).
- `rewrite_editor/right.rs:143–144`: `"AFTER y · checking…"` / `"AFTER y · not written yet"` — literal `y` in a header string, `format`-free.

## Overlaps
- **10f**: `update/banner/focus_keys.rs` (10f spells `c continue  a abort` there; 09f edits :147/173). Both edit `crates/graft/MAP.md` and graft-ui `tests/no_literal_keys*` (10f: `known_stops.rs`; 09f: the pattern in `no_literal_keys.rs`) — different files, same test binary.
- **17**: no file in 09f's list named in 17's ticket (17 names `rows.rs`, `detail_pane/files.rs`, theme, graph_column). Both edit graft-ui and graft `MAP.md`, and both re-shoot captures.
