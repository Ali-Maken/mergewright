# M8 Mergewright set (46–58r) — whole-app usability walk

**Counts: Critical 0 · Major 2 · Minor 11 · Questions 4.** Plus 3 findings from the earlier walk
(`docs/reviews/m8-usability.md`) seen still standing, and 4 seen fixed.

Reviewer, read-only, against `c5ce1587`. CLAUDE.md "Every milestone ends with a whole-app usability walk". Scored against
`docs/research/ux-principles.md` (**UX §n**, heuristics **H1–H10**) and `docs/design/02-keymap.md`. Step counts against
the `journey-counts` fence in `docs/design-notes/m8-polish-and-ship.md` and the Journeys and Lands tables in
`docs/design-notes/m8-polish-and-ship/pass-3-mergewright.md:33-51`.

## How this was checked, and what was not

- **Driven:** a release `mwr` built from `c5ce1587` into a scratch `CARGO_TARGET_DIR`, run under a pty (pyte) at
  187×30, 120×30 and 80×24. Unicode tier forced in config. `XDG_CONFIG_HOME` was a scratch folder. Five scratch worlds:
  a bare origin; his clone with `main`, `feature-a`, `feature-b` built on it, and `fix` (conflicts with `main` on
  `notes.md`); a teammate's clone that pushed two commits; a dirty tree; a `pre-push` hook that sleeps 6 s. "Observed"
  below means seen in a screen dump. The binary, worlds and driver were deleted afterwards. Nothing from them is
  committed.
- **Older-names repository:** built by hand, not by running an older build. I made a repo under Mergewright names
  (two switches), then renamed `.git/mergewright` to `.git/graft`, removed its marker, moved `refs/mergewright/*` to
  `refs/graft/*`, rewrote the journal's ref names, and renamed the config folder `mergewright/` to `graft/`.
  `mwr` then moved everything back. I did not build a Tributary-era repo, and I did not build a `graft:`-tagged
  set-aside stash; ticket 57's S1–S2 walks cover both.
- **pyte artifacts, ignored:** the Kitty probe prints as text (`apis=…` in the header, `GMergewright …=32,s=1…` on the
  empty state). The one leftover I could not explain is filed as **Q4**, not as a defect.
- **Behaviour:** every behavioural finding names the file:line that proves it, read at `c5ce1587` (rule 7).
- **Not run:** the gates, the heavy tier and the release perf run. They are outside this walk; the milestone code review
  owns them. This walk is not evidence that the gates pass.
- **Colour and look: not judged** (rules 5 and 6). No finding here rests on colour.
- **Not reached:** mouse and drag; a real network remote and credentials; `$EDITOR`/mergetool round trips; Pixel tier;
  the `.` menu; quit flows; clone and init.

## Daily tasks, walked

Counts follow UX §1.3 (keys + decisions; a typed string counts as 1). ✓ = holds · ✗ = fails (finding id) · – = n/a.

| Task | Keys observed | Note | H1 | H4 | H6 | H9 | 3.1 lands |
|---|---|---|---|---|---|---|---|
| Launch, stash newer than HEAD | 0 | 0, cursor on HEAD | ✓ | ✓ | ✓ | – | ✓ HEAD (earlier Major A fixed) |
| Open a Graft-era repo (`open-older-names-repo`) | `mwr` **1** | 1 | ✓ no `rename:` line | ✓ | ✓ | – | ✓ HEAD; config folder and refs moved |
| Undo after the move (`undo-after-rename`) | `u` **1** | 1 | ✓ `undo: checkout main ✓` | ✓ | ✓ | – | ✓ |
| J after the move | `J` | 1 | ✗ m2 | ✗ m2 | ✓ | – | – |
| Fetch, then pull | `f`, `p` **1** each | 1 | ✓ | ✓ | ✓ | – | ✓ new tip |
| Switch branch, dirty tree | `C <name> ⏎ y` **4** | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Pick the branch he is on (`picker-enter-current`) | `C <name> ⏎` **3** | 3 | ✓ | ✗ m1 | ✓ | – | ✓ HEAD, nothing written (`git reflog` unchanged) |
| Rebase his branch onto main | `r y` **2** | – | ✓ | ✓ | ✓ | ✓ | ✓ new tip |
| See that a rebase's old tip is safe (`journal-replaced`) | `J` **1** | 1 | ✓ `⇢ replaced … undo brings it back` | ✓ | ✓ | – | – |
| Same, after `u` on that rebase | `u J` | – | ? **Q1** | – | ✓ | – | – |
| Merge that stops on a conflict, finish | `m y o ⏎ y` **5** | 4 (row 845) | ✓ | ✗ standing m2 | ✓ | ✓ | ✓ resolver, then new merge commit |
| Rebase that stops on a conflict, finish | `r y t ⏎ c` **5** | – | ✓ | ? **Q2** | ✓ | ✓ | ✓ resolver, then new tip |
| `?` at a rebase stop | `?` **1** | 1 | ✓ IN PROGRESS first; `Take upstream`/`Take yours` | ✗ standing m2 | ✓ | – | – |
| Stage one file, commit | `k ⏎ space c <msg> ⏎` **6** | 6 | ✓ | ✓ | ✓ | – | ✓ new commit |
| Cherry-pick by search | `⌃P <words> ⏎ o y` **5** | 3 (seek) + 2 | ✓ | ✓ | ✓ | ✓ | ✓ new tip |
| Undo, redo | `u`, `U` **1** each | 1 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Push | `P y` **2** | – | ✓ | ✗ m4 | ✓ | – | ✓ |
| J while a push runs (58r `running`) | `J` | – | ✓ row reads `running` | ✗ **A** | ✗ **A** | – | ✓ row updates when the push lands |
| J after `u`/`U` on a switch | `J` | – | ✗ **B** | ✗ **B** | – | – | – |
| Seek a branch | `⌃P <name> ⏎` **3** | 3 | ✓ | ✓ | ✓ | – | ✓ |
| Practice tour from the empty state | `t` **1** | 1 | ✓ | ✓ | ✓ | – | ? **Q4** |
| Empty state: recent repos | – | – | ✗ m8 | ✓ | ✓ | – | – |

