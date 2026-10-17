# ease-acceptance — walk the three mocks in the real app

Status: ready
Blocked by: all of 01–13
Build with: Fable — the acceptance ticket; judges the promise against the mocks.
Needs: every other ease-pass ticket: roles-and-route, ease-vocabulary, commit-what-goes-in, clash-reads, resolver-picks, graph-fold-and-lanes, graph-rows, branch-pane, pixel-roles-and-tall-row, commit-screen, merge-preview, clash-screen-look, finish-review-and-look.

## Read first
- `docs/design-notes/ease-pass.md`: Promise, Journeys (incl. the `journeys` fence), Mock behaviours (G1–G14, C1–C15, M1–M19), New UI, Questions (1–13 answered a; 14 as answered by then).
- Each pass-2 spec's Acceptance captures list (file names only).
- `TESTING.md`: the M8 acceptance section only (for its form); `scripts/m8-accept.sh` usage line; `scripts/contact-sheet.sh` usage; `crates/graft/MAP.md` entry for `m8_walk`.
- ADR-0037 (contact sheet), ADR-0063 Consequences.

## Scope
In:
- `scripts/ease-accept.sh`: one command; builds a fixture repo shaped like the mocks (`main ← wallet-release ← sara/fix-toc`; teammates' branches by other authors; PR-style merges `Merged PR <n>: …` with the branch deleted; a named merge; a tag; a branch 1 ahead of its upstream; a new `.env` plus 3 modified files; a merge where one file clashes in 1 place, and another where files clash in 3 places across 2 files; a rebase that clashes); walks every journey and every behaviour line in a pty; writes captures; prints one summary line per scenario.
- `ease_walk::within_the_note`: reads the note's `journeys` fence; fails when a walk takes more keys than its row.
- **Step counts, asserted by name:** `commit-all-subject` = **3** (`c`, typed subject, `⏎`) ending with HEAD one commit ahead and the cursor on it; `merge-one-clash` = **5** (`m y o y y`) ending with a two-parent HEAD and the cursor on it. Both fail on one key over; both also fail if the end state is not reached in that count.
- One assertion per behaviour ID (G1–G14, C1–C15, M1–M19), named by the ID in its test name. Text and shape checks only.
- One assertion per answered question, named `q<N>_…`:
  - Q1 `C` on a teammate's row → picker on that branch, `⏎` switches: 2 keys.
  - Q2 bar on a teammate's row has `x compare`; `x x` across two rows still compares.
  - Q3 `#` → no colour attributes anywhere in the frame, glyphs unchanged, still off after opening and closing the commit screen; restored by `#`.
  - Q4 `docs/perf/ease-pixel.txt` exists with `SUMMARY` lines for base / roles / roles+tall row; the summary is quoted in the report, **not judged** (results go to him).
  - Q5 ghost rows drawn above the merge preview box.
  - Q6 merge preview has no `[ Cancel ]`/`[ Confirm ]`; `⏎` opens the incoming list.
  - Q7 `esc` on the clash screen → graph, banner focused; `⏎` → first open clash: journey `pause-and-resume` 2 keys.
  - Q8 no option row on the commit screen; `⌃S` adds a sign-off.
  - Q9 `u` after a commit → edits uncommitted, next `c` shows the old message.
  - Q10 opening a folded merge → its rows on their own lane joining the merge.
  - Q11 top line keeps today's frame and ends with `lanes drawn: N of M`; bar end is today's.
  - Q12 a rebase clash uses the marks and advance; `⏎` stages the file; `c` goes on.
  - Q13 a PR merge's folded row reads `▸ N commits from PR <n>`.
- Captures: every file the pass-2 specs list under `docs/shots/ease-pass/` re-taken by the walk at their widths (120, 100, 90); each ≤200 KB. Captions file `docs/shots/ease-pass/ease-pass-contact-sheet-captions.tsv` (filename, what right looks like, matching mock path); sheet from `scripts/contact-sheet.sh --captions-order ease-pass docs/shots/ease-pass`.
- `TESTING.md` "## Ease pass acceptance": per scenario, concrete words (e.g. "cursor row has a second line starting with the author's name and ending with a 7-char hash"; "`lanes drawn: 5 of 8` at the top line's right end"; "`.env` listed under Staying out with `!`"), never "looks correct". Every line where colour carries meaning says **colour: unverified** (rule 5).
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
Captures, perf output and raw pty streams go to files; the builder reads summary lines and paths only. Never paste a capture into the report.

## Touches
`TESTING.md`, `scripts/`, `docs/shots/ease-pass/`, the app's test tree; no product code.

## Acceptance
- [ ] `scripts/ease-accept.sh` exits 0; prints 9 journey lines with walked/allowed counts, including exactly `commit-all-subject 3/3` and `merge-one-clash 5/5`.
- [ ] A deliberately padded walk (one extra key) of each of those two fails `within_the_note` (negative test).
- [ ] Every behaviour ID G1–G14, C1–C15, M1–M19 has a passing test named with the ID; every `q1`–`q13` assertion passes; a list of IDs → test names goes in the ticket NOTES.
- [ ] Every capture under `docs/shots/ease-pass/` ≤200 KB; captions file has one line per capture with its mock path; contact sheet assembled with no `NOT RECORDED`.
- [ ] `TESTING.md` section present, one entry per capture, colour marked unverified.
- [ ] Pixel summary lines quoted from `docs/perf/ease-pixel.txt` (or the gap recorded in `KNOWN_GAPS.md` as **unverified**). Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide; plus `scripts/ease-accept.sh`.

## Size
Large (~75k). Split by screen (main / commit / merge) if it overruns.
