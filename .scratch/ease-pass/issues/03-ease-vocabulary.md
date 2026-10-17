# ease-vocabulary — the keys, bar words and colour roles the three mocks use

Status: ready (revised 2026-10-17, fitted to `.scratch/ease-pass/scouts/03-ease-vocabulary.md`; its file:line facts stand)
Blocked by: none
Build with: Opus — scout recommends builder-opus: brownfield edits to shared registries (key table, hint lists, theme) pinned by golden and sweep tests; mechanical once decided, no structural risk.
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: ADR-0063 departures paragraph, Mock behaviours G13–G14, C9–C15, M1–M4, M11–M18, Where I'd differ, Questions (answered).
- `docs/design/02-keymap.md`: §2.3, §2.3b, §2.6, §2.6a, §2.6b, §2.10 (only these).
- `crates/graft-ui/MAP.md`: `keymap`, `graph_screen::hints`, `theme` sections.
- The mocks' visible text and key handlers only: `prototypes/ease-mocks/mergewright-{main,merge}-screen.html` (graph, clash screen, review, look bars). The commit mock's bars are S's; the merge preview's and incoming list's are Mp's.
- The scout's file (above).

## What the scout changed (decided here)
- **No new `ContextId`** (stays 22; `ContextId::ALL` pins untouched). The incoming list is a state of `Preview`; the read-only look is a state of `Finish`; the commit screen's file list is a focus of `CommitEditor`, keyed by focus in `update::commit_editor` like today's option rows (`graft-ui/src/commit_editor.rs:41-53`). Their keys are existing ids: `y` = `confirm.yes` borrowed by `Preview`/`Finish` (`keymap.rs:204`), `esc` = `app.back`, `j/k` = the Global move rows.
- **Existing ids keep their names and keys**; only bar words change: `resolver.next_block` `n` (`table.rs:323`) → "next open clash"; `app.back` `esc` in the resolver (`table.rs:77`, handled `graft/src/update/resolver.rs:363`) → "pause"; `app.undo` `u` in the resolver (`table.rs:78`, answered `resolver.rs:379`) and in the review → the mock's pick-undo words; `resolver.finish` `c` (`table.rs:325`) — in a merge P points it at the review (that is the `c` alias), elsewhere it goes on as today; `finish.toggle_diff` `⏎`/`space` (`table.rs:362-364`) — in the merge review Fr points it at the look; **not renamed** (a rename needs an alias here and edits at the graft app use sites `finish/keys.rs:100-101`, `finish/mouse.rs:74`, outside this crate set; the stale name goes in NOTES as a readability item). This group changes no handler.
- **`⏎` in the merge preview gets no id.** It is hard-matched (`graft/src/update/preview/keys.rs:67`); Mp branches there by preview kind. One `Preview` context.
- **Bars drawn by screen code stay screen code, owned by their screen's group:** commit screen (`commit_editor.rs:178,192,212`) → S; preview box footer (`preview_box/footer.rs`) and incoming list → Mp. This group owns the hint lists only.

## Scope
In (graft-ui `keymap`, `graph_screen::hints`, `theme`; `docs/design/02-keymap.md`; `tests/keymap_ids.txt`). **This group owns `graph_screen/hints.rs` and `hints/lists/*` for the whole pass**: later groups pick a `HintContext`, none edits the hint lists.
- **New action ids** (5), in `tests/keymap_ids.txt`:
  - `graph.colours_off` — `#`, context `Graph` (Q3 a).
  - `resolver.review` — `y`, context `Resolver`. Meaning (P builds it): in a merge, the review; in a rebase/cherry-pick/revert/stash clash, no meaning — not drawn, answered as today's modal resolver answers an unbound key (Q12 a keeps `c` there). `y` is unbound in `Resolver`'s scope today (§2.6 table) — the conflict test proves it.
  - `commit.toggle_amend`, `commit.toggle_no_verify`, `commit.toggle_allow_empty` — context `CommitEditor`, **no default key** (`Binding.default` empty; `spell` gives `—`); listed in `?` and `⌃P` while the commit screen is open (Q8 a). If the table or palette cannot hold a keyless row, stop and report — do not invent a key.
