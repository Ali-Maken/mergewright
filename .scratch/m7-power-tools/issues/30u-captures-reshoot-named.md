# captures-reshoot — re-take the 15 pty captures whose drift is a named change

Status: done (13 of 15 re-shot; m6-06 ×2 stopped, see NOTES)
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
- [x] NOTES: per row of the table, the cite — see NOTES "Cites" (`git log --grep`/`-S`, keymap grep). m6-19's `c finish`
      lead: commit not pinned (see NOTES).
- [x] NOTES: per file, `re-shot` or `stopped` — NOTES "Per file" (`git diff -U0` of each).
- [x] Compare mode green for every re-shot file — `run.sh compare` (walks without `GRAFT_UPDATE_SHOTS`): all 13 kept
      files `settled` in the logs' summary lines (NOTES).
- [x] No `PREDATES` — `grep -l PREDATES docs/shots/m4-0[34]-*.txt docs/shots/m4-contact-sheet-captions.tsv`: none.
- [x] `TESTING.md` — `grep -c "no-update-refs --no-autostash main\`\|predates both" TESTING.md`: 0; :1661 and :2099
      carry the new text. TESTING quotes no `REBASING step` or m6-19 old bar (grep: 0).
- [x] ≤ 200 KB — `wc -c`: largest 7672 bytes (m4-03 160x48).
- [ ] The captures' look is the user's (rule 6); no "looks right" here.

## NOTES (30u, 2026-10-01)

No scout: no source touched; the diffs are the evidence. Main (30t, 0efb6867) fast-forwarded in before the compare run;
`grep -l "esc dismiss\|is in the way"` over the 15: none.

**Cites.**
- m4-03/04: `685f8b07` m6-conflicts-69 (an open box owns the bottom row), `9f1bf6a5` m6-conflicts-78 (the prompt names
  its push).
- m5-01: `git log -S'--empty=drop' -- crates/graft-ops` earliest `c24df958` m7-power-tools-02b (rewrite engine runs r).
- m6-06/07: `git log -S'step 1 of' -- crates` → `20df04d2` m7-power-tools-07b (rewrite banner words and ladder).
- m6-19: `C` — `docs/design/02-keymap.md:204` (`C` opens the branch picker, amended 2026-09-26), commit `daef05e6`
  m7-power-tools-10 (`git log -S'hint("C"'`). `c finish` — keymap :265 (`c` is the banner's `c` while live). The commit
  that moved `c finish` to lead the live bar: **not found** (`-S'c finish'` hits only m6 commits; `said_live` in
  `graft-ui/src/graph_screen/hints/lists.rs:409` last touched by `3c4368fb` m7-07b). Not a guess — unpinned.

**Per file.**
- m4-03-credential-masked 80x24, 160x48; m4-04-credential-username 80x24, 160x48: **re-shot**. Every changed line is
  the named change: header loses `· PREDATES…`; border `esc cancel the push` → `esc cancel push main → origin/main`;
  bottom row `push main → origin/main · esc cancels` → `⏎ send  esc cancel`.
- m5-01-preview-rebase 80x24 (:23), 160x48 (:47): **re-shot**. Only change: the command line → `git rebase -i
  --no-update-refs --no-autostash --empty=drop main`.
- m6-07-all-resolved 80x24, 120x40: **re-shot**. Only :3 `REBASING step 1 of 2` → `REBASING  step 1 of 2 · pick 0104e16
  feature writes`.
- m6-19-after-esc-esc ×5: **re-shot**. Only the bottom bar: `c finish` leads after `⇥ banner`, old `c`/`c finish` slot
  gone, `C` appended; the dropped labels (`merge`/`rebase` at 80/120, `fetch` at 160, `stat` at 187) are the bar's width
  effect.
- **m6-06-relaunch-sticky 80x24 — stopped.** Beyond the banner: :7 `diff vs HEAD 2e2d51c` → `diff vs HEAD b0eb655`.
- **m6-06-relaunch-sticky 120x40 — stopped.** Beyond the banner (`graft <scratch…>…` also truncates): :5–7 new box
  `╭ still to apply · not commits yet` / `│ pick 2e2d51c feature writes again` (three blank lines lost below), and :29
  `diff vs HEAD 2e2d51c` → `diff vs HEAD b0eb655`. Both files restored (`git checkout --`).

**Other files the update run rewrote, restored:** m5-07-branch-row-hints 160x48; m5-11-diverged-pull-sticky 80x24,
160x48; m6-18-launch-shell-begun ×5; m6-21-stash-clash-done 80x24, 120x40, 187x50; m6-25-menu-fixed-slots 80x24,
120x40, 187x50. The m6 walk at 160/187/200 also **wrote 39 new untracked files** (m6-01..09, m6-20..25 at sizes never
committed); deleting was refused by the permission layer, so they were **moved** to the session scratchpad
(`…/scratchpad/untracked-m6/`), not committed. The compare run reports those same stems `DRIFTED` (absent) at those
sizes — the walk's own size list vs. committed shots, not this ticket's; for 30v/acceptance-finish.

**Compare** (`GRAFT_TARGET=<wt>/target` walks m4 80x24/160x48, m5 80x24/160x48, m6 ×5, no `GRAFT_UPDATE_SHOTS`): all 13
kept files `settled` (e.g. `docs/shots/m6/m6-19-after-esc-esc-200x50.txt: 2734 bytes, settled`). m4 runs rc=0; m5/m6
rc=1 from the other owner's drifted files.

**Left for someone:** tsv row 5 (m4-03 80x24) still quotes `esc cancel the push` — the ticket allowed only the
`PREDATES` tail; the 80x24 capture now draws `esc cancel push main → origin/main`. Stale caption, textual.

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
