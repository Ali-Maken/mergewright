# Conflict visuals and the finish step: prior art for the three-pane resolver

Pipeline step 2 for M6 follow-up. It adds to `docs/research/conflict-resolution.md`, which covered journeys and layout but
not colour, connecting lines or how "finish" is presented.

**The need** (`docs/field-notes.md:60-61`, 2026-09-24, Zed terminal, 187 columns):
- "there was no color and lines like there on other tools on market." The only styling in the design is reverse video for
  focus (`docs/design-notes/m6-conflicts.md:387`).
- After the file was done he did not know to press `c`, and the finish screen did not read as a preview, so the `⏎` that
  committed surprised him.

Sources are primary (vendor docs, source code, issue trackers) unless marked. `(?)` = not confirmed. Access failures are
listed in §5.

---

## 1. Colour in three-way conflict views

### 1.1 Per tool

| Tool | Yours / theirs | Base | Conflict | Resolved vs open | Source |
|---|---|---|---|---|---|
| **VS Code merge editor** | Current = teal `#40C8AE`, incoming = blue `#40A6FF`. Header at 50% alpha, content at 40% | grey `#606060` at 40% | `conflictingLines.background` yellow `#ffea0047` | **Border by state:** unresolved = orange `#ffa600` (dimmer `#ffa6007a` when not focused); resolved = grey `#c1c1c1cc` (focused), `#86868649` (not focused). Overview ruler: unresolved `#fcba03`, resolved `#adaca8ee` | `src/vs/platform/theme/common/colors/editorColors.ts:389-391`; `src/vs/workbench/contrib/mergeEditor/browser/view/colors.ts:33-84` (raw GitHub, main); key list https://code.visualstudio.com/api/references/theme-color |
| **JetBrains** | Diff viewer: "Green - Added, Blue - Modified, Gray - Deleted" | not stated | salmon `ffd5cc` / dark `45302b`; yellow for deuteranopia/protanopia (confirmed, §2.3) | Docs: "If there are no remaining conflicts, the Conflicts dialog enables the Accept and Finish option." Whether a resolved block's colour changes `(?)` | https://www.jetbrains.com/help/idea/differences-viewer.html ; https://www.jetbrains.com/help/idea/resolve-conflicts.html |
| **Beyond Compare** | "Changes on the left are marked in teal, and changes on the right are marked in magenta" | Centre pane | "Overlapping changes are considered conflicts and are marked in red" | Editing the output by hand "will change the section to yellow" | https://www.scootersoftware.com/v4help/using_text_merge.html |
| **KDiff3** | Per input A/B/C (hex values not read `(?)`) | A | `<Merge Conflict>` "all in red", with a red `?` in the summary column | The summary column shows which input each line came from: `A`/`B`/`C`, `m` for a hand edit, `?` for a conflict. **A letter tells you the state, not only the colour.** "Saving is disabled until all conflicts are resolved" | https://kdiff3.sourceforge.net/doc/merging.html |
| **Araxis Merge** | not stated | Centre pane | "a red conflict icon"; after an automatic merge "only lines within conflicting changes will retain their original colours" | Resolved blocks lose their colour | https://www.araxis.com/merge/documentation-windows/three-way-file-comparison-and-merging.en |
| **Meld** | "Each colored block represents a section that is inserted, deleted, changed or in conflict" | — | own colour (hex values not read; the style-scheme file returned 503) `(?)` | `(?)` | https://meldmerge.org/help/file-mode.html |
| **Sublime Merge** | Docs name the columns only (Ours / Merged / Theirs) and a CONFLICT marker. Colours `(?)` | selectable | `(?)` | `(?)` | https://www.sublimemerge.com/docs/getting_started |
| **Kaleidoscope** (user asked, 2026-09-24) | "Choose A" / "Choose B", with the result in the middle. Docs describe no colours `(?)` | toggles "Base to A" / "Base to B" (5.2) | `(?)` | Git finish: "Finish up and confirm your changes in the Terminal" | https://kaleidoscope.app/help/docs/text-merging ; https://blog.kaleidoscope.app/2024/10/31/kaleidoscope-5-2-eases-the-process-of-resolving-merge-conflicts-by-offering-base-to-a-b-comparisons-and-more/ |

**The pattern across tools that do state colours:** each side gets **one hue of its own** (VS Code teal/blue, BC
teal/magenta), **conflict gets a warm alarm colour** (red in BC, KDiff3 and Araxis; orange/yellow in VS Code), and a
**resolved block loses its alarm** (Araxis drops the colours, VS Code turns the border grey). None of the documented
tools uses red/green for yours/theirs.

### 1.2 Terminal peers