- **Unchanged:** `⌃S` keeps its existing row in `CommitEditor` (whatever id `table.rs` gives it); `C` = `graph.switch` (Q1 a); `x` = `graph.compare` (Q2 a); `finish.toggle_diff` keys. Read the merge mock's review key handler: if it gives `space` a meaning other than showing the file, stop and report it as a question.
- **`HintContext`** — the bars below, words exactly the mocks' (note G13, M11, M15, M17, M18, C13). Extend existing variants with fields where one fits (`BranchRow { current, verbs }`, `PlainRow { verbs }`, `Banner { merge }`, `Resolver { .. }`, `Finish { .. }`, `hints/context.rs:47+`) or add variants; the set of bars is fixed:
  - Graph (`hints/lists/rows.rs`): own branch's row · teammate's row (`C switch to <branch>`, `x compare`) · folded merge row (`space show the N commits`) · opened row · trunk row · after-commit landing (`u undo the commit, your edits come back · P push · c commit again`) · after-merge landing (`u undo the merge · P push · space show the N commits inside`).
  - Banner: a merge paused from the clash screen leads `⏎ back to the clashes` (Q7 a).
  - Clash screen (`hints/lists/resolver.rs` `resolver_list`): open clash · solved file · all solved — merge tail `y` review; non-merge tail today's `c` words (Q12 a); `esc` spelled "pause"; `u` the mock's pick-undo word.
  - Finish (`finish_list`): the merge review (`⏎ look`, `u`, `y`) · the look (`esc` back, `y` commit).
  - **Names in words** (`<branch>`, `N`): no `HintContext` carries a name today (scout). Carry it as a variant field (`SanitizedText` / count) — the first such field; if the enum's derives (e.g. `Copy`) forbid an owned name, the caller formats the verb with `Keymap::key_verb` as `graft/src/update/resolver/resolved.rs:44` does. Say which in NOTES.
- **Tokens** `RoleThis`, `RoleUnder`, `RoleTrunk`, `RoleOther` (18 → 22; arrays sized by `TOKEN_COUNT`, `theme/mod.rs:62`). Dark defaults from the mock: `#7ed49b`, `#e9b65c`, `#9fb7ff`, `#4d5654`. Light theme: builder picks; this/under/trunk ≥ 3.0:1 on the background (computed test beside `theme/tests.rs:368`). Colour ships **unverified** (rule 5).
- `02-keymap.md` edits, each marked `Proposed (ADR-0063)`: §2.3 `#`, fold default; §2.3b merge preview `⏎` lists the incoming commits (by preview kind; other previews keep the choice under the cursor), no buttons; §2.6 `y` review (merge), `n` next open clash, `u` undoes the last pick, `esc` pause, `⏎` in a merge; §2.6a `u` inside the resolver; §2.6b review: `⏎`/`space` look, `u`; §2.10 `⇥` subject → body → files, options from `?`/`⌃P`.
Out: handlers (P, S, Mp, Fr, G, B), drawing, the commit screen's and preview's bar words (S, Mp), the picker's open-on-branch (B).

## Answered questions → behaviour this group fixes in the table
- **Q1 a:** teammate's row bar `C switch to <branch>`; `C` stays `graph.switch`. **Q2 a:** `x` stays `graph.compare`; bar `x compare`. **Q3 a:** `#` = `graph.colours_off`, `Graph` (G builds it).
- **Q6 a:** no table row; `⏎` stays hard-matched; the §2.3b edit says it is scoped by preview kind.
- **Q7 a:** `esc` in the resolver stays `app.back`; the bar word is the mock's "pause" (P's handler).
- **Q8 a:** `⌃S` unchanged; the three `commit.toggle_*` rows. **Q11 a:** bar end `… . more` unchanged.

## Signatures
```rust
// graft-ui keymap — ContextId unchanged (22). New rows in `defaults()` (id, context, default chords):
//   graph.colours_off          Graph         [KeyChord::key('#')]
//   resolver.review            Resolver      [KeyChord::key('y')]
//   commit.toggle_amend        CommitEditor  []
//   commit.toggle_no_verify    CommitEditor  []
//   commit.toggle_allow_empty  CommitEditor  []
pub enum Token { /* existing 18 */, RoleThis, RoleUnder, RoleTrunk, RoleOther }
pub const TOKEN_COUNT: usize = 22;
pub enum HintContext { /* existing; graph row, landing, banner, resolver and finish states added or extended as above */ }
```
Name the new/extended variants and fields in NOTES; G, L, Fr, S (after-commit), P (paused banner) pick them by those names.

