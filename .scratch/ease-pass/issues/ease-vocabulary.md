# ease-vocabulary — the keys, places and colour roles the three mocks use

Status: draft — **blocked on the user's answers to Q1–Q3, Q6–Q8, Q11** (design note, Questions for the user) and on his approval of the DESIGN edits in the note's "Where I'd differ".
Build with: (blank — brownfield; conductor sets after the scout)
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: ADR-0063 departures check paragraph, Mock behaviours C9–C15, M1–M4, M11–M17, G13–G14, Where I'd differ, Questions.
- `docs/design/02-keymap.md`: §2.3, §2.3b, §2.6, §2.6a, §2.6b, §2.10 (only these).
- `crates/graft-ui/MAP.md`: `keymap` section (`ContextId`, `ActionId`, `HintState`), `theme` section (`Token`, `TOKEN_COUNT`).
- The scout's file for this group.

## Scout questions (≤1 page)
- Where each bar's words come from (keymap hints vs screen code) for graph, commit editor, preview box, resolver, finish summary.
- Every test pinning `ContextId::ALL` length, `TOKEN_COUNT`, action ids (`tests/keymap_ids.txt`), and hint captures.
- Which screens read `preview.*`, `resolver.next`, `finish.toggle_diff` today.

## Scope
In (graft-ui keymap + theme, `docs/design/02-keymap.md`, `tests/keymap_ids.txt`):
- Contexts: `CommitFiles`, `Incoming`, `MergeLook` (22 → 25), with `scope()` / `borrows()` / `stands_down()` set as siblings of `CommitEditor`, `Preview`, `FinishSummary`.
- Actions (default key): `graph.colours_off` `#` (Q3) · `commit_files.toggle` `space` · `commit_files.commit` `⏎` · `commit_files.back` `esc` · `preview.show_incoming` `⏎` (Q6) · `incoming.back` `esc` · `incoming.merge` `y` · `resolver.review` `y` (`c` alias, not on the bar) · `resolver.undo_pick` `u` · `resolver.next` relabelled "next open clash" · `finish.look` `⏎` (replaces `finish.toggle_diff`) · `finish.undo_pick` `u` · `look.back` `esc` · `look.commit` `y`. Bar words: the mocks' (note C12, M2, M11, M15).
- Tokens `RoleThis`, `RoleUnder`, `RoleTrunk`, `RoleOther` (18 → 22). Dark defaults from the mock: `#7ed49b`, `#e9b65c`, `#9fb7ff`, `#4d5654`. Light theme: builder picks; this/under/trunk ≥ 3.0:1 against the background (computed test). Colour itself ships **unverified** (rule 5).
- `02-keymap.md` edits for exactly the above, marked with the ADR-0063 reference.
Out: handlers (S, Mp, P, L, Fr, G), drawing.
If `finish.toggle_diff` is a released config name (`ActionId` "never renamed after release"): keep it as a deprecated alias that maps to `finish.look`, and say so.

## Signatures
```rust
pub enum ContextId { /* existing 22 */, CommitFiles, Incoming, MergeLook }
pub enum Token { /* existing 18 */, RoleThis, RoleUnder, RoleTrunk, RoleOther }
pub const TOKEN_COUNT: usize = 22;
```
Action ids as strings above, in `tests/keymap_ids.txt`.

## Touches
Keymap help screen (`?`) and `⌃P` palette listings; config loading (`config_name()`s); every capture showing a bar (list them; recaptured by their owning groups, not here); `tests/keymap_ids.txt`; theme files / light-theme table; `02-keymap.md`.

## Acceptance
- [ ] No key bound twice in one scope (existing conflict test passes with the new contexts).
- [ ] `#` resolves to `graph.colours_off` in `Graph`; `y` and `u` resolve in `Resolver`; `⏎` resolves to `preview.show_incoming` in `Preview` (if Q6 a).
- [ ] Contrast test for the three role tokens, both themes.
- [ ] `tests/keymap_ids.txt` lists every new id; old ids unchanged (or aliased, stated).
- [ ] `02-keymap.md` diff limited to the listed sections. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Small–medium (~35k).
