# Conflict resolution: how the best-loved tools do it, and Graft's M6 against them

Researcher, 2026-09-23. Standalone research, no source code edited.

**How to read this.** Every claim about another tool carries a URL. I have not run any of these tools. The step counts
come from their docs, and where the docs leave a step out I say so. Everything I could not confirm is marked **(?)**.
Claims about Graft carry a repo `file:line`. Sites that refused or 404'd are listed at the end.

**Principle used for the recommendations:** use the fewest steps that are still enough. One step at any cost is wrong,
and so is 5–6 steps where 2–3 do the job.

---

## 1. The tools

### 1.1 JetBrains IDEs (IntelliJ and the rest): the "Conflicts" dialog, then the 3-way merge dialog

Source: https://www.jetbrains.com/help/idea/resolve-conflicts.html

- **Where the user lands:** a dialog, *Files Merged with Conflicts*, opens by itself when a merge, rebase, pull,
  cherry-pick, unstash or patch hits conflicts. Each file has three choices: **Accept Yours**, **Accept Theirs** and
  **Merge…** (resolve by hand). If you close the dialog, a *Merge Conflicts* node stays in the Commit tool window, with
  *Resolve Conflicts* and *Abort Merge* buttons.
- **Journey:**
  - To the first conflict on screen: 0 steps to the file list, then 1 to open the file (Merge… or double-click).
  - Whole-file take: 1 click (Accept Yours/Theirs), which also marks the file resolved.
  - Per conflict: 1 click (the arrow `>>`/`<<` to accept a side, or `X` to ignore it). There is also *Resolve simple
    conflicts* and *Apply Non-Conflicting Changes* (the "magic wand").
  - Mark the file resolved: 1 click (*Apply* moves on to the next file).
  - Commit the merge: the Commit dialog, ~2 steps **(?)** because the docs do not count them.
  - **Total for 1 file with 1 conflict: about 4–5 clicks.**
- **Layout:** three panes. The left pane is the read-only local copy. The centre pane is the result, a full editor, and
  **starts as the base revision**. The right pane is the read-only incoming copy. The page does not describe line
  numbers per pane **(?)**. Binary and deleted-by-one-side files are not covered on that page **(?)**.
- **Undo / start over:** a *Revert conflict resolution* option puts the file back in conflict. The gutter toolbar
  reverts one change at a time.
