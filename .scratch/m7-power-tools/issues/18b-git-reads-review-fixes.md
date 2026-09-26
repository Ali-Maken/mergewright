# 18b git-reads-review-fixes — review 18/19/27/10a's Majors and mutation gaps

Status: ready
Blocked by: none (18, 19, 27, 10a merged)
Crates: graft-git only (`blame.rs`, `trace/log.rs`, `history_reads/unreached.rs`, `switch_refs.rs`, their tests),
plus a perf script and `docs/perf/trace-readers.txt`. Another crate needed → commit at the seam, report `blocked:
needs <crate>` with the signature.
Build with: Opus (git reads with oracles against real git; one perf run).
Scout: none. The review cites every site with file:line at `62ace7b0`; re-read each site before you edit it.
Split by: conductor, 2026-09-26, from `docs/reviews/m7-18-19-27-git-reads.md`. No new decision.

## Scope
Each fix is the one the review states, at the cause. Read the review's section for each.
- **Major 1** (blame ignores `truncated`): return an error, as the siblings do (`trace/log.rs:123`). A test through a
  lowered cap or a cut porcelain sample, red first.
- **Major 2** (`--not` on `--stdin`; floor git 2.38): **verify first**. Try a git 2.38–2.41 (a brew-extracted or
  source-built older git under `$TMPDIR`, never installed system-wide). Whatever it shows, send negatives as `^<rev>`
  lines, which every version takes. State in NOTES what you verified on which version, or that you could not.
- **Major 3** (blame perf unmeasured): a long file with interleaved history, measured from a script into
  `docs/perf/trace-readers.txt` as `blame-longfile p95=…`. Read summary lines only. Over 1 000 ms → report it, don't
  tune.
- **Major 4** (files over 400 lines): split `blame.rs` (porcelain parser → `blame/porcelain.rs`, delete its private
  `unquote`, Minor 13) and `trace/log.rs` (the `--follow` reader → `trace/follow.rs`). No behaviour change.
- **Mutation gaps** (§Mutation check): a test that fails under each surviving mutation: M1 and M4 (10a's two error
  rules), M15 (blame without `-M`), M17 (shallow end reads as `Start`), M23 (working-tree binary check).
- **10a `@{-1}` with a same-named tag** reads as detached (the review saw it on real git; see its Minor). Fix at the
  cause, with an oracle test, red first.

- **Minor 4** (enclosing function: output not checked for truncation; a capped diff reported as `Stopped`): the
  same rule-3 class as Major 1; fix it with it, and a test.

Not in scope: other Minors (they go to KNOWN_GAPS).

## Acceptance
- [ ] Major 1: the test named, red first then green.
- [ ] Major 2: NOTES say what was verified on which git; the `^<rev>` form; name the test that proves the scan still
  finds the same commits.
- [ ] Major 3: the `blame-longfile p95=` line in `docs/perf/trace-readers.txt`, and the script that wrote it.
- [ ] Major 4: both splits; no file in `crates/graft-git/src` over 400 lines (state the `wc -l` of the four files).
- [ ] Each of M1, M4, M15, M17, M23: its test named, seen red under that mutation.
- [ ] `@{-1}` with a same-named tag: the test named, red first.
- [ ] Minor 4: the test named, red first.
- [ ] Gates workspace-wide.

## Touches
MAP.md (graft-git) lines for moved code; every caller of `blame`, `unreached`, `switch_refs` (grep). Name how each
was checked.
