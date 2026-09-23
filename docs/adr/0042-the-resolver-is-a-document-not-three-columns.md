# ADR-0042 — The conflict resolver is a document, not three columns

Date: 2026-09-22
Status: superseded by ADR-0043 (2026-09-23). Was: accepted (conductor, on the M6 design pass; design functionality is
delegated to `planner-fable` by the user's standing instruction of 2026-09-22)

## Context

`DESIGN.md` / `docs/design/01` §1.5 specifies M6's resolver as **three
full-height columns** — ours, theirs, result — with a `CONFLICTED FILES` strip.
That shape was chosen in the design phase, before M1–M5 were built and before
the hint-bar shrink ladder (ADR-0039) made the cost of narrow columns concrete.

The M6 design pass proposes replacing it, and five smaller spec edits with it.
This record is the approval, so that the tickets stop following a superseded
§1.5 and start following this.

## Decision

**The resolver shows one file as a document.** Plain lines are drawn as they
are. Each conflict block — what git delimits with `<<<<<<<` and `>>>>>>>` — is
drawn as a labelled card with **OURS and THEIRS side by side at ≥100 columns,
stacked below that**. BASE is a third labelled section inside the card, shown on
demand (`B`). `o`/`t`/`b` rewrite that block **on disk**; the card collapses to
the taken lines and keeps its header, so a take can be changed until the
resolver closes.

Ours, theirs and result are all on screen. **The result is the page.**

### Why, in the order the reasons actually weigh

1. **Three independent columns need a live three-way row alignment** that
   re-runs on every edit — a real algorithm, and one with no ground truth to
   test against. The in-place model needs none: git has already delimited the
   sides. This is the reason that decides it; the other two only confirm it.
2. **At the widths the user works at, three columns are unreadable.** At 80 and
   120 columns, three columns are 25 and 38 cells. Halves at 120 are 58;
   stacked at 80 are 78. Code does not survive 25 cells.
3. **Nothing can diverge from disk.** What is drawn is byte-for-byte what `⏎`
   and `space` stage. There is no result buffer to fall out of step, and `e`
   (`$EDITOR`) and `M` (`git mergetool`) act on the same file, with the resolver
   re-reading on return.

**No text editor is built.** §2.6's "edit the result inline" is `$EDITOR`, per
ADR-0038 §3.

**Marker labels are git's own, verbatim** (`HEAD`, `feature/auth`,
`3f2a1b (Add OAuth provider)`), plus one explainer line on a rebase — *ours =
main, what you are rebasing onto · theirs = the commit being replayed*. That
reversal is the classic confusion of rebasing, and ADR-0041's carve-out makes
answering it functionality, not polish: it is answered where it happens.

### Keys, which are a spec edit and not a build decision

`docs/design/02-keymap.md` is the single place shared namespaces are decided. It
gains, and the binding ticket does not build until it has:

- **`⏎` on a `CONFLICTS` row (§2.4) and on the focused banner (§2.6a) opens the
  resolver.** This is §2.2's existing global *open / drill in*, applied. No new
  letter is spent.
- **`B` shows and hides BASE** inside the resolver — UPPERCASE toggles a mode
  (§2.1). `b` stays take-both.
- **`d` starts this file over** inside the resolver (`checkout --merge`, behind
  a confirm box, and **not undoable while the operation is open** — the box must
  say so). `d` is the discard verb and this is its one meaning.
- **`Esc` in the resolver closes the resolver** and never aborts the operation.
  The banner's own `Esc` stays exactly as M5 built it.

Three further edits are documentation catching up with decisions already made:
`M` joins §2.1's opens-a-place set (a hand-over *is* a place you go to and come
back from — KNOWN_GAPS row 482); `e` is `$EDITOR`, not an inline editor; and the
commit editor is listed as modal, which §2.10 already says (row 571).

## Consequences

- **§1.5 is superseded** by the note's "New UI — the conflict resolver" section.
  `docs/design/01` and `02-keymap.md` are edited before group 04 builds; until
  they are, the specs mark these keys *pending* and no ticket binds them.
- **The user has not seen it.** Rule 6 stands: this is a shape, judged by eye,
  on a capture, by him — not by me and not by a reviewer. Approving the design
  is not approving the look, and the M6 acceptance ticket owes a capture.
- **Reversible at a real cost.** The document model is the resolver's data
  shape, not a skin over a column layout, so going back to three columns after
  group 04 means rebuilding it. The cheap moment to disagree is the first
  capture.
- `graft-ui`'s layout is a pure function of a fixed data shape, so the alignment
  algorithm that three columns would have needed never gets written, and is not
  a maintenance surface anyone inherits.

## Related

ADR-0038 §3 (`$EDITOR` hand-over), ADR-0039 (the hint-bar ladder, whose widths
made reason 2 measurable), ADR-0041 (functionality first — and its carve-out,
which is why the rebase explainer line ships in M6 rather than M8).