| Tool | What it colours | Source |
|---|---|---|
| **lazygit** | Only the marker lines `<<<<<<< ======= >>>>>>>`, in `style.FgRed`. Everything else is the default text colour, and ours/theirs are not told apart by colour. How the selected hunk is highlighted is not in this file `(?)` | `pkg/gui/mergeconflicts/rendering.go:18-22` (master) |
| **Neovim diff mode** (and diffview.nvim, which builds on it) | Background colour on whole lines: `DiffAdd` green, `DiffChange` grey, `DiffText` cyan, `DiffDelete` red foreground, bold. The dark and light variants are defined separately. The 16-colour fallbacks are `ctermbg=2/10`, `6/14` | `src/nvim/highlight_group.c:373-376` (light), `:457-460` (dark); `runtime/doc/diff.txt` "The diffs are highlighted with these groups" |
| **Emacs ediff** | The current difference has a background per buffer: A `#553333` dark / `#ffdddd` light (red tint), B `#335533` / `#ddffdd` (green tint), C (merge result) `#888833` / `#ffffaa` (yellow). Faces fall back to **`:inverse-video t` on terminals with no colour** | `lisp/vc/ediff-init.el:772-824` (emacs-mirror, master) |
| **delta** | Merge conflicts are drawn as two diffs (ours and theirs against the base), each under a header with `box` decoration. The region begins with `▼` and ends with `▲` | `src/cli.rs` options `merge-conflict-begin-symbol` (default `▼`), `merge-conflict-end-symbol` (`▲`), `merge-conflict-{ours,theirs}-diff-header-decoration-style` (default `box`) |

### 1.3 Dark, light, colour-blind

