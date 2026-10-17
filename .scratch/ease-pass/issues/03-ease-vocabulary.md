# ease-vocabulary — the keys, bar words and colour roles the three mocks use

Status: ready
Blocked by: none
Build with: (blank — brownfield; conductor sets after the scout)
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: ADR-0063 departures paragraph, Mock behaviours G13–G14, C9–C15, M1–M4, M11–M18, Where I'd differ, Questions (answered).
- `docs/design/02-keymap.md`: §2.3, §2.3b, §2.6, §2.6a, §2.6b, §2.10 (only these).
- `crates/graft-ui/MAP.md`: `keymap`, `graph_screen::hints`, `theme` sections.
- The mocks' visible text and key handlers only: `prototypes/ease-mocks/mergewright-{main,commit,merge}-screen.html` (for every bar's exact words).
- The scout's file for this group.

## Scout questions (≤1 page)
- Where each bar's words come from today (keymap hints vs screen code) for graph, commit editor, preview box, resolver, finish summary.
- Every test pinning `ContextId::ALL` length, `TOKEN_COUNT`, action ids (`tests/keymap_ids.txt`), `HintContext` sweeps (`every_context`), and hint captures.
- Which screens read `preview.*`, `resolver.next`, `finish.toggle_diff`, the commit editor's option toggles today.
- Whether `finish.toggle_diff` and the commit option ids are released config names.

## Scope
In (graft-ui `keymap`, `graph_screen::hints`, `theme`; `docs/design/02-keymap.md`; `tests/keymap_ids.txt`). **This group owns `graph_screen/hints.rs` for the whole pass**: every later group picks a `HintContext`, none edits the hint lists.
- Contexts: `CommitFiles`, `Incoming`, `MergeLook` (22 → 25), `scope()` / `borrows()` / `stands_down()` as siblings of `CommitEditor`, `Preview`, `FinishSummary`.
- Actions (default key): `graph.colours_off` `#` · `commit_files.toggle` `space` · `commit_files.commit` `⏎` · `commit_files.back` `esc` · `commit.toggle_amend`, `commit.toggle_no_verify`, `commit.toggle_allow_empty` (no default key; listed in `?` and `⌃P` in the commit screen) · `commit.sign_off` stays `⌃S` · `preview.show_incoming` `⏎` · `incoming.back` `esc` · `incoming.merge` `y` · `resolver.review` `y` (`c` kept as an alias, not on the bar) · `resolver.undo_pick` `u` · `resolver.pause` `esc` (relabel of today's close) · `resolver.next` relabelled "next open clash" · `finish.look` `⏎` (replaces `finish.toggle_diff`; alias kept if released) · `finish.undo_pick` `u` · `look.back` `esc` · `look.commit` `y`.
- `HintContext` variants and their words, exactly the mocks' (note C12, C13, M1, M2, M11, M15, M18, G13): graph row kinds — own branch row, teammate's row, folded merge row, opened row, trunk row; commit screen message / body / files; after-commit landing; merge preview; incoming list; clash screen open clash / solved file / all solved; review; look; after-merge landing.
- Tokens `RoleThis`, `RoleUnder`, `RoleTrunk`, `RoleOther` (18 → 22). Dark defaults from the mock: `#7ed49b`, `#e9b65c`, `#9fb7ff`, `#4d5654`. Light theme: builder picks; this/under/trunk ≥ 3.0:1 on the background (computed test). Colour ships **unverified** (rule 5).
- `02-keymap.md` edits for exactly the above, each marked `Proposed (ADR-0063)`.
Out: handlers (S, Mp, P, L, Fr, G, B), drawing, the picker's open-on-branch (B).

## Answered questions → behaviour this group fixes in the table
- **Q1 a:** on a teammate's row the bar says `C switch to <branch>`; `C` stays `graph.switch` (opens the picker; B makes it open on that branch). No new key.
- **Q2 a:** `x` keeps today's mark-then-compare; on a teammate's row the bar says `x compare`.
- **Q3 a:** `#` = `graph.colours_off`, in `Graph`; toggles the whole screen's colour until quit (G builds it).
- **Q6 a:** the merge preview has no buttons; its bar is `y merge · esc cancel · ⏎ see the N commits`; `⏎` = `preview.show_incoming` in a preview opened by `m`. Other previews keep today's `⏎` (choice under cursor) — scoped by a `HintContext`/context check, stated in the keymap doc.
- **Q7 a:** resolver `esc` = pause: the bar word is the mock's; the action lands on the graph with the operation banner focused (P builds it).
- **Q8 a:** commit options off the screen: `⌃S` signs off; amend, skip hooks, allow empty are rows in `?`/`⌃P` while the commit screen is open.
- **Q11 a:** today's bar end is kept (`… . more`), not the mock's `? more`.

## Signatures
```rust
pub enum ContextId { /* existing 22 */, CommitFiles, Incoming, MergeLook }
pub enum Token { /* existing 18 */, RoleThis, RoleUnder, RoleTrunk, RoleOther }
pub const TOKEN_COUNT: usize = 22;
pub enum HintContext { /* existing */,
    GraphOwnRow, GraphTeammateRow, GraphFoldedMerge, GraphOpenedRow, GraphTrunkRow,
    CommitMessage, CommitBody, CommitFiles, AfterCommit,
    MergePreview, Incoming, ClashOpen, ClashFileSolved, ClashAllSolved, Review, Look, AfterMerge }
```
Variant names may follow the scout's existing naming; the set is fixed. Action ids as strings above, in `tests/keymap_ids.txt`. Words that carry a name (`<branch>`, `N`) are passed in by the caller as today's hints do (scout says how).

## Touches
Keymap help (`?`) and `⌃P` listings; config loading (`config_name()`s, deprecated aliases); every capture showing a bar (listed in NOTES; recaptured by their owning groups, not here); `tests/keymap_ids.txt`; `every_context`/`every_hint` sweeps; theme files / light-theme table and `contrast_misses`; `02-keymap.md`; man page key table (`cli::man`, generated from the key table — regenerate and diff).

## Acceptance
- [ ] No key bound twice in one scope (existing conflict test passes with the new contexts).
- [ ] `#` resolves to `graph.colours_off` in `Graph` (Q3); `y`, `u`, `esc` resolve to `resolver.review`, `resolver.undo_pick`, `resolver.pause` in `Resolver`; `c` still resolves to `resolver.review`.
- [ ] `⏎` resolves to `preview.show_incoming` in a merge preview and to today's choice-under-cursor in a rebase preview (Q6).
- [ ] `C` in `Graph` is still `graph.switch`; `x` still `graph.compare` (Q1, Q2).
- [ ] `commit.toggle_amend`, `commit.toggle_no_verify`, `commit.toggle_allow_empty` appear in the `?` listing for the commit screen with no key; `⌃S` resolves to `commit.sign_off` (Q8).
- [ ] Each new `HintContext` renders the mock's words at 120 cols (one snapshot test per context, words compared against the mock text listed in NOTES); bar end unchanged (Q11).
- [ ] Contrast test for the three role tokens, both themes.
- [ ] `tests/keymap_ids.txt` lists every new id; old ids unchanged or aliased (stated).
- [ ] `02-keymap.md` diff limited to the listed sections, each edit marked `Proposed (ADR-0063)`. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~55k). Split point if it overruns: keymap + theme here; `HintContext` words as a follow-on in the same files.
