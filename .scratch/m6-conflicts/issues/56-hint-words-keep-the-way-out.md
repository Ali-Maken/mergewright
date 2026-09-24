# hint-words-keep-the-way-out: at 120 columns the resolver's bar keeps `d start over` and `esc close` whole

Status: ready
From: user picks 2026-09-24 item 5 (batch 9, option A); `.scratch/m6-conflicts/user-facts-m632.md`; ADR-0044 §4
(amends ADR-0039's third amendment).
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: in `graft-ui::graph_screen::hints`, the ladder's fixed step sequence in `hints/fitting.rs` (`ladder`,
`fit_list`), how `Hint` is built in `hints/lists.rs` for `Resolver { .. }`, and the monotonicity test
(`no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`). Which captures show the resolver's bar at 100-160
columns. Whether any `graft` source test asserts the resolver bar's text at 120 (not a capture).
Needs: none. (Independent of the two colour specs; same crate, so same lane.)
Crates: graft-ui only (plus `docs/shots/` regenerated). A `graft` source test pinning the bar → report
`blocked: needs graft`.
Lane: wave 1, lane U, third.

## Read first
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §4.
- `docs/adr/0039-*.md`, the third amendment (the ladder).
- `crates/graft-ui/MAP.md` line 22 (the ladder, `. more`, `Resolver` table) and line 16 (hints files).
- `.scratch/m6-conflicts/user-facts-m632.md` (the measured widths: 135 wanted, 137 for a rebase).

## Scope
**In:**
- `Hint` gains a flag, **keeps its words**. The ladder's step sequence becomes: the three most relevant at `Full`; then
  every hint that keeps its words at `Full`, in list order; then every remaining hint at its bare `Key`; then those
  raised one at a time from the left, short then full. Still one fixed sequence, longest prefix that fits.
- The resolver's tables (full, card, whole-file, done) mark `d start over` and `esc close`. No other context marks any
  hint.
- At 120 columns, the merge resolver with `B base` and `d start over` offered draws `d start over` and `esc close`
  whole; `j/k` and `M` go bare first.

**Out:**
- Short forms (`short-words-from-the-draft`, blocked).
- Hint order (`c finish` first is `last-file-opens-the-summary`'s).
- Any context other than the resolver.

## The failing test first
`graft-ui/src/graph_screen/hints` tests: `resolver_bar_keeps_the_way_out_at_120`.
- `hints_for(Resolver { base: true, restart: true, whole_file: false, roles: Merge, done: false, finishable: false },
  Divider::Hidden, 120)` → `hints_text` contains `d start over` and `esc close`; `M` and `j/k` appear bare.
- Red today: `fix-stop-lands-in-resolver-120x40` shows `M mergetool  d  esc` (user-facts-m632.md).

## Signatures
```rust
// graft-ui/src/graph_screen/hints.rs
pub struct Hint { pub key: &'static str, pub verb: &'static str, pub state: HintState, pub rung: Rung,
                  pub keeps_words: bool }            // new field; `false` everywhere but the resolver's two
impl Hint { #[must_use] pub fn keeps_words(self) -> Self; }   // builder used in lists.rs
```
If `Hint` is built by struct literal outside `graft-ui` (scout checks `grep -rn "Hint {" crates`), a new field breaks
`graft`: then add the flag as a private side table keyed by (context, key) instead, and say which in NOTES.

## Touches (each entry, and how the builder checks it)
- **Every other context's bar is unchanged** at every width.
  - Check: `only_the_resolver_marks_words` (iterate `every_context()`; any marked hint outside `Resolver` fails) and
    the existing snapshot/width sweeps green without edits.
- **Monotonicity in width.** Check: `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows`, unchanged, green.
- **The floor and `. more`.** The resolver is modal (no `. more`). Check: the floor tests green.
- **`graft`'s `every_key_the_bar_draws_is_answered`** — verbs unchanged, keys unchanged. Check: it runs green.
- **Captures of the resolver's bar at 100-160:** `m6-10`…`m6-14` at 120x40 and 160x48, `fix-stop-lands-in-resolver-120x40`,
  `fix-file-keys-*`, `fix-done-*`.
  - Check: `grep -rln "esc$\|  d  esc" docs/shots/m6`; regenerate (`GRAFT_UPDATE_SHOTS=1`); NOTES lists each file and
    its new bottom row.
- **ADR-0039:** the amendment is recorded by ADR-0044 §4; no edit to 0039.
- **Docs:** `crates/graft-ui/MAP.md` line 22 (the step sequence, the flag).

## Acceptance (name the verifying test at tick time)
- [ ] `resolver_bar_keeps_the_way_out_at_120`: red first, then green.
- [ ] `resolver_bar_keeps_the_way_out_at_120_in_a_rebase` (rebase roles, 120 columns).
- [ ] `only_the_resolver_marks_words`
- [ ] `no_hint_bar_context_drops_a_hint_as_the_room_for_it_grows` (existing, green)
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-bar-keeps-the-way-out-120x40.txt`.
  - Scene: the merge resolver on `notes.txt`, `B base` and `d start over` offered, 120 columns.
  - **What right looks like:** the bottom row contains `d start over` and ends `esc close`; `M` and `j/k` are bare
    keys; no hint is cut mid-word.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Small. One context.

## NOTES