## Touches
Key table and `tests/keymap_ids.txt` (golden, `keymap/tests/table.rs:7-21`); clash check (`keymap/clash.rs:21`); `?` and `⌃P` listings; hint sweeps `hints/catalogue.rs:42,365`, `hints/tests.rs:1360`, `graph_screen/tests.rs:1446,2651`, `hints/spell_sweep.rs`, `hints/tests/typed_fields.rs`; literal-key guard `tests/no_literal_keys.rs`; hint snapshots in `src/graph_screen/snapshots/`, `tests/snapshots/` (recaptured by their owning groups, listed in NOTES); theme arrays (`theme/mod.rs:260,266`, `theme/chrome.rs:30,137`, `theme/overrides.rs:38`), `contrast_misses` (`overrides.rs:89`), light-theme table; `02-keymap.md`; man page key table (`cli::man`, generated — regenerate and diff).

## Acceptance
- [ ] No key bound twice in one scope (`y` in `Resolver` included) — **open**: `keymap::tests::table::every_action_has_a_default_and_no_context_binds_one_chord_twice` passes (keyless rows allowed by its `KEYLESS` list), but the `y` row is held back (Handoff, blocker 1). `keymap::tests::ease::the_clash_screen_keys_resolve_to_their_ids` shows `y` free in `Resolver` today.
- [ ] `#` → `graph.colours_off`, `y` → `resolver.review` — **open** (held back, blocker 1). The rest holds: `c` → `resolver.finish`, `n` → `resolver.next_block`, `esc` → `app.back`, `u` → `app.undo` in `Resolver` — `keymap::tests::ease::the_clash_screen_keys_resolve_to_their_ids`.
- [x] `⏎` and `space` still resolve to `finish.toggle_diff` in `Finish` — `keymap::tests::ease::the_review_look_keys_are_still_toggle_diff`.
- [x] `C` in `Graph` is still `graph.switch`; `x` still `graph.compare` — `keymap::tests::ease::switch_and_compare_keep_their_keys`.
- [ ] `commit.toggle_*` in the `?` listing for `CommitEditor` spelled `—` — **open, palette side** (blocker 2). Table side holds: three keyless `CommitEditor` rows, `spell` = `—`, `⌃S` still `commit.sign_off` — `keymap::tests::ease::commit_options_are_keyless_rows_of_the_commit_screen`; man page lists them `(none)` (`cli::docs_tests::committed_man_page_is_the_generated_one`).
- [x] `ContextId::ALL.len()` still 22 — pin at `keymap/tests/rules.rs:383` untouched and passing; `keymap::tests::ease::no_new_context`.
- [ ] Each new or extended `HintContext` state renders the mock's words — **not started** (split point, blocker 3).
- [x] Contrast test for the three role tokens, every shipped theme — `theme::tests::role_tokens_clear_the_non_text_floor_on_bg`; dark values pinned to the mock by `theme::tests::dark_role_tokens_are_the_mocks`. Colour **unverified** on screen (rule 5).
- [ ] `tests/keymap_ids.txt`: exactly 5 new ids — **open**: 3 of 5 (`commit.toggle_*`), `keymap::tests::table::action_ids_match_the_golden_list`; none renamed or removed.
- [x] `02-keymap.md` diff limited to the listed sections, each marked `Proposed (ADR-0063)` — no test (a doc): checked by reading the `-U0` diff: 6 hunks, one each in §2.3, §2.3b, §2.6, §2.6a, §2.6b, §2.10; 7 marks (§2.3 has a row and a paragraph).