Every journey walked came in at or under the note's count, except row 845, which is already known. Both Majors are
about what a screen tells him, not step counts.

**Product name (this set's main ask):** observed `Mergewright` on the empty state, J's column and legend, the `.`
refusals, and the busy line. Observed `mwr` in `--help`, `--version` and the usage-error text. The config folder,
`.git/mergewright/` and `refs/mergewright/` were created and the Graft-era ones moved. No `Graft`, `Tributary` or
`trib` appeared on any screen I reached, except inside records, which is by design (ADR-0057 §12). The record cases:
ticket 57's capture shows a Graft-era stash message `graft: your edits, set aside…`
(`docs/shots/m8-mergewright-s1-graph-120x36.txt`). A grep of non-test string literals found the old words only in
fixtures, tests, `on_disk.rs`, and `graft-capabilities` (which renders the in-repo `CAPABILITIES.md`, a project doc).
Thread names and temp prefixes stay the codename per ADR-0057:226. The one name gap is m7: the man page.

## Critical

None.

## Major

### A. In J while a write runs, the bar says `J/esc close`, but `Esc` cancels the write

- **Observed:** a push running (6 s hook), `J` opened. J's row reads `running` (58r). The status row under the list
  reads `push main → origin/main · esc cancels`, and J's own bar reads `/ find  space command log  J/esc close
  ? keys`. Pressing `Esc` cancelled the push (`push main → origin/main cancelled ✓`, row `failed: cancelled`) and left
  J open. `origin/main` did not move.
- **Lines:** `crates/graft/src/update/keys.rs:262-269` handles `Esc` while any write runs, before the overlays
  (`pane_and_overlay_key` at `:287`). The comment there says this is on purpose. J's bar builds `J/esc close` with no
  check for a running write: `crates/graft-ui/src/journal_screen.rs:251-252`.
- **Why it matters:** one key carries two meanings on one screen (H4). The one he is likely to follow is the bar's:
  close J and get back to the graph. Instead he stops his own push or pull (H5). 58r gives him a reason to open J
  mid-operation (the new `running` word), so this path is now more likely. Cancelling a push is recoverable (push
  again), so this is not Critical. The same `esc close` wording probably shows on every overlay's bar during a write;
  I only observed J.
- **Fix:** while a write runs, overlay bars say what `Esc` will do: J's becomes `J close  esc cancels …`, from the same
  `remote::esc_hint`. Or `Esc` closes the overlay first and a second `Esc` cancels. That second option reverses a
  recorded decision (keys.rs:262-266), so it goes through the conductor. Add a test: J's bar while `operating` is set.

### B. Undoing or redoing a switch draws a blank `$ outside` row in J

- **Observed:** after `C feature-a ⏎ y`, `C main ⏎ y`, then `u` and later `U`, J shows a `$  outside  now` row with no
  description, expanding to `git: ""`, under each `● undo: checkout main` and `● redo: checkout main` row. `git reflog`
  has two HEAD lines with an empty message at those times. `⏎` on that row offers `only HEAD's log has this step, no
  branch's · b makes restored at d7943ab`.
- **Lines:** the restore puts `HEAD` back with `git symbolic-ref HEAD <branch>` and no `-m`
  (`crates/graft-ops/src/restore/steps.rs:131-135`), so git writes a reflog line with an empty subject. J claims a
  switch's reflog line only by its `checkout: moving from X to Y` subject (`crates/graft-ops/src/journal_list/owners.rs:51-58`),
  so the empty line is unclaimed and drawn as the shell's.
- **Why it matters:** J exists to say what happened and who did it. Here it reports a step he never took "outside
  Mergewright", with no words (H1, H2), while the legend says `$ done outside Mergewright`. Every switch's status line
  ends `u goes back` (`crates/graft/src/update/switch.rs:272`), so this is the advertised path, and each use leaves
  one of these rows. Nothing is lost.
- **Fix (cause):** give the restore's `symbolic-ref` a message (`-m "<tag>: undo …"`, or the `checkout: moving from …`
  form the claim already reads), and have `owners.rs` claim it for the undo/redo entry. Add a J test: undo of a switch
  draws no `$` row.

