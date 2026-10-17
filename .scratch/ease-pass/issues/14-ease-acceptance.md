# ease-acceptance — walk the three mocks in the real app

Status: ready (2026-10-17: a team-topology repo and G12 assertions added for 01r; 03b added to Blocked by / Needs and Q8 extended, 2026-10-17; 05b, 06b, 06c added and Q16/Q17 and the fold checks added after the 05/06 scouts, 2026-10-17)
Blocked by: all of 01–13, plus 01r, 03b, 05b, 06b, 06c
Build with: Fable. This is the acceptance ticket: it judges the promise against the mocks.
Needs: every other ease-pass ticket: roles-and-route, roles-and-route-fixes (01r), ease-vocabulary,
ease-vocabulary-rest (03b), commit-what-goes-in, clash-reads, resolver-picks, merge-review-at-y (05b),
graph-fold-and-lanes, lanes-on-screen-and-header (06b), fold-seek-reveal (06c), graph-rows, branch-pane,
pixel-roles-and-tall-row, commit-screen, merge-preview, clash-screen-look, finish-review-and-look.

## Read first
- `docs/design-notes/ease-pass.md`: Promise, Journeys (including the `journeys` fence), Mock behaviours (G1–G14,
  C1–C15, M1–M19), New UI, Hard-to-reverse choices 2 and 3 (as amended 2026-10-17), Questions (1–13 answered a;
  14–17 as answered by then).
- `.scratch/ease-pass/issues/01r-roles-and-route-fixes.md`: "The rules, corrected" (R1–R3, Known limits).
- Each pass-2 spec's Acceptance captures list (file names only).
- `TESTING.md`: the M8 acceptance section only, for its form. `scripts/m8-accept.sh` usage line;
  `scripts/contact-sheet.sh` usage; `crates/graft/MAP.md` entry for `m8_walk`.
- ADR-0037 (contact sheet), ADR-0063 Consequences.

## Scope
In:
- `scripts/ease-accept.sh`: one command. It builds a fixture repo shaped like the mocks, walks every journey and every
  behaviour line in a pty, writes captures, and prints one summary line per scenario. The mock-shaped repo has:
  - `main ← wallet-release ← sara/fix-toc`, and teammates' branches by other authors;
  - PR-style merges `Merged PR <n>: …` with the branch deleted, plus a named merge, plus a sync merge (main into a
    feature branch);
  - a tag, and a branch 1 ahead of its upstream;
  - a new `.env` plus 3 modified files;
  - a merge where one file clashes in 1 place, and another where files clash in 3 places across 2 files;
  - a rebase that clashes.
