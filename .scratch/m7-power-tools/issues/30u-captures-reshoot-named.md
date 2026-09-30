# captures-reshoot — re-take the 15 pty captures whose drift is a named change

Status: ready
Build with: **Sonnet.** Mechanical: run the walks with `GRAFT_UPDATE_SHOTS=1`, then prove each diff is only the named
change. Anything else stops that file.
Needs: nothing. Base: `main`.
Runs concurrently with: `captures-verify-questions` (disjoint files), `f-recover-set-aside-bytes`,
`f-guard-refusal-at-80` (neither changes these screens; Touches checks it).

## Why
26's compare run: 26 capture files drifted. These 15 have their change named (26 NOTES "pty compare", and the m4 files'
own headers). A stale capture shows the user a screen the app no longer draws (rule 6: he judges by these).

| Files | Named change (26 NOTES) | Cite (find and write the line) |
|---|---|---|
| `docs/shots/m4-03-credential-masked-{80x24,160x48}.txt`, `docs/shots/m4-04-credential-username-{80x24,160x48}.txt` | the header's own note: `PREDATES m6-conflicts/69 and 78 … the app now draws ⏎ send  esc cancel on the bottom row and esc cancel push main → origin/main in the border` | the m6-conflicts/69 and /78 commits (`git log --grep`) |
| `docs/shots/m5-01-preview-rebase-{80x24,160x48}.txt` (:23, :47) | M7's rewrite engine: `git rebase --no-update-refs --no-autostash main` → `git rebase -i --no-update-refs --no-autostash --empty=drop main` | `git log -S'--empty=drop'` in graft-ops; the M7 ticket it belongs to |
| `docs/shots/m6/m6-06-relaunch-sticky-{80x24,120x40}.txt`, `docs/shots/m6/m6-07-all-resolved-{80x24,120x40}.txt` | M7's banner: `REBASING step 1 of 2` → `REBASING  step 1 of 2 · pick <sha7> feature writes` | `git log -S` on the banner's new words; the M7 ticket |
| `docs/shots/m6/m6-19-after-esc-esc-{80x24,120x40,160x48,187x50,200x50}.txt` | M7's keymap: the banner bar gained `c finish` and `C`, lost `c` and the `merge`/`rebase` labels | the `docs/design/02-keymap.md` line that gives `C` and `c finish`; the commit |

## Files (exact)
- The 15 capture files above.
- `docs/shots/m4-contact-sheet-captions.tsv`: the m4-03/m4-04 rows lose the `· PREDATES …` tail (their caption text).
  Nothing else in it.
- `TESTING.md`: only lines that quote these captures' old text (grep each old string).
- This ticket's NOTES.
- Not: contact sheets (`m7-acceptance-finish` rebuilds them); any crate; any walk script; any other capture.

## How
1. Build as TESTING.md :2173 says (`-p graft --example m5_walk --bin graft`, `-p graft-fixtures --bin graft-fixtures`,
   `GRAFT_TARGET`); m4 as TESTING.md :1478 says. Logs outside the repo (`$CARGO_TARGET_DIR/pty-logs/`), never
   committed; read summary lines only.
2. Run `.scratch/m4-remotes/pty-walk-m4.py`, `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py`,
   `.scratch/m6-conflicts/pty-walk-m6.py` at each size above with `GRAFT_UPDATE_SHOTS=1`.
3. `git checkout --` every file the run rewrote that is **not** in the 15 (the other owner's; list them in NOTES).
4. For each of the 15: `git diff` old vs new, **whole frame**. Every changed line is the named change (or its direct
   layout effect: a line that moved because a word grew). Anything else → `git checkout --` that file, **stop for it**,
   and write in NOTES the file, the line, old and new. No guess at a cause.
5. Run compare mode (no `GRAFT_UPDATE_SHOTS`) at every size: green for the files kept.

## Acceptance (name the verifying command at tick time)
- [ ] NOTES: per row of the table, the cite (commit and, for m6-19, the keymap line) found in step "Cite".
- [ ] NOTES: per file, `re-shot` with "every changed line is <the named change>", or `stopped` with its line.
- [ ] Compare mode green for every re-shot file (the command and its summary line).
- [ ] The m4 headers no longer carry `PREDATES` (`grep -l PREDATES docs/shots/m4-0[34]-*.txt`: none) and neither do
      their captions rows.
- [ ] `TESTING.md`: each old quoted string gone (grep: 0 hits), replaced by the new text; the m5-01 command line and
      the m6-06/07 banner words included if TESTING quotes them.
- [ ] Each capture ≤ 200 KB (`wc -c`).
- [ ] The captures' look is the user's (rule 6); no "looks right" here.

## Touches
- The other drifting files (verify part's): restored after the run; `git status docs/shots` lists only the 15 and the
  tsv.
- `f-guard-refusal-at-80` changes the in-the-way refusal row only: grep the 15 new files for `is in the way`; none
  expected.
- `f-recover-set-aside-bytes` changes no drawn text.
- m4 contact sheet: its rebuild is `m7-acceptance-finish`'s; the tsv edit here feeds it.
- `docs/shots/m4-credential-prompt-in-app.txt` (the in-process capture the header points to): untouched.

## Gates
No Rust changed → the compare runs. `git diff --stat crates/` empty.

## Size
~35k.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Raw logs never
committed.