- **Loved:** "the best, IMHO, because it helps you edit the resolved result, while updating both left and right diffs".
  People also praise the magic wand and having full IDE features while merging.
  https://news.ycombinator.com/item?id=31105227 . Other tools copy it: the lazygit 3-pane request names JetBrains as its
  model (https://github.com/jesseduffield/lazygit/issues/2133).
- **Complaints:** "If both sides change a line, accepting them both will lead to duplicating that line"
  (same HN thread).

### 1.2 VS Code: inline CodeLens (the default) and the 3-way merge editor (opt-in)

Source: https://code.visualstudio.com/docs/sourcecontrol/merge-conflicts

- **Where the user lands:** the docs page does not say which view gets focus when a merge stops **(?)**. Conflicted
  files are listed in Source Control; the name of that group is not confirmed here **(?)**.
- **Inline journey:**
  - Open the file: 1 click.
  - Per conflict: 1 click on the CodeLens above it: *Accept Current / Accept Incoming / Accept Both / Compare Changes*.
  - Save (whether this is needed depends on auto-save **(?)**), stage (+): 1 click, commit: 1 step.
  - **Total for 1 file with 1 conflict: about 4–5.**
- **Merge-editor journey:** Incoming and Current panes on top, **Result** at the bottom. You can edit the result
  directly. Base is shown through a *More Actions* toggle. **Complete Merge** "stages that file and closes the merge
  editor". There is an unresolved-conflict counter, and an experimental *Resolve Merge Conflict with AI* button.
- **The most useful piece of evidence in this whole study:** VS Code made the 3-way merge editor the default, then put
  the inline view back as the default. Their reasons: "we underestimated how good the previous inline conflict
  experience is"; "By default we should be simple"; "Git conflicts are complicated, to reduce total complication for
  users the default experience should be the simpler one"; "the large majority of the GitHub, Twitter feedback we got
  strongly prefers the previous inline conflict experience".
  https://github.com/microsoft/vscode/issues/160806
- **Complaints about the 3-pane editor:** "really difficult to use. Harder for me to see what changes there are in the
  side by side view, and not clear how I'm supposed to accept changes"
  (https://github.com/microsoft/vscode/issues/157610). Another user was confused by the three panes
  (https://github.com/microsoft/vscode/issues/159192).

### 1.3 Sublime Merge

Source: https://www.sublimemerge.com/docs/getting_started (the page `docs/merge_tool` returned 404)

- **Where the user lands:** conflicted files appear under **Pending changes**, and each unmerged file header has a
  **Resolve** button.
- **Journey:**
  - To the conflict: 1 click (Resolve).
  - Per conflict: 1 click on `▶` or `◀` next to the hunk.
  - **Save and stage**: 1 click, which marks the file resolved.
  - Commit: ~1–2 **(?)**.
  - **Total: about 4.**
- **Layout:** three columns: Ours on the left, Merged in the middle, Theirs on the right. "Base can also be selected".
  Conflicts carry a **CONFLICT** marker.
- **Complaints:** people ask for a single take-theirs or take-mine for the whole file, and for branch names so they know
  which side is which (https://github.com/sublimehq/sublime_merge/issues/316, closed; whether it was ever implemented is
  unknown **(?)**). The same request: https://forum.sublimetext.com/t/feature-request-let-me-pick-theirs-or-ours-without-opening-a-diff/47258 .
  Merge-tool keyboard shortcuts changed in 2.0 (https://forum.sublimetext.com/t/where-did-keyboard-shortcuts-go/51761).
  I did not read that thread's resolution **(?)**.

### 1.4 Araxis Merge

Source: https://www.araxis.com/merge/documentation-windows/three-way-file-comparison-and-merging.en (from search; the
macOS page 404'd)

- It is a standalone merge tool, launched through `git mergetool`. It has no "merge stopped" moment of its own.
- **Layout:** three panes with the **common ancestor in the centre**. The derived files sit either side, and changes
  are merged into the centre. Automatic merge applies simple changes; afterwards "only lines within conflicting changes
  will retain their original colours".
- Step counts, line numbers, undo and sentiment: not confirmed **(?)**.

### 1.5 lazygit (the nearest peer: a keyboard TUI)

Sources: https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md ; source code as
below.

- **Where the user lands:** a menu titled **"Conflicts!"** with *View conflicts* (which pushes focus to the Files
  panel) and *Abort* (`a`).
  `pkg/gui/controllers/helpers/merge_and_rebase_helper.go` `PromptForConflictHandling`, master:
  https://raw.githubusercontent.com/jesseduffield/lazygit/master/pkg/gui/controllers/helpers/merge_and_rebase_helper.go .
  Whether the conflicted file is already selected in the Files panel is not confirmed **(?)**.
- **Journey:**
  - To the conflict: `Enter` on *View conflicts*, then `Enter` on the file. That is 2 keys, plus `j` presses if the
    file is not selected.
  - Per conflict: `space` picks the hunk under the cursor, `b` picks both, `↑/↓` moves between hunks and `←/→` between
    conflicts.
  - Resolved: with `git.autoStageResolvedConflicts` (on by default), the file is staged automatically, and lazygit
    "will also ask you if you want to continue a merge or rebase" (https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md).
    The prompt reads "All merge conflicts resolved. Continue the %s?" (`pkg/i18n/english.go` line 1521, master).
  - Commit: `Enter` on that prompt.
  - **Total for 1 file with 1 conflict: `Enter Enter space Enter` = 4 keys.**
- **Layout:** inline. The file is shown with its markers, and you pick hunks within it. Base is not described **(?)**.
  Line numbers are not confirmed **(?)**.
- **Whole-file cases:** the Files-panel `M` menu is titled "Resolve merge conflicts". Modify/delete conflicts get
  *Keep file* / *Delete file*, together with a plain-English explanation per shape: "this file was modified in the
  current changes and deleted in incoming changes … The most likely resolution is …" (`english.go` lines 1229–1240).
- **Undo:** `z` = "Undo last merge conflict resolution" (keybindings doc).
- **Complaints:**
  - "Merge conflicts suck … not sure what you're looking at when the changes are stacked on top of each other" — a
    request for a JetBrains-style 3-pane view (https://github.com/jesseduffield/lazygit/issues/2133, closed).
  - No built-in take-ours/theirs for a whole file; the workaround is custom commands. A maintainer points out that
    `checkout --ours` is not the same as picking ours in every conflict block
    (https://github.com/jesseduffield/lazygit/discussions/1608).
  - Hundreds of conflicts in lockfiles (https://github.com/jesseduffield/lazygit/issues/2026).

### 1.6 Magit (smerge / ediff)

Sources: https://docs.magit.vc/magit/Resolving-Conflicts.html ; smerge keymap from
https://raw.githubusercontent.com/emacs-mirror/emacs/master/lisp/vc/smerge-mode.el

- **Where the user lands:** the Magit status buffer, which lists the unmerged file. Magit docs, as above.
- **Journey:**
  - From status: `k` (`magit-discard`) on a conflicted hunk. With point on one side, it "keeps it without prompting".
    That is 1 key per conflict, plus moving point there.
  - Or in the file: `C-c ^ m` / `C-c ^ o` keep upper/lower, `C-c ^ a` keeps all, `C-c ^ b` keeps base, `C-c ^ n` goes
    to the next conflict, and `C-c ^ RET` keeps the side under the cursor.
  - "Smerge automatically stages the file when you save the buffer after resolving the last conflict."
  - Commit: `c c` plus the message **(?)**.
  - **Total: about 4–5.**
- **Layout:** inline (smerge), or ediff: separate buffers for the versions, launched with `e`. Magit's own docs call
  smerge the simpler choice.
- **Undo:** Emacs undo in the buffer **(?)**; I did not check the Magit docs for this.
- Sentiment threads were not found in the time spent **(?)**.

### 1.7 Meld

Source: https://gnome.pages.gitlab.gnome.org/meld/ ("Three-way merge assistance with conflict handling and base
version display")

- It is a mergetool (git hands it the file). Three panes: LOCAL and REMOTE on the outside, MERGED or BASE in the
  centre, depending on how you call it (https://lukas.zapletalovi.com/posts/2012/three-way-git-merging-with-meld/ is a
  blog, not a primary source).
- **Complaint:** the LOCAL / BASE / REMOTE columns "are confusing / difficult to understand"
  (https://gitlab.gnome.org/GNOME/meld/-/work_items/937). I did not read the body **(?)**.
- Steps, line numbers and undo: not confirmed **(?)**.

### 1.8 KDiff3

Source: https://kdiff3.sourceforge.net/doc/merging.html (plus search summary of the same doc)

- **Layout:** input panes A (base), B and C, with an editable **output** window. An unsolved conflict shows "`?`" and
  the line "`<Merge Conflict>`", all in red.
- **Per conflict:** 1 click on A/B/C in the button bar. You can pick more than one source.
- **Auto-advance:** "Automatically go to next unsolved conflict after source selection". The docs say to turn it off
  before picking lines from several inputs or before editing afterwards.
- **Bulk:** choose A, B or C everywhere, or only for the remaining unsolved conflicts.
- **"Saving is disabled until all conflicts are resolved"**, and the prev/next *unsolved* conflict buttons take you to
  what is left.
- Line numbers: an option exists according to the handbook, but I did not read it on that page **(?)**.

### 1.9 GitKraken

Sources: https://gitkraken.com/features/merge-conflict-resolution-tool (marketing page; the help-centre page 404'd)

- **Where the user lands:** "Conflicted files appear in the Commit Panel"; clicking one launches the Merge Tool.
- **Layout:** current and target side by side, with a live output below. You pick lines or edit the output. Then
  "Save, mark resolved, and commit". There is also "Auto-resolve with AI".
- Step counts, base, undo and sentiment: not confirmed **(?)**. The "take all from A/B" wording came from a page summary
  that labelled it as an inference, so it is **not** used here.

### 1.10 Fork

Source: https://fork.dev/blog/posts/fork-1.0.73/ : a built-in merge resolver "rewritten from the ground up … Now the
end result can be edited manually". Layout, keys and steps: not confirmed **(?)**. Complaints: an open request to show
how each conflict was resolved in the file list (https://github.com/fork-dev/Tracker/issues/1304, title only read
**(?)**).

### 1.11 Tower

Source: https://www.git-tower.com/help/guides/branches-and-tags/merge-conflicts/mac

- **Where the user lands:** "a small warning sign icon" next to *Working Copy* in the sidebar, and "the commit area in
  the Working Copy view is replaced with information about the current status". You have to go there yourself.
- **Conflict Wizard:** "Version from <branch1>" and "Version from <branch2>" are shown on top, and **Resolve Using
  <branch>** takes a whole file (1 click). To combine the two versions, use **Open <Merge Tool>**, which launches an
  external tool configured in *Git Config*. Then mark resolved, then commit with a prefilled message, or *Abort*.
- **Journey (whole file):** open Working Copy (1), select the file (1), pick a version (1), resolve (1), commit (1–2).
  **Total: about 5–6.** Block-level work leaves the app.
- **Start over:** release notes: "You can now reset a merge conflict to its unmerged state, even after it has been
  resolved" (https://www.git-tower.com/release-notes, v4.1; which platform the version number refers to is **(?)**).
  Secondary guides say Tower now has a built-in 3-panel resolver. I did not confirm that on a Tower page **(?)**.

---

## 2. Graft today (M6)

### 2.1 The designed journey

| Moment | Keys | Source |
|---|---|---|
| Merge stops | Banner takes focus: "It takes focus the moment an operation stops" | `docs/design/02-keymap.md:260-262` |
| First conflict on screen | `⏎` on the banner opens the resolver | `02-keymap.md:266`; ADR-0042 (`docs/adr/0042-*.md:58-60`) |
| Per conflict | `o` / `t` / `b` (1 key); `n` to the next open block (1 key; a take does not auto-advance, see the walk's `n t n b`) | `02-keymap.md:242-247`; `TESTING.md:2133-2137` |
| File resolved | `⏎` stages the file and goes to the next conflicted file, or closes to the banner | `02-keymap.md:248` |
| Merge committed | `c` on the banner ("all conflicts resolved · c continue") | `TESTING.md` scenario 1, `:2140-2141` |

**Designed count for 1 file with 1 conflict: `⏎ o ⏎ c` = 4 keys.** That ties lazygit's
`Enter Enter space Enter` and beats the GUI tools (4–6 clicks). With k conflicts in a file it is `2k + 2`
(`o n o n …`), while an auto-advancing tool (KDiff3's option) needs `k + 3`.

**Whole-file shapes** (UD/DU/AU/UA/DD, binary): a card with `o` = keep and `t` = delete/theirs, and the key also marks
the file resolved. The capture caption reads `deleted by them · o keeps the file  t deletes it`
(`docs/shots/m6/m6-contact-sheet.txt:99`). Binary shows `binary · 22 B` (`:259`).

**Undo / start over:** a take can be changed while the resolver is open, because the card keeps its header
(`docs/design-notes/m6-conflicts.md`, "New UI" bullets). `d` starts the file over behind a confirm box, "not undoable
while the operation is open" (`02-keymap.md:246`; capture `m6-contact-sheet.txt:661`). `u`/`U` are refused while an
operation is live (`02-keymap.md:278-282`).

### 2.2 What the user actually hit (field finding, 2026-09-23)

- **Stuck after the stop:** he pressed `Esc`, moved to the uncommitted-changes row and opened it before he reached the
  conflicts. That is at least 3 extra steps plus navigation, so **~7+ steps** against the designed 4.
  - **Why is an open question.** I found no line proving the cause. Two candidates to check, **not facts**:
    - (a) The merge was begun **in the shell**. For that case the walk does not prove the banner path, and `c` refuses
      it (`TESTING.md:108`, `:2122`).
    - (b) Something else took the first key. In the relaunch capture, the commit-graph offer banner (`B build one … X
      never here`) is drawn **above** the operation line (`m6-contact-sheet.txt`, capture m6-06). Which of the two has
      focus at that moment is not proven by a capture.
  - The designer should reproduce his exact start (in the app or in the shell? a pull or a merge?) before fixing
    anything.

### 2.3 Layout, from the capture (`m6-contact-sheet.txt:22-45`, three.txt at 120x40)

- **Line numbers only on OURS.** In `3 one-ours │      one-theirs` the THEIRS half has blank padding where a number
  would be (`:28`, `:39`). Confirmed on the capture.
- **Stray number on the header:** `   2 ── conflict 1 of 3 · open ──` (`:26`). The number is the `<<<<<<<` marker's own
  line in the file on disk. The design says line numbers are "the file's current line numbers (`render()`)"
  (`m6-conflicts.md`, "Line numbers" bullet), and an unresolved block still has its markers on disk. **So is the `2`
  the design working as written, or a defect?** The number is correct by that rule. The rule is what is wrong for a
  reader.
- **Numbers jump across a card:** `3 one-ours` then `7 b1` (`:28`, `:31`). Lines 4–6 are the `=======`, the theirs line
  and the `>>>>>>>` on disk. From the capture alone, a reader sees skipped numbers with nothing to explain them.
- Taken block: the header reads `took theirs`, the offer shrinks to `o ours  b both`, and the taken line follows it
  (`:34-36`). The closing rule carries a `┴` with no column above it. The walk already raised this as F11, a question
  (`:19`).
- BASE on demand (`B`), stacked under the pair (`:29-30`). For zdiff3 markers no BASE is drawn, although the block
  holds one; the sheet raises this as a question (`:581`).
- Rebase explainer line: `ours = main · theirs = the branch being merged in` (`:25`). This is the confusion Sublime #316
  and Meld #937 users complain about, answered on screen.

---

## 3. Comparison and recommendations

T = take, R = reject, I = invent.

| Aspect | Best-in-class answer | Graft today | Rec | Why |
|---|---|---|---|---|
| Landing when the op stops | JetBrains: the conflicts dialog opens by itself; lazygit: the "Conflicts!" menu, `Enter` → files | Banner focused, `⏎` opens resolver (design); **user was stuck** (field) | **T** (keep the design, fix the path) | 1 key from stop to conflict is right; the real path failed him. Reproduce his exact start first. The cause is unknown, see 2.2 |
| Whole-file take for a text file | JetBrains Accept Yours/Theirs; Tower Resolve Using; requested of Sublime (#316) and lazygit (#1608) | Only per block; whole-file keys only on UD/binary cards | **T** as "take ours/theirs in **every remaining block**", not `checkout --ours` | Most-requested gap in two peers. The lazygit maintainer's caveat says what it must mean. Needs a key: a spec edit via the conductor |
| Advance after a take | KDiff3 auto-advance to the next unsolved conflict (optional) | Stays; `n` needed | **T** | Saves k−1 keys per file. `b` already covers combining, so KDiff3's reason to turn it off does not apply |
| Line numbers | Each pane numbered in its own file (JetBrains/KDiff3; not confirmed per tool **(?)**) | OURS only, numbered by the on-disk file with markers | **I** | Number each side by its own stage file (`:2:`/`:3:`), or number plain lines only and none inside cards; never the marker's line |
| Header number `2` | None of the tools number a marker | Marker line number on the card header | **I** (drop it) | It shows a line the reader never sees |
| Pane count | VS Code went back to **inline** as the default after the 3-pane editor (#160806) | Inline document, sides paired inside the card (ADR-0042) | **R** 3 columns | The strongest outside evidence here backs ADR-0042 |
| Base | Toggle (VS Code, Sublime); centre (Araxis, JetBrains' start state) | `B` toggle; missing for zdiff3 (question) | **T** as built; answer the zdiff3 question | On demand is the common answer |
| Which side is which | Branch names asked for (Sublime #316); columns confuse (Meld #937) | Git's labels plus an explainer line | Keep (already ahead) | Answers a documented complaint |
| Whole-file shapes explained | lazygit: plain-English per-shape text plus Keep/Delete | `deleted by them · o keeps the file  t deletes it` | Keep | Same idea, shorter |
| Undo a take | lazygit `z`; JetBrains revert; Tower reset to unmerged even after resolved | Re-take while open; `d` start over (not undoable in-op) | **T** Tower's "reset after resolved" (the design note says start-over after `⏎` is ops-only, `TESTING.md:2120-2121`, `:2178`) | Lets a mistaken `⏎` be walked back without the shell |
| Mark resolved | Save and stage (Sublime), Complete Merge (VS Code), auto-stage (lazygit, smerge) | `⏎` = stage + next file; refused while blocks are open | Keep | 1 key doing two jobs is already as short as auto-stage |
| Finish | lazygit prompts "Continue the merge?" when the last file resolves | Closes to the banner: `all conflicts resolved · c continue` | Keep; **fix shell-begun merges** (M7 row) | Same count; the shell-begun refusal is the gap |
| Save gate | KDiff3 disables save until every conflict is solved | `⏎` refused: `2 conflicts still open · n next` | Keep | Same safety, and it names the way out |
| Auto-apply non-conflicting / AI | JetBrains magic wand; VS Code/GitKraken AI | None | **R** | Git has already merged the non-conflicting parts. AI is outside the brief |

**Step count, 1 file with 1 conflict:** Graft as designed = **4** (`⏎ o ⏎ c`), tied with lazygit (4) and ahead of
JetBrains/VS Code/Sublime (~4–5 clicks, some steps **(?)**) and Tower (~5–6). **Graft as the user met it: ~7+.**
With auto-advance, a 3-conflict file goes from 8 to 6.

---

## 4. Could not confirm / access notes

- 404 or refused: `sublimemerge.com/docs/merge_tool`, `magit.vc/manual/...` (the docs.magit.vc copy was used instead),
  `gnu.org` (connection refused), `docs.kde.org` KDiff3 index, Araxis macOS page, GitKraken help centre, Tower
  `solve-conflicts` guide.
- Not confirmed for any tool: where keyboard focus lands in VS Code; line numbers per pane in JetBrains, Meld and
  Araxis; commit-step counts in the GUIs. All the counts above are **read from docs, not measured**.
- Graft: **why** the user was stuck is not established (2.2). No source was read for this; only design docs and
  captures.
