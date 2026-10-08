# Prior art: M8 look polish (tickets 17, 17b, 17c)

> **POST HOC.** Written 2026-10-08, after 17/17b/17c shipped, to close review Major M1
> (`docs/reviews/m8-stop-and-look.md:54-66`). Pipeline step 2 was skipped for these features; this file
> records what prior art says now. It did not inform the decisions it is compared against.

Scope and limits:
- Sources: official docs, source files, issue trackers. Where only a search snippet was read, it says so.
- GitHub's API rate limit was hit mid-pass; several issue searches returned nothing usable. User praise and
  complaint coverage is **thin**: only the issues listed were seen. Not read: Sublime Merge, GitUp, Magit, tig,
  gitk, GitLens docs for these aspects. Each is an open question below, not a finding.

## 1. Folded/collapsed commit counts (`+N`)

- **jj**: commits outside the shown revset draw as a `~` node (`builtin_log_node`, `if(!self, label("elided", "~"))`,
  https://github.com/jj-vcs/jj/blob/main/cli/src/config/templates.toml). With `ui.log-synthetic-elided-nodes`
  a synthetic row reads `(elided revisions)` (https://github.com/jj-vcs/jj/blob/main/cli/src/commands/log.rs,
  lines ~61, 202, 338). **No count.**
- **jj users ask for the count**: open FR "Estimate the number of elided commits" (#3099,
  https://github.com/jj-vcs/jj/issues/3099); also "make excluded commits more compact" (#5937,
  https://github.com/jj-vcs/jj/issues/5937). Titles only read (via `gh search`), not bodies.
- **JetBrains**: "Collapse Linear Branches … a dotted line is shown instead of successive commits"
  (https://www.jetbrains.com/help/idea/log-tab.html). The doc mentions no count.
- Open: how Sublime Merge, GitKraken, lazygit mark a collapsed merge (not read).

## 2. Fading less relevant rows (hue fade)

- **VS Code Git Graph** (mhutchie): `git-graph.repository.commits.mute.commitsThatAreNotAncestorsOfHead`
  (default false) "Display commits that aren't ancestors of the checked-out branch … with a muted text colour";
  `…mute.mergeCommits` (default **true**) (https://github.com/mhutchie/vscode-git-graph/blob/develop/package.json,
  ~:1001-1010). Mutes **text colour**, not lanes.
- **JetBrains**: "Non-Picked Commits: shown greyed out"; current branch blue background; my commits bold
  (https://www.jetbrains.com/help/idea/log-tab.html).
- Open: user complaints about muted rows being unreadable in some themes (not found; not searched deeply).

## 3. Whitespace in diffs/blame

Note: what shipped is an **ignore-whitespace mode indicator**, not trailing-whitespace highlighting. Both covered.
- **delta**: `whitespace-error-style`, "Defaults to color.diff.whitespace … or else 'magenta reverse'"
  (https://github.com/dandavison/delta/blob/main/src/cli.rs ~:901-903). Highlights whitespace errors; no mode tail.
- **VS Code**: `diffEditor.ignoreTrimWhitespace` "ignores changes in leading or trailing whitespace"
  (https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/config/editorConfigurationSchema.ts:241-244);
  editor `renderWhitespace` includes `'trailing'`
  (https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/config/editorOptions.ts:749). Default value
  and whether the diff toolbar shows the state persistently: **not confirmed**.
- **lazygit**: `git.ignoreWhitespaceInDiffView`, toggled `<ctrl+w>`, uses `--ignore-all-space`
  (https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md ~:505-509). PR #1340 added **toast**
  messages "Showing/Ignoring whitespace in diff view" (https://github.com/jesseduffield/lazygit/pull/1340 — search
  snippet only). Toast is transient; no persistent indicator confirmed.
- **Complaint**: GitHub users want "Hide whitespace" to persist across PRs
  (https://github.com/orgs/community/discussions/65597 — title only).

## 4. Overview / minimap strip (`ui.overview`)

- **Araxis Merge**: since 2021 the overview strips sit in the vertical scroll bars, colour-coded marks per
  difference, shift-click to jump; inner thumb shows the visible part
  (https://araxis.com/merge/documentation-windows/file-comparison-overview — search snippet).
- **VS Code**: `editor.minimap.enabled` "Controls whether the minimap is shown"; diff editor `renderOverviewRuler`
  (editorOptions.ts :3505-3508, :984). Default value not read from source (`defaults.enabled`) — **question**.
- Open: complaints about minimaps taking width (not searched).

## 5. Previews of rewritten history (ghost rows), graphical vs text tier

- **lazygit**: during an interactive rebase the todo entries show as rows in the commits panel, editable with
  s/d/e/p/f (https://github.com/jesseduffield/lazygit/wiki/Interactive-Rebasing; PR #4442 shows todo items for
  pending cherry-picks/reverts, https://github.com/jesseduffield/lazygit/pull/4442). Text tier only; the preview
  is the real todo, not a ghost over the graph.
- No tool found that draws a preview in a pixel graph and falls back to a text graph for it. Open: GitUp,
  Sublime Merge, GitKraken preview behaviour (not read).

## What shipped vs prior art

| Aspect | Shipped (source) | Prior art |
|---|---|---|
| Fold mark | Muted ` +N` after subject; `+` counting, `+?` failed; N by `rev-list --count` cached per merge (17 dec. 2; 17b dec. 1) | **Supports**: jj marks elisions without a count and users request one (#3099); JetBrains uses a dotted line, no count |
| Fade | Off-thread lanes' hue mixed 45% to `bg` at truecolor/256; DIM at 16/`NO_COLOR` (17 dec. 3) | **Supports** the idea (Git Graph mutes off-HEAD text; JetBrains greys non-picked). Silent on lanes vs text |
| Whitespace | Title tail names the mode only when off the screen's default (17 dec. 8; 17b dec. 3) | **Supports** making the state visible (lazygit added toasts); silent on persistent tail. Trailing-ws highlight (delta) not shipped — not asked |
| Overview | `ui.overview` key, default false; `O` toggles (17 dec. 5) | **Silent/unclear**: Araxis, VS Code have overviews; their defaults not confirmed |
| Ghosts at Pixel | Whole graph column drops to Unicode during preview (ADR-0060) | **Silent**: no tool found with this tier split |
