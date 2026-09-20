# hint-bar-redesign: three most relevant keys per context at 80 and 120, never all hidden; failure lines keep their keys

Status: done
Blocked by: none
Build with: (blank — brownfield in `graph_screen`'s footer and `view::rows::graph_view`; recommendation: **Sonnet** — a pure table + width rule, fully unit-testable per width; the mockups in the note are the spec)
**Conductor set after the scouts: Sonnet.**
Crates: graft-ui, graft (one seam: `GraphView.hint_context` filled by `view/rows.rs`)
Needs: none (`HintContext::Banner`/`Preview` variants exist from day one; pass-2 groups only start producing them)

## Scout first (Haiku `Explore`, ≤ 1 page)

- `crates/graft-ui/src/graph_screen.rs` (+ its footer file if split): how hints are listed today, the drop-from-the-right rule, the `changes_pane`/`detail_shown`/`reveal`/`problem`/`notice`/`search`/`compare_footer`/`commit_graph` footer slots and their precedence; the `. explain` rule.
- `crates/graft/src/view/rows.rs` (`graph_view`): which `GraphView` fields it sets from `AppState` (`changes_pane`, marker on the cursor row, `detail_shown`); how the cursor row's badges are known there.
- `crates/graft/src/update/remote.rs` `remote_failed` and the `Sticky` lines (the `reason · keys` shapes to fit); `view/frame.rs` where `Status::Sticky` is drawn into the footer.
- Report: every string the footer can draw today and which slot wins.

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — New UI "Hint bar" (table + mockups: the spec), Interfaces (G6 block, `HintContext`/`hints_for`/`fit_status_line`).
- `docs/adr/0037-m4-checkpoint-decisions.md` §4.
- `docs/design/02-keymap.md` §2.1 (a key not named there is not free), §2.2, §2.3, §2.4; `docs/design/01-view-mockups.md` §1.1–1.3 footers, §1.16.
- `docs/reviews/m4-07-acceptance.md` §M2 (the 80-column truncation) and `KNOWN_GAPS.md` rows 483, 485.
- `crates/graft-ui/MAP.md` §graph_screen (the footer history: 144/155-column thresholds are what this replaces).

## Scope

In, in order:

1. **`graph_screen/hints.rs`** (new): `Hint { key: &'static str, verb: &'static str }`; `HintContext`; `hints_for(context, columns) -> Vec<Hint>`: the note's priority table verbatim, then the width rule — `< 100`: first 3 + `. more`; `100..160`: first 6 + `. more`; `≥ 160`: all + `. more` (still dropping from the right if the row overflows, never below 3 + `. more`). `Marked(inner)`: `. explain` first, then `inner`'s list, and `. more` is omitted (`.` is already shown). `Preview`: an empty main-footer list is **allowed only for this context** (the box carries its own footer) — every other context's list is non-empty by construction (unit test over every variant × widths 40..200).
2. **The footer** draws `hints_for` in place of the old fixed list; the old `changes_pane`/`. explain`/`<> resize` special cases fold into contexts (`ChangesPane` adds `<> resize` after its six when `detail_shown`). Precedence among slots unchanged: `problem` > `search` > `notice` > `commit_graph` > `compare_footer` > `reveal` > hints.
3. **`fit_status_line(reason, keys, columns)`**: a status line is `reason · keys`; when it does not fit, cut the reason to what is left with `…`, keeping ` · keys` whole; when even ` · keys` does not fit, cut keys from the right. Used by `view/frame.rs` for `Status::Sticky`/`Failed` lines that carry keys: `update::remote` splits its lines into `(reason, keys)` — change `remote_failed`'s outputs to carry the two parts (`Status::Sticky { reason, keys }`? — no: keep `Status` as is and have `remote_failed` build via a helper that stores `reason` and `keys` in `RemoteState` for the footer to fit per frame; simplest is a `Status::Sticky` variant carrying `Option<keys>`: `Sticky(message)` → `Sticky { reason: String, keys: Option<String> }`; the scout says which is smaller). Every existing `Sticky` site compiles with `keys: None`.
4. **`view/rows.rs`**: build `hint_context` from `AppState`: `on_working_tree` → `WorkingTreeRow`; cursor row's badges (a local branch → `BranchRow { current: is HEAD's }`; none → `PlainRow`); marker on the cursor row → `Marked(..)`; `on_changes_pane` → `ChangesPane { unmerged: any unmerged entry }`; journal/stash overlays → their variants (the overlays draw their own footer today; the variant exists for the help overlay in M8). `Banner`/`Preview` are set by pass-2 groups; until then never produced.
5. Tests: (a) `hints_for` never empty for every non-`Preview` context at every width 40..=200 — `no_context_hides_every_key`; (b) at 80: exactly `3 + . more` for each row context, matching the note's table order — `eighty_columns_shows_three_and_more`; (c) at 120: six — `hundred_twenty_shows_six`; (d) at 160: the full list — `hundred_sixty_shows_all`; (e) `Marked` leads with `. explain` and omits `. more`; (f) `fit_status_line` keeps ` · p pull · P force` whole at 80 for the rejection line, cuts the reason with `…`; keeps the reason whole when it fits — `status_line_keeps_its_keys`; (g) frame test: the footer at 80×24 for a branch row reads `m merge  r rebase  ⏎ open  . more` — `footer_eighty_branch_row`; (h) the M4 walk's `rejected-sticky-80x24` expectation updated to the fitted line.
6. Every key hinted is in `docs/design/02-keymap.md` for its context and bound in code (`?` and `⌃P` are **not** hinted until M8 binds them) — a test walks the table against a list of bound keys the app exposes (`update::keys::bound_keys() -> &[&str]`, add it).
7. MAP.md (graft-ui graph_screen/hints; graft view::rows, `Status::Sticky` if changed); captures: `docs/shots/m5-hints-{branch,plain,uncommitted,marked}-{80x24,120x40,160x48}.txt` (12 files, footer row trimmed to the point — the whole frame is fine under 8 KB) via the existing pty pipeline on a corpus shape (`Shape` with a branch, a marked merge, a dirty tree).

Out: the help overlay (`?`, M8); remapping; the overlays' own footers (journal, stash, editor, boxes).

## Signatures

```rust
// graft-ui/src/graph_screen/hints.rs
pub struct Hint { pub key: &'static str, pub verb: &'static str }
pub enum HintContext { WorkingTreeRow, BranchRow { current: bool }, PlainRow, Marked(Box<HintContext>),
    Banner { skippable: bool }, Preview, ChangesPane { unmerged: bool, detail_shown: bool }, Journal, Stash }