- **Tint the background; leave the text in the theme's own foreground.** VS Code requires merge colours to be
  translucent: "The color must not be opaque so as not to hide underlying decorations"
  (https://code.visualstudio.com/api/references/theme-color). ediff and Neovim ship separate dark and light values
  (see above). In a terminal, the **16 ANSI colours** follow the user's theme, so they adapt to dark and light on their
  own. Truecolor hex values do not.
- **Complaint, JetBrains:** people say conflict colours are hard to see in dark themes, and that Diff & Merge settings
  cannot set a foreground colour. Seen only in search snippets of
  https://intellij-support.jetbrains.com/hc/en-us/community/posts/115000192964 and `/360002336559`; both pages 403 → `(?)`.
- **Colour-blind users:** red/green diffs are reported as "impossible for color-blind users" in Gerrit
  (https://phabricator.wikimedia.org/T232893), GitLab (https://gitlab.com/gitlab-org/gitlab/-/issues/18818) and Gitea
  (https://github.com/go-gitea/gitea/issues/25680). The proposed fixes are blue/yellow and configurable colours. **None
  of these pages was fetched**, so the quotes come from search snippets `(?)`. Teal-vs-magenta (BC) and teal-vs-blue
  (VS Code) avoid the red/green axis, but VS Code's two hues are close to each other; whether they stay distinguishable
  is a question.
- **Redundant non-colour cue:** KDiff3's `A/B/C/m/?` column and ediff's inverse-video fallback carry the state without
  colour. This also matters for Graft's rule 5: captures cannot prove colour, but they **can** prove a glyph.

---

## 2. Connecting lines between panes

### 2.1 GUI tools

| Tool | What it draws | How a taken side shows | Source |
|---|---|---|---|
| **Araxis** | "linking lines connect the changes from the derived files to the same section of the ancestor file". Overlap is recognisable because both sides' lines land on one ancestor section | resolved blocks lose their colour (§1.1) | Araxis 3-way page |
| **Meld** | Between each pair of panes, "a segment that shows how the changed sections between your files correspond to each other". The segments are filled bands | "click on the arrows in a segment to replace sections in one file with sections from the other" | https://meldmerge.org/help/file-mode.html |
| **JetBrains** | **Confirmed (source):** a filled trapezium between the editors, "which indicates the change flow from one editor to another", drawn curved by default (`paintPolygons(..., curved=true, ...)`). Its colour is the block's change type (§2.3) | **Confirmed:** once resolved, the band **loses its fill** and becomes a **dotted outline** in the same colour, and it is no longer always visible | `intellij-community` `platform/diff-impl/src/com/intellij/diff/util/DiffDividerDrawUtil.java:65-83, 345-373, 439-491` (master) |
| **VS Code merge editor** | **Confirmed (source): no bands between panes.** Rows are aligned with spacer view zones (`view/viewZones.ts:192` `class Spacer`, `view/lineAlignment.ts:16`). Each block gets a **gutter bracket**, a left-open box 10 px wide for the block's height, with an accept checkbox in it | **Confirmed:** the bracket and block border go orange → grey when handled. The checkbox shows `check` (first side taken) / `checkAll` (both) | `mergeEditor/browser/view/media/mergeEditor.css:98-111, 128-181`; `view/editors/inputCodeEditorView.ts:420-447` (main) |
| **Sublime Merge** | `▶` / `◀` buttons beside the hunk (`conflict-resolution.md:77`) | `(?)` | getting_started |

### 2.3 JetBrains colours (confirmed, source)

JetBrains colours a band and a block by **change type, not by side**: `INSERTED`, `DELETED`, `MODIFIED`, `CONFLICT`
(`platform/diff-impl/src/com/intellij/diff/util/TextDiffTypeFactory.java:23-33`). The default backgrounds come from
`platform/platform-resources/src/DefaultColorSchemesManager.xml` (master):

| Type | Light (Default, :489-520) | Dark (Darcula, :2255-2276) | Colour-blind variant |
|---|---|---|---|
| Conflict | `ffd5cc` salmon | `45302b` | `deuteranopia`/`protanopia` = `ffec8c` / `453900` (yellow) |
| Inserted | `BEE6BE` green | `294436` | dark `29452e` |
| Modified | `CAD9FA` blue | `385570` | same |
| Deleted | `D6D6D6` grey | `484a4a` | — |

**The shipped schemes carry explicit deuteranopia/protanopia overrides: conflict turns from red to yellow.** Sublime Merge's
side colours are still `(?)`. Its app is closed source, and https://www.sublimemerge.com/docs/themes (search result only)
says only that diff colours come from colour schemes.

### 2.2 What a terminal can do, cell by cell

The terminal peers:
- **None of them draws curves.**
- **Neovim diff mode aligns rows with filler lines instead of drawing links**: "Vim will show filler lines for lines that
  are missing in one window but are present in another" (`runtime/doc/diff.txt`, "filler").
- **delta** uses boxed headers and `▼`/`▲` region markers (§1.2).
- **ediff** relies on background colour only.

Graft already aligns the three panes row for row by git's regions (ADR-0043 §2, note :431). **Because rows line up, the
"link" does not have to be a curve.** It can be a gutter column, one or two cells wide, between each pair of panes.
Possible cells (invented; no peer found doing this):

```
 yours · main          │   result             │   theirs · feature
 12  let x = 1;       ━┫━  12  let x = 1;    ┣━   12  let x = 2;     ← block taken from yours: bar joins left→middle
 13  ...              ┏┛   ...                ┗┓  ...                 ← open conflict: brackets from both sides
```

- **Bracket** a block's rows in the gutter: `┌ │ └`, or heavy `┏ ┃ ┗` for the focused block. Needs 1 cell.
- **Taken side:** a joining bar (`━━▶` or `─►`) from the chosen pane into the middle, with the other side's gutter
  blank.
- **Open conflict:** both sides bracketed and a `?` in the result gutter, as in KDiff3's `?`.
- **Background tint** on the block's rows, full pane width (ediff `:extend t`, Neovim `DiffAdd`), in ANSI colours so the
  theme decides the actual shade.

Box-drawing glyphs are in the Unicode tier that the user lives in (memory: Unicode tier is the daily view), and **they
show in captures**. Colour does not (rule 5).

---

## 3. The finish step

| Tool | "All resolved" signal | Finish action | Reads as preview? | Steps after the last block |
|---|---|---|---|---|
| **JetBrains** | "the Conflicts dialog enables the **Accept and Finish** option" | Apply (per file) → Accept and Finish → Commit dialog | Yes. The Commit dialog shows the file list and message before you commit, with Commit as the final button `(?: layout of that dialog not read here)` | ~3 (Apply, Accept and Finish, Commit) `(?)` |
| **VS Code** | unresolved counter; orange borders turn grey | **Complete Merge** "stages that file and closes the merge editor. It does not finish the entire repository operation." Then commit in Source Control with a message | The commit box is an ordinary commit. Not a preview | 2–3 (Complete Merge, commit, maybe message) |
| **Sublime Merge** | `(?)` | "Save and stage", then commit | normal commit view | ~2 `(?)` |
| **KDiff3** | Save stays disabled until the unsolved count is 0; "Go to prev/next unsolved conflict" | Save, then the caller (git) continues | — | 1 + git |
| **Kaleidoscope** | `(?)` | "Finish up and confirm your changes in the Terminal" | No: git does it | 1 + shell |
| **git itself** | `git status`: "All conflicts fixed but you are still merging." + "(use \"git commit\" to conclude merge)" `(?: second string not grepped)` | `git commit` opens the editor with the merge message, which the user reads and saves | **Yes.** The editor is the preview, and quitting it without saving aborts | 1 + save |
| **lazygit** | "Continue the merge?" prompt (`conflict-resolution.md` §1.5) | 1 key | bare confirm | 1 |

`wt-status.c:1276,1282` (git master) prints "fix conflicts and run \"git commit\"" and "All conflicts fixed but you are
still merging."

**Complaints:**
- VS Code renamed "Accept Merge" because "you aren't really accepting a merge so much as the individual changes". The
  word on the finish button matters (https://github.com/microsoft/vscode/issues/162043).
- VS Code: "Click Complete Merge and it will say there are still unresolved conflicts" when the state and the button
  disagreed (https://github.com/microsoft/vscode/issues/197391).
- VS Code: the Complete Merge button was hidden by overlapping bars (https://github.com/microsoft/vscode/issues/184567).
- VS Code: Complete Merge "is not actionable via keyboard" (https://github.com/microsoft/vscode/issues/183244).
- None of these was read beyond its summary.

**The pattern:** where the finish is discoverable, it is a **labelled control that appears or enables when the count
reaches 0** (JetBrains' Accept and Finish, KDiff3's Save becoming enabled). Where it reads as a preview, the last screen
is the **commit message editor with the file list**, and it has its own "Commit" verb (JetBrains, git's editor).

**Graft today:**
- The signal is a status-line hint `every file done ✓ · c finishes the merge` (note :449).
- The summary is confirmed with `⏎` (note :421, :459).
- Elsewhere, `y` confirms every operation preview (field-notes.md:58). Whether the finish summary deliberately uses `⏎`
  and not `y` is a **question** for the conductor.

---

## 4. What this suggests for a terminal three-pane resolver

| Aspect | Rec | Why |
|---|---|---|
| One hue per side, background tint over the whole row, in ANSI palette colours | **Take** (VS Code, BC, ediff, Neovim) | Every documented tool does it. ANSI colours follow his dark or light theme. Avoid red/green for yours/theirs (colour-blind complaints §1.3); e.g. cyan vs magenta, as BC does. Ships **unverified** (rule 5) |
| Conflict = warm alarm, resolved = alarm removed | **Take** (Araxis, VS Code borders, BC red) | It shows open vs done at a glance (heuristic 1, ux-principles.md:51) |
| A non-colour state glyph per block (`?` open, `◀`/`▶` or `Y`/`T` taken, `m` edited) | **Take** (KDiff3 column) | It works for colour-blind users, and captures can verify it where they cannot verify colour |
| Curved ribbons | **Reject** | A character grid cannot draw curves, and no terminal peer tries |
| Gutter brackets per block (orange while open, grey once done) | **Take** (VS Code, confirmed §2.1) | |
| A joining bar toward the middle for a taken side; open = solid, taken = dotted or thin | **Take the idea / invent the cells** (JetBrains' band goes from filled to a dotted outline, §2.1) | Rows already align (ADR-0043 §2), so a 1–2-cell gutter carries the "link" |
| Finish signal: a labelled row in the resolver when the count hits 0, e.g. `all 2 files done — c  review & finish` | **Take** (JetBrains' Accept and Finish appearing) | A status-line hint alone was missed (field-notes.md:60). Recognition over recall (heuristic 6) |
| Finish screen reads as a preview: a title such as `Review merge — nothing committed yet`, and the commit key labelled with its verb | **Take** (git's editor, JetBrains' Commit dialog) | He did not read it as a preview (field-notes.md:60). Whether the confirm key should be `y` to match other previews (field-notes.md:58): **question** |
| Bare yes/no confirm | **Reject** (lazygit) | Already rejected in note :422, and nothing found reverses that |

---

## 5. Could not confirm / access notes

- 404: jetbrains `settings-editor-color-scheme-vcs.html`, sublimemerge `docs/merge_tool`, kde docs `stable5 .../merging.html`
  (the sourceforge copy was used), scootersoftware `v5help/text_merge_intro.html` (the v4 page was used).
- 503: Meld style scheme on gitlab.gnome.org.
- 403: intellij-support forum posts. Their content came from search snippets only.
- Not fetched: the Gerrit/GitLab/Gitea colour-blind issues (search snippets only).
- Colours not documented: Sublime Merge, Kaleidoscope, JetBrains conflict/resolved colours, Meld hex values.
- Bands between panes: now **confirmed from source** (§2.1, §2.3). JetBrains draws curved bands, which become dotted
  outlines once resolved. VS Code draws no bands, only gutter brackets. No JetBrains help page or screenshot alt-text was
  used; the source answered it.
- Sublime Merge side colours: still not confirmed.
- lazygit selected-hunk styling: not in `rendering.go`. Where it lives is not traced.