## Minor

- **m1. Picker on his own branch: the bar says the same words twice.** `⏎ show it on the graph  esc close  ⇥ show it
  on the graph` at 120 and 80 (observed). `picker_list_with` puts `PICKER_SHOW_VERB` on both `⏎` and `⇥`
  (`crates/graft-ui/src/graph_screen/hints/lists/editors.rs:59-66`). H8: at 80 columns the duplicate pushes `row` off
  the bar. Fix: on the current row, one clause `⏎/⇥ show it on the graph`, the way J does `J/esc close`.
  Capture: `docs/shots/m8-q2-picker-current-row-ui-120x36.txt:849`.
- **m2. The rename row breaks J's legend.** `● Mergewright … rename to Mergewright (2 refs, 1 file)` has no
  `not undoable`, but the legend says `● … u can undo it` (observed; capture
  `docs/shots/m8-mergewright-s1-journal-120x36.txt`). Push and fetch rows do say it. Causes:
  - The summary has arms only for Push and Fetch (`crates/graft/src/view/journal/summary.rs:66-84`), while `Migrate`
    is transparent to undo (`crates/graft-ops/src/journal.rs:235-240`).
  - `⏎` on the row says `Mergewright doesn't undo a step`. `crates/graft/src/update/journal/reading.rs:36-38` has no
    `Migrate` arm, and its comment ("answers false for no other kind") is stale.

  Fix: a `Migrate` arm reading `not undoable`, and the word `rename`.
- **m3. His own cancel reads `failed: cancelled`.** Observed after `Esc` on a running push: `failed: cancelled …
  nothing written`. His own `--abort` reads `aborted by you · nothing changed`
  (`crates/graft/src/view/journal/summary.rs:16`). The cancel goes through `with_failure`
  (`summary.rs:184`, also `:113`). H2/H4: a choice he made is shown as a failure. Fix: `cancelled by you · nothing
  written`.
- **m4. The confirm box and the preview box use the same words for different keys.** Push box: `y push  ←/→ move
  ⏎ choose  esc cancel` (`crates/graft-ui/src/graph_screen/hints/lists.rs:34-35`). Every preview box: `y confirm
  ←/→ choose  ⏎ act  esc cancel` (`crates/graft-ui/src/preview_box/footer.rs:35-36`). So `choose` means `⏎` in one
  and `←/→` in the other (H4). Fix: one pair of words for both boxes.
- **m5. The undo hint on success lines comes in three forms.** Observed: `on main ✓ · your edits are back · u goes
  back` (`crates/graft/src/update/switch.rs:272`); `took "…" onto fix ✓ · u undoes` (`update/operate/outcome.rs:96`,
  `update/recover.rs:265`); and no hint at all after rebase, merge and commit (`rebase fix onto main ✓`, `commit "…"
  ✓`). H4, textual. Fix: one wording, on every undoable outcome or on none.
- **m6. `?` in J labels `⏎` "Revert / put back (preview)", but on the `← u`/`← U` row `⏎` runs at once.** Label:
  `crates/graft-ui/src/keymap/table.rs:471`. Observed: `⏎` on `undo: checkout main ← U` ran the redo with no preview
  (`redo: checkout main ✓`). That matches the bar (`⏎ redo this`) and the design (`crates/graft/src/update/journal/keys.rs:320-324`);
  only the help label is wrong. Fix: "Undo / redo / revert to before this (preview when older)".
