# ease-acceptance — walk the three mocks in the real app

Status: draft
Build with: Fable — the acceptance ticket; judges the promise against the mocks.
Needs: every other ease-pass ticket: roles-and-route, ease-vocabulary, commit-what-goes-in, clash-reads, resolver-picks, and pass 2's graph-fold-and-lanes, graph-rows, branch-pane, pixel-roles-and-tall-row, commit-screen, merge-preview, clash-screen-look, finish-review-and-look.

## Read first
- `docs/design-notes/ease-pass.md`: Promise, Journeys (incl. the `journeys` fence), Mock behaviours (G1–G14, C1–C15, M1–M19), New UI, the user's answers to Questions (as recorded by the conductor).
- `TESTING.md`: the M8 acceptance section only (for its form); `scripts/m8-accept.sh` usage line; `crates/graft/MAP.md` entry for `m8_walk`.
- ADR-0037 (contact sheet), ADR-0063 Consequences.

## Scope
In:
- `scripts/ease-accept.sh`: one command; builds a fixture repo shaped like the mocks (`main ← wallet-release ← sara/fix-toc`, teammates' branches, PR-style merges `Merged PR <n>: …`, a deleted merged branch, a new `.env`, one file clashing in 1 place and one in 2), walks every journey and every behaviour line in a pty, writes captures, prints one summary line per scenario.
- `ease_walk::within_the_note`: reads the note's `journeys` fence; fails when a walk takes more keys than its row.
- One assertion per behaviour ID (G1–G14, C1–C15, M1–M19), named by the ID in its test name. Text and shape checks only.
- Captures: `docs/shots/ease-pass/<mock>-<state>.txt` (`main-…`, `commit-…`, `merge-…`), widths 120 and 100 (and 90 where the note says the pane drops); each ≤200 KB. Contact sheet `docs/shots/ease-pass-contact-sheet.txt`, ticket order, each capture under its "what right looks like" line, each with the matching mock path beside it.
- `TESTING.md` "## Ease pass acceptance": per scenario, concrete words (e.g. "cursor row has a second line starting with the author's first name and ending with a 7-char hash"; "`lanes drawn: 5 of 8` in the top line"), never "looks correct". Every line says **colour: unverified** where colour carries meaning (rule 5).
- Pixel tier: the frame-time trace from `docs/perf/ease-pixel.txt` (X) summarised; not judged by eye here.
Out: fixing what fails — file it back to the owning ticket with file and line (rule 7).

## Signatures
```rust
pub mod ease_walk {
    pub struct JourneyRow { pub id: String, pub keys: u32, pub list: String }
    pub fn journeys_in(note: &str) -> Result<Vec<JourneyRow>, FenceError>;
    pub fn within_the_note(id: &str, walked_keys: u32, note: &str) -> Result<(), OverCount>;
}
```
Mirror `m8_walk`'s shapes; if `m8_walk` already offers these generically, reuse and say so.

## Context guard
Captures and raw output go to files; the builder reads summary lines and paths only. Never paste a capture into the report.

## Touches
`TESTING.md`, `scripts/`, `docs/shots/ease-pass/`, the app's test tree; no product code.

## Acceptance
- [ ] `scripts/ease-accept.sh` exits 0; prints 9 journey lines with walked/allowed counts (`commit-all-subject 3/3`, `merge-one-clash 5/5`, …).
- [ ] Every behaviour ID G1–G14, C1–C15, M1–M19 has a passing test named with the ID; a list of IDs → test names goes in the ticket NOTES.
- [ ] Every capture under `docs/shots/ease-pass/` ≤200 KB; contact sheet assembled.
- [ ] `TESTING.md` section present, one entry per capture, colour marked unverified.
- [ ] Pixel trace summary line present (or the gap recorded in `KNOWN_GAPS.md` as unverified). Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide; plus `scripts/ease-accept.sh`.

## Size
Large (~70k). Split by screen (main / commit / merge) if it overruns.
