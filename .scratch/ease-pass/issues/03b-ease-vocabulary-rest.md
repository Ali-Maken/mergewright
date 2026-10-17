# ease-vocabulary-rest — names and counts in bar words; graph, banner and clash bars; commit options in `?`/`⌃P`

Status: ready
Blocked by: 03-ease-vocabulary (merged)
Build with: Opus (builder-opus) — brownfield edits to registries pinned by sweeps and snapshots; the representation is decided below, so a wrong turn shows up in a failing sweep or snapshot, not later. Fails Fable test 2 (correctness is checkable by tests). Scout: palette questions only (below); the hint facts are in 03's scout and Handoff — the conductor may skip the scout if 03's builder answers them.
Needs: none. Feeds: G (07) `HintContext::GraphRow`; S (08) `RowBar::JustCommitted`, the commit-option listing; P (05) `HintContext::Clash`, the merge banner words; L (11) `ClashBar`; Fr (12) `RowBar::JustMerged`, the `Cow` verb; Mp (10) M1 words.

## Read first
- `.scratch/ease-pass/issues/03-ease-vocabulary.md`: "What the scout changed", Handoff (blockers 2 and 3, **the mock bar words, exact**).
- `docs/design-notes/ease-pass.md`: Mock behaviours G13, M1, M11, M14, C11; Questions 1, 2, 7, 8, 11, 12 (answered a).
- `docs/design/02-keymap.md` §2.6, §2.10 (03's Proposed edits).
- `crates/graft-ui/MAP.md` `graph_screen::hints`, `keymap`. `crates/graft/MAP.md` `update::palette`, `update::commit_editor`.
- `.scratch/ease-pass/scouts/03-ease-vocabulary.md`.
- The merge mock's **graph bar only** (`prototypes/ease-mocks/mergewright-merge-screen.html`, visible text): the words on `integration`'s row before `m`.

## Scout questions (palette only, ≤½ page)
- Does `⌃P` open the palette from the commit editor today, in which focuses? Does `?` open help from a non-text focus (today's option rows)? Where `NEVER_LISTED` (`graft/src/update/palette/mod.rs:301-307`) and `listed` (`listing.rs:194-196`) are read, and why `CommitEditor` is never listed.
- How a chosen palette row dispatches (id → handler) and whether it can return to the screen it opened from.
- How today's editor flips `Field::{Amend,NoVerify,AllowEmpty}` (`graft/src/update/commit_editor.rs`) — the toggle the three ids call.

## Decided here (blocker 3: names in words)
- **`Hint.verb` becomes `Cow<'static, str>`**; `Hint` drops `Copy` (keeps `Clone, Debug, PartialEq, Eq`). Static words stay `Cow::Borrowed` (const-constructible, lists unchanged); a verb with a name or count is `Cow::Owned`, built by the list function. The fitter measures the verb it draws — one path, no second segment. Rejected: an interner (leaks by design, global state); a caller-formatted hint (the bar has no slot); a template + side `BarNames` argument (two measuring paths, `hints_for` signature churn at every caller).
- **Names and counts ride in `HintContext`**, as fields of **new** variants. Existing variants (`Banner`, `Resolver`, `Finish`, `BranchRow`, `PlainRow`) gain **no fields** — that would break ~70 struct literals in `graft`. If `HintContext` derives `Copy` today, drop it; fix sites by borrow or `.clone()`; the count goes in NOTES.
- A name in a bar is the branch's **last path segment** (`sara/fix-toc` → `fix-toc`), sanitized; a count is `u32`, pluralised (`1 commit`, `3 commits`). At the `Short` rung a named verb drops its name (`C switch`), so a 60-cell name never splits or crowds out a hint; if `Short` is derived by a rule, make the rule hold and say how in NOTES.
- **A bar state lands with the handler that makes its words true** (same rule as 03's blocker 1). This ticket adds every state whose keys act today. The merge's all-solved words (`y review the merge`) are P's; the review and look states are Fr's (05, 12 carry them as acceptance lines).

## Scope
In — graft-ui: `graph_screen/hints/hint.rs`, `hints/context.rs`, `hints/lists/rows.rs`, `hints/lists/resolver.rs` (`resolver_list` only), the banner's list arm, `hints/catalogue.rs` (`every_context`, `every_hint`), hint tests, `spell_sweep.rs`, `tests/typed_fields.rs`, every `.verb` reader in graft-ui. graft: `update/palette/{mod.rs,listing.rs}`, `update/commit_editor.rs` (the three ids' dispatch only), `update/tests/{sweep_screens.rs,dead_keys.rs,palette_live.rs}`, every `.verb` reader in graft tests. Disjoint from P's files; build before P.
- **Graph rows** — new `HintContext::GraphRow { row: RowBar, verbs: RowVerbs }`, words exactly 03's Handoff list (bar end `. more` kept, Q11 a):
  - `Own` `⏎ open · P push · c commit` · `Elsewhere { branch }` (teammate's and trunk rows) `⏎ open · C switch to <b> · x compare`, then the merge mock's `m merge <b> into yours` (M1; its order as that mock shows it; if both mocks' words don't fit at 120 cols, stop and report) · `FoldedMerge { inside }` `space show the N commit(s) inside · ⏎ open · x compare` · `Opened` `space hide the commits inside` · `JustCommitted` `u undo the commit, your edits come back · P push · c commit again` (C13) · `JustMerged { inside }` `u undo the merge · P push · space show the N commits inside` (M18).
  - G picks these at the row-context choice site; this ticket does not touch the choice site. `BranchRow`/`PlainRow` stay until G reports them unpicked.
- **Banner** (Q7 a): the live merge's banner (`Banner { merge: true }`, no new field) leads `⏎ back to the clashes`. If that state also covers a case where `⏎` does not reach the resolver, add `HintContext::BannerPaused` instead and say so.
- **Clash screen** — new `HintContext::Clash(ClashBar)`, every operation (Q12 a):
  - `Open` `o take yours (left) · t take theirs (right) · b take both · e edit by hand · u undo pick · esc pause, come back later` · `Picked` `n next open clash · u undo pick` · `FileSolved { next }` `n next file: <name> · u undo pick · esc pause, come back later` · `AllSolved` today's finish words (`c` …) then `u undo pick · esc pause, come back later`.
  - `ClashOp::Other` (rebase, cherry-pick, revert, stash): `FileSolved` and `AllSolved` lead with today's resolver `⏎` word (mark the file done); `AllSolved` keeps today's `c` words. `ClashOp::Merge` `AllSolved` keeps today's `c` words **until P** replaces that one arm.
  - `Resolver { .. }` stays for whatever L does not move to `Clash`; L reports.
  - Transient, named: between this ticket and P, `u undo pick`, `n next open clash` and `esc pause` run ahead of their handlers (each key acts today, so sweeps pass). P's acceptance closes it. NOTES says so; not a KNOWN_GAPS row unless the milestone ends before P.
- **Commit options in `?`/`⌃P`** (blocker 2, Q8 a): opened from the commit editor — `⌃P` in every focus, `?` in every non-text focus (today's option rows; S's file list later; in the subject and body `?` types `?`) — the listing includes the `CommitEditor` rows, keyless ones spelled `—`. Choosing `commit.toggle_{amend,no_verify,allow_empty}` flips today's option (the toggle `Field` uses) and returns to the commit editor, focus kept. From any other screen `CommitEditor` rows stay unlisted. A row lands with its dispatch, so `palette_live` stays green.
Out: the `#` and `y` rows (G 07, P 05); the merge all-solved arm (P); review and look states (Fr); the row-context choice site (G), after-commit/after-merge picking (S, Fr); commit header option words (S); any `handler` but the three toggle dispatches. **16 colours:** `RoleUnder`/`RoleTrunk` both reduce to index 7 at 16 colours (03 Handoff); the shapes `│`/`║` still differ and the mocks are true-colour — no work, no user question; conductor logs it unverified (rule 5).

## Signatures
```rust
// graft-ui graph_screen::hints
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint { /* keys, state, rung unchanged */ pub verb: Cow<'static, str> }

pub enum HintContext {
    /* existing variants unchanged, no new fields */
    GraphRow { row: RowBar, verbs: RowVerbs },
    Clash(ClashBar),
}
pub enum RowBar {
    Own,
    Elsewhere { branch: SanitizedText },
    FoldedMerge { inside: u32 },
    Opened,
    JustCommitted,
    JustMerged { inside: u32 },
}
pub struct ClashBar { pub at: ClashAt, pub op: ClashOp }
pub enum ClashAt { Open, Picked, FileSolved { next: SanitizedText }, AllSolved }
pub enum ClashOp { Merge, Other }
/// The bar's form of a branch name: its last path segment.
pub fn bar_name(full: &SanitizedText) -> &str;
```
graft: no new public items; the palette gains a "listed from" source (scout names it).

## Touches
`hints_for` fitting and rungs; every `.verb` reader (~19 files, 03 Handoff) — list them; `every_context`/`every_hint` sample values (`sara/fix-toc`, `3`); graft sweeps `sweep_screens.rs:47 screen_for`, `dead_keys.rs:191 classified_form` (an arm per new state, each in a graft state where every drawn key acts); `palette_live.rs:222` known table; `?`/`⌃P` listing for every other screen (unchanged — test); the subject's typed `?`; existing hint snapshots in `src/graph_screen/snapshots/`, `tests/snapshots/` (must pass **unedited**: no existing bar changes words); `tests/no_literal_keys.rs`; the commit editor's option rows (S retires them, keeps the dispatch); man page (no id change — regenerate and diff empty).

## Acceptance
- [ ] `Hint.verb` is `Cow<'static, str>`; every existing hint snapshot passes unedited (no `.snap.new`).
- [ ] Each `RowBar` at 120 cols draws 03 Handoff's words exactly, keys spelled from the keymap; `Elsewhere { branch: "sara/fix-toc" }` reads `C switch to fix-toc` and `m merge fix-toc into yours`; `FoldedMerge { inside: 1 }` reads `1 commit`, `{ inside: 3 }` `3 commits`.
- [ ] At 60 cols a 60-cell branch name: whole hints only, the named verb at `Short` (`C switch`) or dropped, never cut mid-word.
- [ ] The live merge banner leads `⏎ back to the clashes`; non-merge banners unchanged (snapshot unedited).
- [ ] Each `ClashBar` for `Merge` and `Other` draws its words; `esc` spelled `pause, come back later`, `u` `undo pick`, `n` `next open clash` / `next file: <name>`.
- [ ] graft sweeps pass with an arm per new state: `every_reachable_row_acts_or_is_listed_and_follows_its_remap`, `dead_rows_help_lists_elsewhere_match_the_known_table`, the `screen_for`/`classified_form` sweeps.
- [ ] Commit editor: `⌃P` lists `commit.toggle_amend`, `_no_verify`, `_allow_empty` spelled `—` (closes 03's open `?`-listing box); choosing `_no_verify` turns skip-hooks on, back on the editor, focus kept; `?` in the subject types `?`; from the graph the palette lists no `CommitEditor` row.
- [ ] Workspace gates green; NOTES name the variants, the `.verb` files touched, whether `HintContext` lost `Copy`. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~60k). Split point: the palette listing (blocker 2) as its own ticket; the `Cow` verb and the bar states stay together.