- **m7. The man page's NAME line has no product name.** `docs/man/mwr.1:3`: `mwr \- a terminal Git client for the
  daily work`. `man -k mergewright` and `apropos` read this line, so they won't find it. The pass-3 Need
  (`pass-3-mergewright.md:13-15`) asks for the name to be "the same everywhere he sees it: command, man page, brew".
  Fix: `mwr \- Mergewright, a terminal Git client …`.
- **m8. Recent repos with the same folder name read the same.** On the empty state at 120 columns, two `api` repos
  in different parents both read `api /private/…/scratchp…`. The line is cut from the end
  (`crates/graft-ui/src/empty_state/draw.rs:116-124` (`fit` at :124)), so the part that tells them apart goes first, and the age
  column with it. UX §3.6. In his use, paths are likely shorter; worth it for a monorepo plus clones. Fix: cut the
  path from the left (`…/w1/api`) and keep the age.
- **m9. J's legend drops `⇢` when it is on screen and keeps `✗` when it is not.** At 120 columns a `⇢ replaced` row is
  shown and the legend ends `✗ a commit no branch holds` with no `⇢` clause. `legend_at` adds the clause only when
  everything fits (`crates/graft-ui/src/journal_screen/legend.rs:37-45`). At 80 columns the row's own tail is cut to
  `undo brings…` (observed). Fix: at narrow widths drop `· was X's tip` before `undo brings it back`
  (`crates/graft/src/view/journal.rs:275`), and let the legend explain the glyphs that are on screen.
- **m10. `b makes restored at d7943ab` reads as a broken sentence.** The branch name is unquoted
  (`crates/graft/src/update/journal/answers.rs:175`). Fix: `b makes branch "restored" at d7943ab`.
- **m11. The bar offers `u undo` in a repo with no journal; `u` answers `nothing to undo ✓`.** Observed on a repo the
  app had never written to. The hint is unconditional (`crates/graft-ui/src/graph_screen/hints/lists/rows.rs:18`).
  That contradicts the promise at `crates/graft-ui/src/graph_screen/hints/lists.rs:129-133` ("the bar never hints a
  row the key refuses"). Low: only a repo's first session.

## Questions

- **Q1. The redo side of Q1.** After `u` on a rebase, J reads `└ ✗ lost fd798dc "feature b on a" · was feature-b's tip ·
  no branch holds it · +2 under it`, while `← U` on the same row puts them back (observed). This is by spec: commits
  an undo dropped stay `Lost` (`crates/graft-ops/src/journal_list/fate.rs:67`, ticket 46 Decisions). It is the same
  "reads as damage" he ruled on ("yes say replaced"). Does he want `⇢ … U brings it back` here too?
- **Q2. A rebase stop ends with `c`, a merge stop with `⏎ … y`.** In a merge, `⏎` on the last file opens the finish
  summary and `y` commits. In a rebase, `⏎` leaves `✓ every file done — c continue the rebase`. There, `y` changed
  nothing on screen and showed no message (observed; I did not find the line that drops it). Is the different ending
  key intended? He has pressed the wrong one before (`field-notes.md:63`).
- **Q3. The "another window" line needs `esc dismiss` and asks him to do nothing.** `rename: still using the old names ·
  another Mergewright window is open on this repository · esc dismiss` (`docs/shots/m8-mergewright-s3-busy-120x36.txt`,
  as the Lands table specifies). UX §3.1 says a status with no work in it should not need a dismiss key. Should it
  clear itself, or not show at all, since the move runs at the next launch with no window open?
- **Q4. Leftover text after starting the tour from the empty state (pty only).** After `t`, the empty state's `RECENT`
  (as `◌ECENT`), a recent repo's name and its path line stayed under the tour's graph. The leftovers were still there
  after more keys. I could not find a cause in code (`event_loop/row_clear.rs` clears row by row as it should), and
  pyte mis-parses the Kitty probe, so this may be the harness. Needs his eye: in Zed, start `t` from the empty state
  and look for leftover text.

## From the earlier walk (`m8-usability.md`)

- **Fixed (observed):** Major A (launch lands on HEAD with a stash newer than HEAD) · Major B (`?` at a rebase stop
  says `Take upstream`/`Take yours`) · Q1 (`⇢ replaced`) · Q2 (`⏎` on the current branch closes the picker).
- **Still standing (observed, not re-filed):**
  - **m2:** `?` in the resolver lists `c` (Continue / Finish), `e` and `M` twice each, at both merge and rebase stops.
  - **m6:** the empty state's `?` lists `u`/`U`/`⌃R` under EVERYWHERE.
  - **row 845:** a merge with one conflict takes 5 keys against the note's 4.

## Captures the user must judge by eye (rule 6; colour unverified, rule 5)

- `docs/shots/m8-q1-journal-replaced-187x30.txt`, `…-80x24.txt`, `…-ui-187x30.txt`, `…-ui-80x24.txt`: `⇢ replaced` vs
  `✗ lost` (glyph and word differ; muted vs warning ink unverified).
- `docs/shots/m8-q2-picker-current-row-ui-120x36.txt`, `docs/shots/m8-q2-picker-enter-current-120x36.txt`
- `docs/shots/m8-mergewright-journal-187x30.txt`, `…-80x24.txt`, `docs/shots/m8-mergewright-s1-graph-120x36.txt`,
  `…-s1-journal-120x36.txt`, `…-s3-busy-120x36.txt`
- No capture exists of J showing `running` (58r). Its look is unjudged.