pub fn hints_for(context: &HintContext, columns: u16) -> Vec<Hint>;
pub fn hints_text(hints: &[Hint]) -> String;   // `m merge  r rebase  ⏎ open  . more`
pub fn fit_status_line(reason: &str, keys: Option<&str>, columns: u16) -> String;
// graft-ui/src/graph_screen.rs
pub struct GraphView { …, pub hint_context: HintContext }   // replaces `changes_pane`
// graft/src/update/mod.rs
pub enum Status { …, Sticky { reason: String, keys: Option<String> } }   // if the scout confirms this is the smaller change
pub fn bound_keys() -> &'static [&'static str];   // update::keys — what the app actually binds on the graph screen
```

## Acceptance

- [ ] Tests (a)–(h) green, named at tick time
- [ ] The 12 captures exist, each footer matches the note's mockup for its width and context; `TESTING.md` entry "Hint bar" lists per capture the exact footer string expected
- [ ] `grep -rn "144\|155\|156" crates/graft-ui/src/graph_screen*` finds no column threshold left — inspection
- [ ] KNOWN_GAPS rows 483 and 485 named closable in NOTES
- [ ] MAP.md updated; NOTES says scout used or why not

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-ui -p graft` · `cargo test --doc -p graft-ui`.

## Size

One context. graft-ui footer files + `graft/src/view/rows.rs`, `view/frame.rs`, `update/remote.rs` (Sticky shape). Disjoint from `ui-ghost-banner-conflict` (header slot, confirm, detail) by file, same crate: separate worktrees; merge this one second and resolve `GraphView`'s initialisers.

## NOTES (builder)

Scout: used (Haiku `Explore`) before building, per ADR-0035. Its findings
(footer slot precedence, `changes_pane`/`. explain` cases, `remote_failed`'s
`reason · keys` shape) drove the design below; not re-verified line by line
here since the compiled result and tests are the check that matters.

Compacted mid-ticket once; conductor resumed from a summary and finished
the remaining work (`view/rows.rs`'s `hint_context`, the graft-crate
`Status::Sticky` fallout, gates, captures, docs) in the same worktree/context
line as the original builder — reporting as one ticket, not split, since
everything landed and gates are green.