## Handoff (builder, 2026-10-17)
Done: 3 keyless `commit.toggle_*` rows; tokens `RoleThis/Under/Trunk/Other` (`TOKEN_COUNT` 22; dark = mock; light `#1E7F45 #9A6200 #3550B8 #B4BAC2`, high contrast `#00E676 #FFB300 #82A0FF #6E6E6E`, builder's picks); `02-keymap.md` Proposed edits; man page regenerated; file-size ratchet rows raised (`crates/graft/tests/it/over_400.txt`: `keymap/table.rs` 533, `theme/mod.rs` 431, `theme/tests.rs` 869).

**Blocker 1 — the `#` and `y` rows cannot land before their handlers.** Added, they failed `graft`'s dead-key sweeps: `update::tests::keymap_edge::sweep::every_reachable_row_acts_or_is_listed_and_follows_its_remap` (`sweep.rs:185`: "Graph: graph.colours_off (#) does nothing", "Resolver: resolver.review (y) does nothing") and `update::tests::palette_live::dead_rows_help_lists_elsewhere_match_the_known_table` (`palette_live.rs:222`): `?` would list a key that does nothing. So each row lands **in the same commit as its handler**. G: `b("graph.colours_off", Graph, &[k('#')], "Turn colours off / on (lines keep their shapes)")` after `graph.journal`. P: `b("resolver.review", Resolver, &[k('y')], "Review the merge (a merge only)")` after `resolver.finish`. Each adds its id to `tests/keymap_ids.txt` and flips the `None` asserts in `keymap::tests::ease::{hash_is_free_on_the_graph, the_clash_screen_keys_resolve_to_their_ids}`.

**Blocker 2 — the palette never lists `CommitEditor` rows nor keyless rows** (`graft/src/update/palette/mod.rs:301-307` `NEVER_LISTED` holds `CommitEditor`; `listing.rs:194-196` `listed` drops an empty default), and `?` in the subject is typed text. Q8 a needs a `graft` change (S, or its own ticket): how `?`/`⌃P` open from the commit screen, and listing keyless rows there.

**Blocker 3 — `HintContext` words (the split point).** (a) New variants break `graft`'s compile: its sweeps match `HintContext` exhaustively on purpose (`graft/src/update/tests/sweep_screens.rs:47 screen_for`, `dead_keys.rs:191 classified_form`), and each new state needs a `graft` state in which every key it draws is answered — G/P/Fr's handlers. Extending `Banner`/`Resolver`/`Finish` with a field breaks ~70 struct literals in `graft`. (b) Names in words: `Hint` is `Copy` with `verb: &'static str` (`hints/hint.rs:68-72`), and `hints_for`'s fitting measures the verb, so `C switch to <branch>` and `space show the N commits inside` cannot ride a `HintContext` field through today's `Hint`. Options: (i) `Cow<'static, str>` verb — drops `Copy`; `.verb` is read in ~19 files, most `graft` tests; (ii) an interner handing out `&'static str` per distinct name (bounded by branch count; leaks by design); (iii) the caller formats with `Keymap::key_verb` (`resolved.rs:44`) — but the bar has no slot for a caller's hint. **Question for conductor/planner: (i), (ii) or another.** Next step: one builder holding `graft-ui` hints and `graft`'s sweep arms, after that choice, sequenced with or after G/P/Fr.

Mock bar words, exact, for that group: own row `⏎ open · P push · c commit`; teammate/trunk row `⏎ open · C switch to <last path segment> · x compare` (Q2 a); folded merge `space show the N commit(s) inside · ⏎ open · x compare`, opened `space hide the commits inside`; bar end `u undo · ? more` in the mock, today's `. more` kept (Q11); after merge `u undo the merge · P push · space show the N commits inside`; after commit (C13) `u undo the commit, your edits come back · P push · c commit again`; clash open `o take yours (left) · t take theirs (right) · b take both · e edit by hand · u undo pick · esc pause, come back later`; clash picked, file still open `n next open clash · u undo pick`; file solved, next file `n next file: <name> · u undo pick · esc pause, come back later`; all solved `y review the merge · u undo pick · esc pause, come back later`; review `y commit the merge · j k move · ⏎ look at it · u undo last pick · esc back to the clashes (y brings you here again)`; look `esc back to the review · y commit the merge`. The merge mock's review gives `space` no meaning (its key handler never reads it) — no question raised.

Readability item: `finish.toggle_diff` opens the look in a merge review; the name goes stale (rename needs an alias and `graft` edits at `finish/keys.rs:100-101`, `finish/mouse.rs:74`).

Colour (unverified, rule 5): at 16 colours dark `role-under` and `role-trunk` both reduce to palette index 7 (`theme/snapshots/graft_ui__theme__tests__resolved_values.snap`) — told apart by shape only there.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~45k). Split point if it overruns: key table + theme here; `HintContext` words as a follow-on in the same files.