- **A second small repo, `team`** (01r topologies; separate, so the journeys' commits never move these branches):
  - `develop` (an integration branch by ADR-0001 precedence) off `main`;
  - ordinary `wallet-release` off `develop`, with `sara/fix-toc` cut from its tip;
  - `ali/x` cut from `wallet-release` at sara's fork, then one more commit on `wallet-release`;
  - `bob/x` cut from an older sara commit. It commits `x1`, then runs `git merge sara/fix-toc`.
- `ease_walk::within_the_note`: reads the note's `journeys` fence, and fails when a walk takes more keys than its row.
- **Step counts, asserted by name.** Both fail one key over, and both fail if the end state isn't reached in that count:
  - `commit-all-subject` = **3** (`c`, typed subject, `⏎`), ending with HEAD one commit ahead and the cursor on it;
  - `merge-one-clash` = **5** (`m y o y y`), ending with a two-parent HEAD and the cursor on it.
- One assertion per behaviour ID (G1–G14, C1–C15, M1–M19), named by the ID in its test name. Text and shape checks
  only.
- **G12 on `team`**, each a named `g12_…` assertion, text only. The merged and fresh-branch states have no row the
  cursor can rest on (merged rows fold; opened rows take the merge row's branch), so 01r's unit tests cover them and
  this walk doesn't. With the cursor on a sara row:
  - `g12_sibling_never_built_on`: the `built on` line names neither `ali/x` nor `wallet-release` (wallet moved, so by
    Q15 (a) it reads `main ─◆ develop ─◆ sara/fix-toc`); `ali/x`'s lane is `┊`;
  - `g12_teammate_sync_is_not_a_merge`: the merged line reads `not merged`, never `merged into bob/x`;
  - `g12_dependent_before_the_move`: run on a copy of `team` taken before wallet-release's extra commit. The `built on`
    line reads `main ─◆ develop ─◆ wallet-release ─◆ sara/fix-toc`.
- **Fold checks** (06, 06c), each named `fold_…`, text only:
  - `fold_sync_merge_has_no_mark`: the sync merge's row carries no `▸`;
  - `fold_reveal_parent`: detail pane on a folded PR merge, follow parent 2 → the row reads `▾ …`, cursor on parent 2;
  - `fold_same_without_commit_graph`: the mock repo with `.git/objects/info/commit-graph` removed — once the view
    has promoted (wait on the promotion event, not a sleep), the rows' text equals the commit-graph run's.
- **Picks in a shell-begun merge** (05, 05b), named `picks_shell_merge`: `git merge` in the shell, launch → banner
  focused; `⏎ o u o y y` → merged; `git ls-files -u` non-empty until the last `y`.
- One assertion per answered question, named `q<N>_…`:
  - Q1: `C` on a teammate's row → picker on that branch; `⏎` switches. 2 keys.
  - Q2: the bar on a teammate's row has `x compare`; `x x` across two rows still compares.
  - Q3: `#` → no colour attributes anywhere in the frame, glyphs unchanged, still off after opening and closing the
    commit screen; `#` again restores colour.
  - Q4: `docs/perf/ease-pixel.txt` exists with `SUMMARY` lines for base / roles / roles+tall row. Quote the summary in
    the report; **don't judge it** (the results go to him).
  - Q5: ghost rows drawn above the merge preview box.
  - Q6: the merge preview has no `[ Cancel ]`/`[ Confirm ]`; `⏎` opens the incoming list.
  - Q7: `esc` on the clash screen → graph, banner focused, bar leads `⏎ back to the clashes`; `⏎` → first open clash.
    Journey `pause-and-resume`, 2 keys.
  - Q8: no option row on the commit screen; `⌃S` adds a sign-off; `⌃P` from the commit screen lists amend, skip hooks
    and allow empty spelled `—`, and choosing skip hooks shows `· skipping hooks` in the header.
  - Q9: `u` after a commit → edits uncommitted, and the next `c` shows the old message.
  - Q10: opening a folded merge → its rows on their own lane joining the merge.
  - Q11: the top line keeps today's frame and ends with `lanes drawn: N of M`; the bar end is today's.
  - Q12: a rebase clash uses the marks and advance; `⏎` stages the file; `c` goes on.
  - Q13: a PR merge's folded row reads `▸ N commits from PR <n>`.
  - Q14: as answered. Under (a) the top line names the branch once.
  - Q15: as answered. Under (a) it is `g12_sibling_never_built_on` above.
  - Q16: as answered. Under (a) `o` on a taken clash → `t` replaces it, and `u` brings `o`'s take back.
  - Q17: as answered. Under (b) a rebase stop lands on the graph with the banner focused; under (a) on the first clash.
- Captures:
  - every file the pass-2 specs list under `docs/shots/ease-pass/`, re-taken by the walk at their widths (120, 100,
    90);
  - plus `main-pane-sibling-120.txt` and `main-pane-dependent-120.txt` from `team`;
  - each ≤200 KB.
  - Captions file `docs/shots/ease-pass/ease-pass-contact-sheet-captions.tsv`: filename, what right looks like,
    matching mock path.
  - Sheet from `scripts/contact-sheet.sh --captions-order ease-pass docs/shots/ease-pass`.
- `TESTING.md` "## Ease pass acceptance": per scenario, concrete words, never "looks correct". For example:
  - "cursor row has a second line starting with the author's name and ending with a 7-char hash";
  - "`lanes drawn: 5 of 8` at the top line's right end";
  - "`.env` listed under Staying out with `!`";
  - "`built on` line on sara never names `ali/x`";
  - "the sync merge's row has no `▸`".

  Every line where colour carries meaning says **colour: unverified** (rule 5).

Out: fixing what fails. File it back to the owning ticket with file and line (rule 7).

## Signatures
```rust
pub mod ease_walk {
    pub struct JourneyRow { pub id: String, pub keys: u32, pub list: String }
    pub fn journeys_in(note: &str) -> Result<Vec<JourneyRow>, FenceError>;
    pub fn within_the_note(id: &str, walked_keys: u32, note: &str) -> Result<(), OverCount>;
}
```
Mirror `m8_walk`'s shapes. If `m8_walk` already offers these generically, reuse them and say so.

## Context guard
Captures, perf output and raw pty streams go to files. The builder reads summary lines and paths only. Never paste a
capture into the report.

## Touches
`TESTING.md`, `scripts/`, `docs/shots/ease-pass/`, the app's test tree. No product code.

## Acceptance
- [ ] `scripts/ease-accept.sh` exits 0 and prints 9 journey lines with walked/allowed counts, including exactly
  `commit-all-subject 3/3` and `merge-one-clash 5/5`.
- [ ] A deliberately padded walk (one extra key) of each of those two fails `within_the_note` (negative test).
- [ ] Every behaviour ID G1–G14, C1–C15, M1–M19 has a passing test named with the ID. The three `g12_…`, the three
  `fold_…`, `picks_shell_merge` and every `q1`–`q17` assertion (as answered) pass. NOTES lists IDs → test names.
- [ ] Every capture under `docs/shots/ease-pass/` is ≤200 KB. The captions file has one line per capture with its mock
  path. The contact sheet assembles with no `NOT RECORDED`.
- [ ] The `TESTING.md` section is present, with one entry per capture and colour marked unverified.
- [ ] Pixel summary lines quoted from `docs/perf/ease-pixel.txt`, or the gap recorded in `KNOWN_GAPS.md` as
  **unverified**.
- [ ] Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc`, all workspace-wide; plus `scripts/ease-accept.sh`.

## Size
Large (~75k). If it overruns, split by screen (main / commit / merge).