Acceptance:
- [x] (a) `no_context_hides_every_key` — `crates/graft-ui/src/graph_screen/hints/tests.rs`
- [x] (b) `eighty_columns_shows_three_and_more` — same file
- [x] (c) `hundred_twenty_shows_six` — same file
- [x] (d) `hundred_sixty_shows_all` — same file
- [x] (e) `marked_leads_with_explain_and_omits_more` — same file
- [x] (f) `status_line_keeps_its_keys` — same file
- [x] (g) `footer_eighty_branch_row` — `crates/graft-ui/src/graph_screen/tests.rs`
- [x] (h) `push_lease_refused_stale`/`m4-06-rejected-sticky-80x24` — the fitted
  line equals the old blind-cut line here (72 of 80 cols; the capture needed
  no content change) — `crates/graft/tests/it/remote_by_key.rs`
- [x] 12 captures: `docs/shots/m5-hints-{branch,plain,uncommitted,marked}-{80x24,120x40,160x48}.txt`,
  regenerated by `capture_hint_bar_for_docs_shots` (`crates/graft-ui/src/graph_screen/tests.rs`),
  each under 2 KB; `TESTING.md` § Hint bar lists the exact footer string per
  capture
- [x] `grep -rn "144\|155\|156" crates/graft-ui/src/graph_screen*` — only hit is
  a doc-comment in `hints.rs` naming what the redesign replaced, no live
  threshold
- [x] KNOWN_GAPS rows 493 (`Remote footer hints absent below 144 columns`) and
  495 (`At 80 cols the rejection hint cuts to "or P to fo…"…`) named
  closable: 493 fully — the width rule is exactly `hints_for`'s tiers now.
  495 **partially** — the rejection-hint half is fixed (`fit_status_line`
  keeps keys whole, test f); the same row also names the *journal push
  row's* `"not undoa…"` cut, which is a different piece of text
  (`view::journal_view`'s summary line) this ticket never touched — that
  half stays open.
- [x] MAP.md updated: `crates/graft-ui/MAP.md` (`graph_screen`,
  `graph_screen::hints`, `GraphView::problem`/`hint_context`, `ProblemLine`),
  `crates/graft/MAP.md` (`Status::Sticky`, `view::rows::graph_view`/
  `hint_context`, `view::status::graph_problem`, `update::keys::bound_keys`)

Deviations from the ticket:
- The lane-fold footer hint (`+ expand lanes`/`+ collapse lanes`) is dropped
  from the footer: it is not in the design note's per-context table. The
  `+` key still works; it is simply no longer hinted. Existing tests that
  asserted the old text now assert `⏎ open` instead, with a comment.
- `bound_keys()` lists only keys actually bound today. `m`/`r`/`b`/`t`
  (`BranchRow`'s merge/rebase/branch/tag) and `s`/`a` (banner skip/abort)
  are in the design note's hint table verbatim (as scope item 1 requires)
  but not yet bound anywhere in code — those verbs are other M5 groups' to
  build. `update::tests::every_hinted_key_is_bound_or_a_named_pending_group`
  checks every other hinted key against `bound_keys()` and names these six
  explicitly as the pending exception, rather than silently widening
  `bound_keys()` to make the test pass.
- `Status::Sticky(String)` → `Status::Sticky { reason: String, keys: Option<String> }`
  (the signature block's suggestion, confirmed as the smaller change): every
  existing call site compiles with `keys: None` via `Status::sticky(reason)`;
  `update::remote::remote_failed` still builds `ops`'s combined
  `"reason · keys"` line and splits it once downstream
  (`split_reason_and_keys`), since the convention is `ops::apply::remote::explain`'s
  and reshaping that crate was out of this ticket's crate set.
- `view::status::problem` (single combined line, status screen/stash) is kept
  unchanged; `view::status::graph_problem` (new, `Option<ProblemLine>`) is
  the graph footer's own reading. Two functions, not one field split two
  ways, to avoid an invalid state (`reason` and `keys` as separate unrelated
  `Option`s that could disagree).

Gates (workspace-wide, not `-p`, per CLAUDE.md — stricter than this ticket's
own `-p graft-ui -p graft` line): `cargo fmt --check` clean; `cargo clippy
--all-targets -- -D warnings` clean; `cargo nextest run` 1890/1890 passed, 28
skipped (heavy tier); `cargo test --doc` clean (2 doctests, graft-fixtures).

Not verified: colour (captures are character grids, ADR-0037's own rule);
the reviewer's own read of the 12 captures by eye.
