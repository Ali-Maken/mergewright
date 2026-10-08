# M8: polish, final name and ship — pass 2b, 2026-10-02

**Status: pass 2b — all specs written.** Decisions recorded (his answers 2026-10-02): ADR-0054 pacing, ADR-0055
keymap, ADR-0056 help + tour, ADR-0057 Tributary + what ships, ADR-0058 chrome + cursor row + dark B. Specs in
`.scratch/m8-polish-and-ship/issues/` (list: "Expected groups"). Inputs: PROMPT.md §8
M8 row, §5, §9; research `scroll-smoothness.md`, `big-repo-scroll.md`, `themes.md`, `keymap-help-remap.md`,
`first-run.md`, `install-docs.md`; `docs/naming-research.md`; field notes :46, :48–50, :82–86 and 2026-10-02; the
prototype (`prototypes/m8-polish-and-ship/README.md`, `docs/shots/m8-proto-*.txt`); `real-vs-mockup.md`; MAP.md of
graft, graft-ui, graft-term (2b: also graft-ops, graft-git headings). No source read.
2026-10-07, palette find (steps 3–4): `palette-find.md` → Prior art "Find" rows, J26–J32, P-find-1/2 (Where I'd differ).
2026-10-07, approved (field notes :92): P-find-1 and P-find-2 → ADR-0061; 17b's Pixel ghosts → ADR-0060.
2026-10-08, look polish (13l box C, review M1), **post hoc**: `m8-look-polish.md` → Prior art "look polish" rows,
J33–J37, fence ids `fold-merge-count` and `whitespace-mode-shown`, P-look-1/2 (Where I'd differ). Written after
17/17b/17c shipped; the study did not inform them. Nothing here changes the build.

**Promise (M8):** *"It stays smooth on your biggest repo, looks like the mock-up, installs with one command, and teaches
itself from inside the app."* The acceptance walk proves it: held-key and wheel tails on MONOREPO, every journey at or
under its count, the look captures beside the mock-up's, `brew install` → run on a clean account, the tour finished
with no shell.

**Order: scroll, then look-parity, then the rest** (field notes :85; his (b)). Nothing else starts in the graft
`event_loop` until G-pace lands.

## Need

| What | His words / evidence | Weight |
|---|---|---|
| Scroll on his work repo (17,694 commits, 1,323 refs, Ghostty full screen, Pixel tier) | "i stopped pressing and it kept going" (:82); after 41–48 "still a bit laggy" (:84); "not just j but scroll as well … vscode never lagged neither did sublime merge … up to you" (:85) | daily, every minute |
| Looks like the mock-up | "the prototype looks better than the product" (field notes 2026-10-02); six differences in `real-vs-mockup.md` | daily |
| Learn the app | "He did not know dragging existed … I'll definitely need a tutorial" (:46) | once, then rare |
| Keys he can't remember | "`?` does nothing", `⌃P` and `:` too (M6/u2; `real-vs-mockup.md` r04); PROMPT §9: 7 daily tasks "using only the hint bar and the command palette" | daily |
| Find a branch, tag, commit or file by name | no field note in his words yet; DESIGN §1.6 (one box over actions, branches, tags, files, commits); 1,323 refs on his work repo (row 1) | daily (assumed, unmeasured) |
| Readable in his theme | cursor row unreadable in dark, twice (:44, :48); 3.04:1 (:50); text on it 4.28:1 (M5/44); branch label on it "dim magenta on grey, hard to read" (real-vs-mockup 6) | daily, dark |
| Install like a real tool | runs a build from this repo today | once |
| Two daily rulings | "pull with nothing new → cursor stays; `a` on HEAD with nothing staged → reword" (:86) | daily |
| Fork left behind (2c) | "mid-chain branch left behind after `a` → fine until M8" (:86); ruled 2026-10-02 (:89 item 5): it **moves too** when it moves clean, shown in the preview first | with dependents |
| States you can't see (look polish, 17, post hoc) | no field note in his words; open gap rows carried by 17's Decisions 2, 3, 5, 6, 8 (a folded merge looks plain, fade lost where SGR 2 is ignored, no `[ui]` key for the strip, ghosts at Pixel, whitespace mode on no screen) | daily (assumed, unmeasured) |

## Prior art (T take · R reject · I invent)

| Aspect | Best answer (study) | Ours | |
|---|---|---|---|
| Paint only visible rows | VS Code ListView (`scroll-smoothness` §1) | row cache + strip cache (DESIGN §3.6, §3.8) | T (have) |
| One paint per display frame | VS Code `scheduleAtNextAnimationFrame`; ratatui frame-rate recipe (`big-repo-scroll`) | a turn's wakes share one frame (m7/43) | T (have) |
| **Never let the terminal fall behind** | nothing found | DSR fence per frame (ADR-0054) | **I** — studies cover input batching, not output pacing |
| Atomic frames | DEC 2026 (§3) | text **and** image commands inside one pair | T |
| Wheel bursts | lazygit 2 rows/event, zellij per event (§2) | a turn's wheel events sum; per-event step unchanged until scene 1's verdict | T step · I sum |
| Per-row work while moving: follow at once, cancel on move | lazygit `NewTask` stops the previous diff on each move; fzf `cancelPreview` then starts the new preview, no debounce (`scroll-smoothness` "Addendum: Per-row work while the cursor moves") | a burst's first motion (leading edge) follows at once; a stale detail follow or radar probe is cancelled when the cursor moves (03) | T (lazygit, fzf) |
| Per-row work while moving: defer during a burst | nothing fits (same addendum): lazygit/fzf never defer — they spawn per move, lazygit throttling 30 ms only under load; VS Code Timeline's 500 ms trailing debounce is too slow for a held key | later motions in a burst defer the detail follow and radar probe to rest: settle = clamp(2 × widest gap, 50 ms, 150 ms); emphasis (per-frame drawing) not gated, judged in 05 (03 revision) | **I** — nothing fits, because no tool found both follows the leading edge and defers the burst; 150/50 ms come from key-repeat rates, not prior art |
| Kitty uploads | re-place by id, `d=p` (§3) | have (m7/46) | T (have) |
| Kitty Unicode placeholders | yazi; WezTerm doesn't render | — | R |
| Measure where he is | gitui #2823 | opt-in `GRAFT_PERF_TRACE`, numbers only | I (rule 1, :84) |
| Theme auto + pin | zellij, bat (`themes.md`) | OSC 11 + `ui.theme`, focus re-query | T (have) |
| Switch theme now | zellij ToggleTheme, helix `:theme`, VS Code ⌘K⌘T | palette rows `Theme: …`, no key | T |
| Change one colour | gitui 0.23, helix `inherits`, VS Code `colorCustomizations` | `[theme.dark] selection = "#…"` | T |
| Contrast floor | nobody checks (lazygit #4306/#508, gitui #922) | every theme tested against fixed floors; an override that misses one is named | **I** (we shipped the bug, :48) |
| Per-widget attrs; file-only themes; palette-inheriting default | lazygit; gitui; bat `ansi` | — | R |
| **Screen chrome** | repo + branch on top: Magit, `Head:`/`Push:` as the status screen's first lines; words in the bar: gitui's per-panel key-hint bar, zellij's mode hints — and zellij's lesson that a full-time bar costs space users complain about (`chrome-and-empty-state` §1, verdict `:51`) | ADR-0058 §4, DESIGN §1.0; layout from the M8 mock-up (his pick, 2026-10-02) | T (Magit top, gitui/zellij bar) · gutter `▸`: **I** (next row) — nothing fits, because no studied tool marks the cursor row with a gutter glyph (§1, none found) and no flat fill clears 3:1 over twelve lanes |
| **Cursor row over coloured lanes** | VS Code: fill under everything; lazygit: reverse video — both hide a lane of the fill's luminance (lazygit #4306) | fill starts after the graph column; `▸` in a gutter marks the row | **I** — no flat fill clears 3:1 against twelve hues (row 542) |
| **Text on the fill** | VS Code `minimumContrastRatio` 4.5 (`themes.md`) | `Theme::legible_on`, 4.5:1 | T |
| `?` = live keys here, grouped | lazygit `?`; #3141 (`keymap-help-remap.md`) | palette opened on this place, IN PROGRESS first (ADR-0056, DESIGN §1.16) | T |
| Search by name and by key | VS Code, JetBrains Find by Shortcut | typed bound chord ranks first | T |
| Help from the live table | which-key | bar, `?`, palette, `.`, tour, man from one table | T |
| Remap collisions; unbind | JetBrains warns; tig #816 | checked at load; `"none"` | T |
| Config name beside each key | nobody | `graph.merge` beside `m merge` | **I** |
| Key recorder; which-key popup | JetBrains, VS Code; emacs | — | R |
| First launch | lazygit one-shot popup | one status line, gone after use or 5 launches | T (shrunk) |
| Tour form | vimtutor; JetBrains learning project; VS Code walkthrough ticks on doing | practice repo, 8 tasks (ADR-0056) | T |
| Tour exit visible; shows his keys | zellij #5064; helix #13483 | `esc leave` every step; words from the keymap | T |
| Undo as the tour's hook | GitUp sells undo | task 3 | **I** |
| Tips, wizard, video | lazygit, zellij, JetBrains | — | R |
| **Empty state** | lazygit `notARepository: prompt` — open a recent or init here; its users asked for the picker, not an exit (#3795, #3740); Sublime Merge: clone page + New Repository, both open the repo when done; JetBrains welcome: recents first, with a filter (`chrome-and-empty-state` §2, verdict `:65`). Lessons: a recent since deleted must not crash (lazygit #1609); clone stays findable after first use (VS Code complaint) | mock-up §1.15: o/i/c/t + RECENT, newest preselected | T (lazygit, Sublime Merge, JetBrains) |
| Binaries, tap, installer | dist v0.33 (`install-docs` §2) | 4 targets, tap formula, curl installer | T |
| homebrew-core; clap_mangen; completions; update notifier | all in core; ripgrep; Homebrew discourages | — | R |
| Man page | ripgrep `--generate man` | from our CLI table + keymap; staleness test | T |
| Notarisation | dist doesn't | none (ADR-0057) | R (his call) |
| Reword HEAD with a branch on it (2c) | lazygit reword, no preview (`stacked-branches` §1.9) | M7's rule: previewed when a branch is built on HEAD (keymap `a` row, :219) | T (have) |
| **Fork off a rewritten commit** (2c, approved 2026-10-02) | jj: every descendant follows (`stacked-branches` §1.4); branchless `restack` + skip-on-conflict (§1.3); Graphite `sync` skips conflicting (§1.2) | follows when its replay forecasts clean, else stays and says why (ADR-0059, accepted) | T descendants · T skip-on-conflict (for forks only) |
| **Quit while a write runs; a write that hangs** | `docs/research/quit-while-git-runs.md` (2026-10-04): no tool found waits, then offers a stop after a delay, and says what state it leaves. JetBrains confirms on exit with tasks running (wording unconfirmed); Magit stops a process in two stages, interrupt then kill (`k`); lazygit, VS Code and gitui hang on a stuck hook with no way out but killing the app (lazygit #3557, vscode #171353, gitui #2546) — the dominant complaint | wait for it, then quit; after 10 s `q` offers to stop it half-way; a signal gives the terminal back first, then waits ≤ 60 s (keymap §2.2) | T confirm on exit (JetBrains) · T interrupt-then-kill order (Magit: SIGTERM, SIGKILL after 2 s) · **I** the 10 s offer and the wait-then-quit: nothing found fits. Argued from ux-principles §3.1 (where he lands), heuristic 4 (one box, one `y`) and the measured half-rebase (13b NOTES). The research flagged "leaves the merge paused" as unverified. 13b found a merge stopped in its pre-merge-commit hook did **not** pause; 13c–13d end the hook first, so git pauses it itself (graft-ops it::stop_hooks), per the user's ruling 2026-10-04 (field notes :91) |
| **Find: one box, grouped** (2026-10-07) | JetBrains Search Everywhere: one entry, results in tabs; its Git tab is opt-in (`palette-find` "JetBrains") | one `⌃P`, groups in §2.12's fixed order, a header each, ≤ 8 rows | T grouped · R opt-in Git tab: git objects are the point |
| Find: empty box | JetBrains opens on recent files (same); VS Code: Question | `COMMANDS RUN RECENTLY`, then `ACTIONS`; find groups once he types (P-find-1) | T, narrowed to §2.12's own "recently run commands" |
| Find: match rule | lazygit #3373: fuzzy too loose at 4,000 branches, asks substring per word (`palette-find` "lazygit") | `fuzzy_score`: every word a substring, segment start first (m8/11) | T (have) |
| Find: branch order | Magit #2872: alphabetical is wrong, asks `--sort=-committerdate`; MRU asked on the list (`palette-find` "Magit") | score, local before remote, newest tip, name — the `C` picker's rule (`switch_list`), so one query orders alike in both (H4) | T newest tip · R MRU: a reflog read per open; `C` already offers `↩ before` |
| Find: slow sources | JetBrains lags waiting for every contributor; the workaround is not waiting (IJPL-131511) | each group lands on its own; `reading…` / `searching…` in its place (H1) | T |
| Find: typing on a big repo | per-key cancel: Question, no source (`palette-find` "Big repos") | each edit cancels the last job; the loop never scans; a stale answer is dropped; shown rows re-filtered | **I** — nothing documented fits; PROMPT §6's 16 ms at 50k refs |
| Find: commits, how far | lazygit #3279 wants git-side search, not loaded rows; GitUp searches the whole repo | `/`'s walk over every ref, first 9 hits, then `more · / searches all of history` | T (lazygit ask, GitUp) — settles the study's Question |
| Find: commit fields | GitLens: message, `@author`, `#sha`, `:path`, `~patch` | subject, author, body, sha prefix (`/`'s `Query`); paths → FILES | T message/author/sha · R `~patch`: a diff per commit, `/` doesn't either |
| Find: kind prefixes | VS Code `>` `@` `#` `:`; GitLens `@` `#` `:` | none: headed, capped groups narrow by sight | R — recall, not recognition (UX H6); `:` already opens the palette (§2.12) |
| Find: what `⏎` does | Sublime Merge "Navigate to branch", forgit `gcb`/`gsw`: verb first, then pick | object first: `⏎` puts the cursor on it; the graph's own keys act; a file opens its history | R verb-first · T lazygit's search-not-filter reason: commits before and after stay in view |
| Find: files | VS Code Quick Open by name; lazygit #1525 asks a fuzzy file picker for history | FILES from `ls-files`; `⏎` opens its history (`F`) | T |
| **Fold mark `+N`** (look polish, **post hoc**, 17/17b) | jj: elided commits draw `~` or `(elided revisions)`, no count; jj users ask for one (#3099, title only); JetBrains "Collapse Linear Branches": a dotted line, no count (`m8-look-polish` §1) | muted ` +N` after the subject of a folded merge; `+` counting, `+?` failed; N = `rev-list --count M ^M^1` − 1, once per merge, cached (17b dec. 1) | T mark the elision on its row (jj) · **I** the count — nothing found shows one, and jj's users ask for it · R a dotted line instead of a row: it can't carry a number |
| **Fade what's off the thread** (look polish, **post hoc**, 17) | VS Code Git Graph mutes the *text* of commits not reachable from HEAD, opt-in (default false), merges muted by default; JetBrains greys non-picked commits (`m8-look-polish` §2) | off-thread lanes' hue mixed 45% toward `bg` at truecolor/256; DIM at 16 colours and `NO_COLOR`; Pixel keeps alpha (ADR-0024) (17 dec. 3) | T fade the less relevant (Git Graph, JetBrains) · **I** fading *lanes* by hue — the study is silent on lanes vs text; SGR 2 alone shows nothing where a terminal ignores it (17 dec. 3) |
| **Whitespace mode visible** (look polish, **post hoc**, 17/17b) | lazygit `⌃W`, then a toast "Ignoring whitespace" (PR #1340, snippet only) — transient, no persistent mark confirmed; VS Code `diffEditor.ignoreTrimWhitespace`, toolbar state unconfirmed; GitHub users ask "Hide whitespace" to persist (#65597, title only); delta highlights whitespace errors (`m8-look-polish` §3) | `w` toggles (M7); the title tail names the mode only where it departs from the screen's default: detail/history `· whitespace ignored`, blame `· whitespace counted` (17b dec. 3) | T make the mode visible (lazygit) · **I** a persistent tail, off-default only — a toast is gone when he looks back; "show it only when there's something to report" (CLAUDE.md) · R delta's trailing-whitespace highlight: no field note asks |
| **Overview strip on at start** (look polish, **post hoc**, 17) | Araxis: overview strips in the scroll bars, a mark per difference, click to jump (snippet only); VS Code `editor.minimap.enabled`, diff `renderOverviewRuler` — defaults not read (`m8-look-polish` §4) | `ui.overview = true \| false`, default false; `O` per run (17 dec. 5) | T a setting (VS Code) · default off: unjudged — the study left both defaults open (P-look-2) |
| **Ghost Preview at Pixel** (look polish, **post hoc**, ADR-0060) | lazygit: a rebase's todo shows as rows in the commits list, text only (wiki "Interactive Rebasing", PR #4442); no tool found draws a preview in a pixel graph (`m8-look-polish` §5) | while a preview's picture is drawn, the whole graph column draws at Unicode; strips re-place from cache on close, no re-raster (17c) | **I** — nothing found; §1.7 rules out a lane half pixel, half text (ADR-0060) |

**Look polish rows (post hoc): nothing shipped is rejected.** Each R above rejects another tool's answer (a dotted
line, delta's highlight), not a shipped choice, so 13l box C's "each reject becomes a follow-up ticket spec" yields
none. The two open points are questions (P-look-1/2), not defects: the study's coverage is thin by its own account.

**Dark pick: B** (`docs/shots/m8-proto-4a-dark-a-b.txt:21-24`): cursor row vs ground **3.14** (A 3.05, floor 3.0); text
on cursor row **5.14** (A 4.61, floor 4.5); muted text on ground **6.95** (A 6.15, floor 4.5). Blue lane on cursor row
1.20 (A 1.33): fails in every candidate, so the fill leaves the graph column — lanes on the cursor row sit on ground,
worst lane-3 `#AA3377` ≈ 2.97:1, as on every row today (ADR-0003 band). Ground `#14161B` unchanged. Floors as tests:
text/bg 4.5 · text/selection 4.5 · selection/bg 3.0 · text-muted/bg **4.5** (pass 1 said 3.0; B clears 4.5).

**Pipeline check after 2b.** Prior art: every new aspect has a row. Journeys: every id in the fence has an owning spec
and a walk (`accept-m8.md`). Prototype (step 5) ran, scenes 1–4 shot; scene 1's feel (blind 1/2, wheel step) still
his — the wheel step stays per event until then. Satisfied.
Palette find (2026-10-07): study and rows done; no prototype — find adds rows to 11's built widget, no new screen or
key (conductor: confirm that reading of step 5).
Look polish (2026-10-08, **post hoc**): steps 2–4 written after the build (review M1). Step 5 was not run for 17/17b/17c;
ADR-0060 was his ruling instead (field notes :92). No new key. Fence gains two walked ids (J33, J34); J35–J37 are
reads or person steps, held by 17/17c's tests and the TESTING.md rows (`28-amendment.md`).

## Journeys (steps = keys + decisions, UX §1.3; start: his repo, graph, cursor on HEAD)

| # | Task | Graft | Steps | Best tool |
|---|---|---|---|---|
| 1 | Hold `j` ~300 rows, let go | hold `j` | 1+0; stops ≤ 100 ms after release | VS Code / Sublime Merge |
| 2 | Wheel / trackpad flick | gesture | 1+0; stops when input stops | same |
| 3 | Jump to the oldest commit | `G` | 1+0; ≤ 16 ms after layout | same |
| 4 | Key for an action he knows by name | `?` name | 2+1 | lazygit 2–3, VS Code 3 |
| 5 | What does `x` do here | `?` `x` | 2+1 | JetBrains ~3 |
| 6 | Run an action he can't recall | `⌃P` name `⏎` | 3+1 | VS Code 3 |
| 7 | Rebase stopped: how do I go on? | `?` | 1+1 | lazygit unanswered (#3141) |
| 8 | Rebind an action | `?` name, edit 1 line, restart, `?` | 5+1 | VS Code 5, JetBrains ~7 |
| 9 | Light theme for this run | `⌃P` `theme` `⏎` | 3+1; auto 0 | VS Code 3, zellij 1 |
| 10 | Fix one unreadable colour | edit 1 line, restart | 2+1 | VS Code 2+ |
| 11 | First launch | — | 0 | lazygit 1 |
| 12 | Start the tour | `⌃P` `tour` `⏎`; empty state `t` | 3; 1 | helix 2 |
| 13 | Leave the tour | `Esc` | 1; his repo, same cursor | zellij trapped |
| 14 | Open yesterday's repo, outside any repo | `trib` `⏎` (newest preselected) | 2; second-newest 3 | Sublime Merge 2+ |
| 15 | Clone | `c` URL `⏎` | 3+1 | Sublime Merge ~4 |
| 16 | Install | `brew install <owner>/tap/tributary`, `trib` | 2 | homebrew-core 2 |
| 17 | Read the manual | `man trib` | 1 | ripgrep 1 |
| 18 | Pull, nothing new | `p` | 1; cursor stays | lazygit 1 |
| 19 | Reword HEAD, nothing staged, a branch built on it | `a`, edit, `⏎`, `y` (preview: the branch follows) | 4+2; nothing built on it: 3+1, no preview (M7's amend, unchanged) | lazygit 2–3, no preview |
| 20 | Undo, see what came back | `u` | 1; lands on the result | GitUp 1 |
| 21 | Make this folder a repo | `i` `y` (empty state) | 2+1 | Sublime Merge ~3 |
| 22 | Which repo and branch am I in | read the header | 0 | VS Code status bar 0 |
| 23 | Find the cursor row, read its branch | read the `▸` row | 0 | Sublime Merge 0 |
| 24 | Quit while a merge or rebase runs | `q` `y`, then nothing: it quits when the write lands | 2+1 | JetBrains ~2, confirm on exit (`quit-while-git-runs`; wording unconfirmed) |
| 25 | Quit while a git hook hangs | `q` `y`, wait 10 s, `q` `y` | 4+2 | lazygit, VS Code, gitui: no way out but killing the app (`quit-while-git-runs`) |
| 26 | Look at a branch, not switch to it | `⌃P` name `⏎` | 3+1; cursor on its tip, HEAD unchanged | Sublime Merge 4 ("Navigate to branch"), JetBrains 3–4, lazygit 4 (`palette-find` Journeys) |
| 27 | Jump to a tag | `⌃P` tag `⏎` | 3+1 | JetBrains 4 (Git tab) |
| 28 | Commit by sha prefix | `⌃P` sha `⏎` | 3+1 | Sublime Merge 4 (`navigate_to_commit`), GitLens 4 |
| 29 | Commit by subject words | `⌃P` words `⏎` | 3+1; +1 (`↓` run, UX §1.3) when a group above also matches | forgit 3, JetBrains/GitLens 4, lazygit 4+ |
| 30 | Open a file's history | `⌃P` path `⏎` | 3+1 | lazygit 3 (`⌃S`), JetBrains/VS Code 4+ |
| 31 | Run again what he ran from the palette | `⌃P` `⏎` | 2+1 (P-find-1, approved 2026-10-07, ADR-0061) | VS Code 3 (J6); JetBrains: recent files only |
| 32 | Switch to yesterday's branch | `C` name `⏎` | 3+1 (M7's picker, newest first; not walked here) | forgit 3, JetBrains 3–4, Sublime Merge 4, lazygit 4 |
| 33 | Fold a merge, see how many commits it hid (**post hoc**) | `Space` on the merge | 1+0; row ends ` +` then ` +N` when counted | none shows N (`m8-look-polish` §1): jj needs the shell (#3099 asks for a count); JetBrains a dotted line |
| 34 | Ignore whitespace, and know it's on (**post hoc**) | `w` | 1+0; the title tail names it while it lasts; reading the mode later: 0 | lazygit 1 (`⌃W`, toast fades; afterwards no mark confirmed, `m8-look-polish` §3) |
| 35 | Which commits are off my thread (**post hoc**) | read the lanes (cursor on a commit: its ancestry stays full, the rest fades) | 0 | JetBrains 0 (non-picked greyed); Git Graph 0 once its opt-in setting is on (1 edit) |
| 36 | Overview strip on at every launch (**post hoc**) | edit 1 line (`ui.overview = true`), restart | 2+1; this run only: `O` 1 | VS Code 2 (`editor.minimap.enabled`; default unconfirmed) |
| 37 | See a rewrite's result at the Pixel tier (**post hoc**) | the preview's own keys | +0 over its journey (J19 at Pixel: 4+2); graph column at Unicode while the picture shows | none found at a pixel tier; lazygit's todo rows, text only |

**J19 is 4, not 3 (2c).** The 2b row counted the field-note case (:86) as the plain amend. But `a` on HEAD with
nothing staged was refused only when a branch is built on HEAD (ticket 14 item 1) — the case where other branches
move. There the keymap already says "previewed (`y`) only when a branch is built on HEAD" (`docs/design/02-keymap.md:219`,
his M7 "all yes"). Kept: M5's promise and UX §2 H5 (see the result before confirming: which branches move is not on
the editor); H4 (every history-changing preview confirms with `y`, field notes :58). UX §3.5 (undo permits fewer
confirms) argues for 3, but the preview here shows something nothing else on screen does. The build is right; only the
count was wrong. Nothing built on HEAD stays 3, no preview, not in the fence (M7's path, unchanged by M8).

**Find rows judged (UX §1.3, §5).** The typed text is the target: 1 key, and nothing could pre-select it, so 3 is
the floor for a named find (not 1 at any cost). The `⏎` after it is +1 decision: he checks the top row. Every find
row is at or under the best tool's count — J26–J28 one under (no "which command" step: UX §5 item 2, one box);
J29/J30 equal to the best, which needs one tool per task (forgit `glo`, lazygit `⌃S`); J31 one under VS Code, by
pre-selecting the likely answer (UX §5 item 3, Hick §1.4). Seek, not switch (J26): switching stays on `C` (J32), so
`⏎` on a branch row is never a write and needs no preview (UX §3.5, H5). J29's `↓` costs one motion command; its
walk's fixture has no other group matching, as stated in the fence.

**Look polish rows judged (post hoc; UX §1.3, §3.1).** J33: the fold is the key he already presses; the count costs
nothing more — the best tool needs the shell. J34: equal to lazygit on the toggle, ahead on reading it back (0 vs
toggling again to find out). J35 and J37 are reads, 0 extra: the fade follows the cursor, and the Pixel preview needs
no key of its own. J36 matches J10's shape (a config line), one per-run key beside it. None is over the best tool's
count. J35 rests on colour: rule 5, unverified until his eye.

**Where he lands** (UX §3.1):

| Event | Lands |
|---|---|
| Key released / input stops | the frame with the last applied input; nothing after it |
| Startup with `[keys]`/`[theme]` problems | graph; status row `config: 2 problems · ? shows them` |
| Startup outside a repo | empty state, newest recent preselected (§1.15) |
| Open / init succeeds | that repo's graph, cursor on HEAD |
| Open fails | empty state stays, row stays, bar row reads the reason |
| Init declined (`esc`/`n`) | empty state, nothing written |
| Tour step done | card ticks, next task's words |
| Tour finished or `Esc` | his repo, cursor where he left it; finished adds `practice done ✓ · ⌃P tour goes again` |
| Tour repo can't be built | stays in his repo; `tour: <reason>`; nothing written in his repo |
| Clone fails | prompt stays, URL kept, reason + `⏎ retry · esc cancel` |
| Clone done | new repo's graph, cursor on HEAD |
| Pull, nothing new | cursor where it was, `up to date ✓` |
| `u` / `U` done | on HEAD if it moved, else on the first ref the entry touched (2b) |
| `q` while a write runs | box `Quit?` (keymap §2.2). Can stop (fetch, undo): `<op> is still running · quit stops it`, `y` stops it, quits. Can't stop (merge, rebase, …): `quit waits for it to finish`, `y` → status row `<op> · quitting when it finishes · esc stays`; ends done → journaled, quits; stops or fails → stays, where that event lands (2b) |
| Stuck write, `q` `y` after 10 s | shell, with one line saying where git was left (keymap §2.2 "A stuck write"; the six lines in 13d); next launch: banner `c finish · a abort` (a merge) or `c continue · a abort` (rebase, cherry-pick, revert), set-aside edits offered. Graft ends the stuck hook, not git, so git pauses a merge, cherry-pick or revert itself; a rebase stays paused at its step (13c–13d); git stuck outside a hook: no pause, `u` puts the files back, and the exit line says which. |
| SIGTERM / SIGHUP / terminal closed while a write runs | terminal given back at once (closed: nothing printed); the write lands within 60 s → exits; else stopped as the stuck write's `q` `y` (hook first, git pauses it), with that row's exit line; a second signal stops it at once; never exit 101, never git left running (13g) |
| `q`, git stopped mid-operation, nothing running | quits, no box; next launch keeps the banner: continue or abort (keymap §2.6a, 2b) |
| On-disk rename can't finish | old names stay in force; `rename: still using the old names · <reason>` (2b) |
| Palette branch, tag or commit `⏎` | palette closed, graph focused, cursor on the commit, detail follows; not laid out → `jumping` (`Esc` withdraws) |
| Palette file `⏎` | that file's history place at HEAD; `Esc` → graph, cursor where it was before `⌃P`; not in HEAD → palette closed, `<path> isn't committed yet · no history` |
| Palette `⏎` on `reading…` / `searching…` | nothing; the cursor moves to the first real row when one lands (it was on the placeholder) |
| Palette `Esc`; nothing matches | graph, cursor unchanged; `nothing matches · esc close` (11) only once every group has answered |
| A find source fails | its group reads `branches: <reason>` / `files: <reason>` / `commits: search failed: <reason>`, muted; other groups work (H9) |
| `more · / searches all of history` | palette closed, `/` running for the query, `n`/`N` step (as `/`'s `⏎`) |
| Merge folded (`Space`) (post hoc, 17b) | cursor stays on the merge; its row ends ` +` (counting), then ` +N`; N = 0 → no mark; refresh doesn't recount |
| Fold count fails (post hoc, 17b) | row ends ` +?`; status row, once per merge, `couldn't count <sha7>'s commits: <git's reason>` |
| Preview's picture drawn at Pixel (post hoc, ADR-0060, 17c) | graph column at Unicode, ghosts as text; header `gfx` unchanged; F3 still cycles |
| Preview closed at Pixel (post hoc, 17c) | strips back from the cache, no new raster |

```journey-counts
# id                          keys  # decisions · best tool · the keys
hold-j-stop                   1     # +0 · VS Code/Sublime Merge 1 · hold j (tail ≤ 100 ms after release)
wheel-flick                   1     # +0 · same · wheel (stops when input stops)
jump-oldest                   1     # +0 · same · G
find-key-by-name              2     # +1 · lazygit 2–3, VS Code 3 · ? <name>
what-does-key                 2     # +1 · JetBrains ~3 · ? x
run-unrecalled                3     # +1 · VS Code 3 · ⌃P <name> ⏎
stopped-how-go-on             1     # +1 · lazygit unanswered · ?
light-theme                   3     # +1 · VS Code 3, zellij 1 · ⌃P theme ⏎
first-launch                  0     # +0 · lazygit 1 · —
start-tour                    3     # +0 · helix 2 · ⌃P tour ⏎
start-tour-empty-state        1     # +0 · helix 2 · t
leave-tour                    1     # +0 · zellij trapped · Esc
open-recent-outside           2     # +0 · Sublime Merge 2+ · trib ⏎
open-second-recent            3     # +0 · Sublime Merge 3 · trib j ⏎
init-here                     2     # +1 · Sublime Merge ~3 · i y
clone                         3     # +1 · Sublime Merge ~4 · c <url> ⏎
which-repo-branch             0     # +0 · VS Code 0 · —
cursor-row-branch             0     # +0 · Sublime Merge 0 · —
pull-nothing-new              1     # +0 · lazygit 1 · p
reword-head                   4     # +2 · lazygit 2–3 (no preview) · a <edit> ⏎ y (a branch built on HEAD)
undo-see-result               1     # +0 · GitUp 1 · u
quit-while-write              2     # +1 · JetBrains ~2 · q y (quits when the write lands)
quit-stuck-write              4     # +2 · lazygit/VS Code/gitui: kill the app · q y … 10 s … q y
seek-branch                   3     # +1 · Sublime Merge 4, JetBrains 3–4 · ⌃P <name> ⏎ (HEAD unchanged)
seek-tag                      3     # +1 · JetBrains 4 · ⌃P <tag> ⏎
seek-commit-sha               3     # +1 · Sublime Merge 4, GitLens 4 · ⌃P <sha prefix> ⏎
seek-commit-words             3     # +1 · forgit 3, JetBrains 4 · ⌃P <words> ⏎ (fixture: no other group matches)
file-history                  3     # +1 · lazygit 3 · ⌃P <path> ⏎
rerun-recent                  2     # +1 · VS Code 3 · ⌃P ⏎ (P-find-1; 3 if declined)
fold-merge-count              1     # +0 · none shows N (jj: shell, #3099) · Space on a merge (row ends +N) — post hoc
whitespace-mode-shown         1     # +0 · lazygit 1 (toast fades) · w (title tail names the mode) — post hoc
```

Not walked in the pty (a person's step, a TESTING.md row): rebind (5+1), fix a colour (2+1), install (2), man (1),
overview on at start (J36, 2+1). Reads with no key, held by their tickets' tests and his eye: off-thread fade (J35,
17's buffer test; colour, rule 5), Pixel preview (J37, 17c's frame-bytes tests; a real Kitty/iTerm2 terminal is his).
Before G-rename lands, `trib` in the walk is the dev binary's name.

## Interfaces and data shapes

```rust
// graft-term — frame bracket and fence (G-pace)
pub const SYNC_BEGIN: &[u8] = b"\x1b[?2026h";
pub const SYNC_END: &[u8] = b"\x1b[?2026l";
pub fn write_fence(out: &mut impl std::io::Write) -> std::io::Result<()>;     // CSI 5 n
pub fn fence_answers(bytes: &[u8]) -> usize;                                   // count of CSI 0 n
// graft-core::Capabilities gains `pub answers_dsr: bool` (startup DSR answered)

// graft::event_loop (pub(crate)) — at most one unanswered frame
pub(crate) struct Pacer { /* sent_at, misses, fenced, last_drawn */ }
impl Pacer {
    pub(crate) fn new(fenced: bool, fence_timeout: Duration) -> Pacer;
    pub(crate) fn may_draw(&self, now: Instant) -> bool;
    pub(crate) fn drew(&mut self, now: Instant);          // frame + fence written
    pub(crate) fn answered(&mut self, now: Instant);      // Wake::Fence
    pub(crate) fn deadline(&self) -> Option<Instant>;     // wake for the timeout / 16 ms cap
}
// Wake gains `Fence`.

// graft::event_loop::rest (pub(crate), G-settle, revised after its scout) — when the cursor has stopped.
// Rest is time since the last motion, not "a frame with nothing queued": paced frames go idle between key repeats.
pub(crate) const BURST_GAP: Duration = Duration::from_millis(150);   // closer motions are one burst
pub(crate) const QUIET_MIN: Duration = Duration::from_millis(50);
pub(crate) struct Rest { /* last, gaps: [Duration; 4], the burst's key */ }
impl Rest {
    pub(crate) fn new() -> Rest;
    pub(crate) fn in_burst(&self, now: Instant) -> bool;              // loop writes AppState::defer_follow from it
    pub(crate) fn moved(&mut self, now: Instant, by: Option<&Wake>);  // the wake just applied changed FollowKey
    pub(crate) fn rest_first(&self, wake: &Wake) -> bool;             // another input while owed: Rested before it
    pub(crate) fn deadline(&self) -> Option<Instant>;                 // last + clamp(2 × widest gap, QUIET_MIN, BURST_GAP)
    pub(crate) fn rested(&mut self);                                  // next motion is a leading edge
}
// graft::update — AppEvent gains `Rested`; AppState gains `defer_follow: bool` (loop-written, false in unit tests)
// and `followed: Option<FollowKey>`.
pub(crate) enum FollowKey { Commit(Oid), WorkingTree }
pub(crate) fn follow_key(state: &AppState) -> Option<FollowKey>;
pub(crate) fn follow_owed(state: &AppState) -> bool;

// graft::trace (G-trace) — GRAFT_PERF_TRACE=<file>; the test-only GRAFT_FRAME_TRACE is untouched
pub(crate) struct FrameRecord { pub wakes: u16, pub motions: u16, pub queued: u16, pub update_us: u32,
    pub commands: u16, pub draw_us: u32, pub bytes: u32, pub write_us: u32, pub fence_us: Option<u32>,
    pub input_age_us: Option<u32> }
pub(crate) struct FrameTrace { /* BufWriter<File> */ }
impl FrameTrace {
    pub(crate) fn from_env() -> std::io::Result<Option<FrameTrace>>;
    pub(crate) fn record(&mut self, r: &FrameRecord) -> std::io::Result<()>;
}

// graft-ui::theme (look-parity 5b; G-theme adds overrides)
pub fn contrast(fg: Rgb, bg: Rgb) -> f32;                          // WCAG 2.x
impl Theme { pub fn legible_on(&self, fg: Rgb, bg: Rgb) -> Rgb; }  // ≥ 4.5:1, 5% steps toward white (dark) / black (light)
pub struct ContrastRule { pub fg: Token, pub bg: Token, pub min: f32 }
pub const CONTRAST_RULES: &[ContrastRule];                         // the four floors above
pub struct ContrastMiss { pub rule: ContrastRule, pub ratio: f32 }
// G-theme, revised 2026-10-04 after its scout: Theme holds only reduced colours, so overrides go in at construction,
// before reduction; misses are a pure check on the file (all three kinds), not a by-product of building one theme.
impl Theme { pub fn with_overrides(kind: ThemeKind, caps: &Capabilities, overrides: &[(Token, Rgb)]) -> Theme; } // new = &[]
pub fn contrast_misses(kind: ThemeKind, overrides: &[(Token, Rgb)]) -> Vec<ContrastMiss>;                    // G-theme
impl Theme { pub fn lane_muted(&self, lane: usize) -> TokenColor; }                                   // G-look polish

// graft-ui::graph_screen — chrome (look-parity 5a)
pub const APP_NAME: &str = "graft";                                // G-rename: "Tributary"
pub struct HeaderView<'a> { pub repo: &'a str, pub path: &'a str, pub branch: &'a str,
    pub tail: Option<&'a str>, pub practice: bool }
pub fn header_line(view: &HeaderView, width: u16, theme: &Theme) -> ratatui::text::Line<'static>;
pub fn spaced(rows: u16) -> bool;                                  // rows >= 30: blank row + rule
// ScreenRows gains `gap: Rect`, `rule: Rect` (zero-height under 30 rows); CHROME_ROWS becomes `chrome_rows(rows: u16) -> u16`.

// graft-ui::empty_state (look-parity 5c)
pub struct RecentRow { pub name: SanitizedText, pub path: SanitizedText, pub age: SanitizedText }
pub struct EmptyStateView { pub folder: SanitizedText, pub missing: bool, pub recent: Vec<RecentRow>,
    pub cursor: usize, pub prompt: Option<EmptyPrompt>, pub problem: Option<SanitizedText> }
pub enum EmptyPrompt { Open(FieldView), ConfirmInit }               // G-first-run adds Clone(..)
pub struct EmptyState<'a> { pub view: &'a EmptyStateView, pub theme: &'a Theme }   // Widget

// graft — recents, terminal and in-process switch (5c; revised 2026-10-03 after its scout: age + skipped count needed,
// and `run` probed on every call)
pub(crate) struct Recent { pub path: PathBuf, pub opened: SystemTime }
pub(crate) struct RecentList { pub entries: Vec<Recent>, pub skipped: usize }      // newest first, ≤ 20, deduped
pub(crate) fn read_recent(config_dir: &Path) -> Result<RecentList, RecentError>;
pub(crate) fn remember(config_dir: &Path, repo: &Path, now: SystemTime) -> Result<Remembered, RecentError>;
// event_loop::startup: `Terminal` (session + probe + crash hook) opened once by main; `Place { Repo, Outside }`;
// `drive` loops over places and handles `Exit::Open`; `run(Startup)` stays open-run-close for the gallery and tests.
// Exit gains `Open(PathBuf)`; G-tour widens it to `Open(Open { path, cursor, practice, back })`

// graft-ops::init (5c) — not an Operation, not journaled, not undoable (no journal before it; reversing = deleting .git)
pub fn init_here(dir: &Path) -> Result<PathBuf, InitError>;

// graft-ui::keymap (G-keymap; ADR-0055) — `Binding.search` added in 2b (hidden palette words)
pub enum ContextId { Global, Graph, Changes, Diff, Resolver, Banner, Finish, Rewrite, Picker, Blame,
    History, Stash, Journal, Map, Preview, Menu, Palette, EmptyState, Tour }
pub struct ActionId(pub &'static str);                 // "graph.merge", never renamed after release
pub struct KeyChord { pub code: KeyName, pub ctrl: bool, pub alt: bool, pub shift: bool }
pub struct Binding { pub action: ActionId, pub context: ContextId, pub default: &'static [KeyChord],
    pub label: &'static str, pub search: &'static [&'static str] }
pub fn defaults() -> &'static [Binding];
pub enum Translated { Default(KeyChord), Unbound, Pass }
pub enum KeyWarning { UnknownAction(String), BadKey { action: String, text: String },
    Collision { key: KeyChord, context: ContextId, yours: ActionId, displaced: ActionId }, Reserved(KeyChord) }
impl Keymap {
    pub fn with_overrides(lines: &[(String, Vec<String>)]) -> (Keymap, Vec<KeyWarning>);
    pub fn translate(&self, ctx: ContextId, pressed: KeyChord) -> Translated;
    pub fn spell(&self, action: &ActionId) -> KeyLabel;
    pub fn in_context(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])>;
}

// graft-ui::palette (G-palette)
pub enum PaletteMode { Here(ContextId), Everything }
pub struct PaletteRow { pub label: SanitizedText, pub key: Option<KeyLabel>, pub config: Option<ActionId>, pub kind: RowKind }
pub struct PaletteView { pub mode: PaletteMode, pub query: FieldView, pub groups: Vec<(SanitizedText, Vec<PaletteRow>)>,
    pub cursor: usize, pub warnings: Vec<SanitizedText> }
pub fn fuzzy_score(query: &str, text: &str) -> Option<u32>;        // one matcher, graft-ops (12e; ADR-0007)

// graft::update::palette + jobs::palette (G-palette find, 2026-10-07; full shapes in specs 12 and 12b)
pub(crate) enum PaletteTarget { Action(ActionId), App(AppAction), Note, Seek(Oid), More(FindGroup), File(u32) } // Copy
pub(crate) enum ReadOnce<T> { NotAsked, Reading(JobId), Read(Arc<T>), Failed(String) } // one read per ⌃P open
pub(crate) fn match_names(req: &NamesRequest, stop: &dyn Fn() -> bool) -> Option<NameHits>;     // branches, tags, files
pub fn first_hits(store: &RepoStore, query: &Query, limit: usize, stop: &dyn Fn() -> bool)
    -> Result<Vec<CommitHit>, String>;                                                           // graft::search, `/`'s walk
// graft-git::status: `tracked_paths(repo, stop) -> Result<TrackedPaths, StatusError>` (12b); BRANCHES read
// `graft_git::switch_refs` (unchanged) on a job.

// graft-ops::tour (G-tour) — never touches his repo; TourTask is pub (graft words the card)
pub struct PracticeRepo { pub root: PathBuf, pub work: PathBuf, pub remote: PathBuf }
pub fn build_practice_repo(under: &Path) -> Result<PracticeRepo, TourError>;
pub fn remove_practice_repo(repo: &PracticeRepo) -> Result<(), TourError>;
pub enum TourTask { StageHunk, Commit, Undo, StashAndBack, Branch, MergeWithPreview, ResolveConflict, Push }
pub fn tasks_done(entries: &[JournalEntry]) -> [bool; 8];

// graft-ops::clone (G-first-run) — not journaled
pub fn clone(params: &CloneParams, hooks: RemoteHooks, stop: &StopFlag) -> Result<PathBuf, CloneError>;

// graft::cli (G-docs) — man_page gains the config table in 2b
pub struct Flag { pub long: &'static str, pub short: Option<char>, pub value: Option<&'static str>,
    pub help: &'static str, pub hidden: bool }
pub const FLAGS: &[Flag];
pub fn man_page(flags: &[Flag], keys: &[Binding], config: &[ConfigKeyDoc]) -> String;

// graft-ops::rename (G-rename)
pub enum OnDiskNames { New, Old }
pub fn migrate_repo(ctx: &Context) -> Result<Migrated, (OnDiskNames, MigrateError)>;
```

Callers: `event_loop::pump` owns `Pacer`, `Rest` and `FrameTrace`, and waits on the earlier of `Pacer::deadline` and
`Rest::deadline`. `update::settle` gates the detail follow and the evil probe on `defer_follow`/`follow_owed`, and
`AppEvent::Rested` runs them. Thread emphasis stays per-frame paint and is not gated (G-settle Out). The graph view
builds `HeaderView`. Every geometry mirror (`update::geometry`, mouse row math, `pane_boundaries`) reads `ScreenRows`.
Row painting calls `legible_on` for any fg drawn on `selection`. Startup draws `EmptyState` when discovery finds no
repo. `update` calls `Keymap::translate` once before any `*_key` dispatcher. Every surface that prints a key calls
`Keymap::spell`. Repository open calls `migrate_repo` before any journal read.

## Hard-to-reverse choices (decided)

1. Fence-paced frames — ADR-0054.
2. Keymap translate-at-the-edge; `ActionId` is his config's API — ADR-0055. A golden id list guards it.
3. The name reaches disk; migrate once — ADR-0057. Old ref names mapped at read; nothing old deleted before read-back.
4. The cursor-row fill excludes the graph column — ADR-0058 §2. Undo cost: every cursor-row capture and paint test.
5. (2b) **Crate and package names stay `graft-*`** in M8: unpublished (ADR-0057), and renaming them breaks every
   `crates/graft/…:line` citation in docs and gaps. Rename before the source is published (a gap row).
6. (2c, **approved 2026-10-02**, he said *move*) The rewrite todo gains `label`/`reset` for fork sections — ADR-0059.
   Options: one rebase with sections / a second rebase in the entry / a merge-tree replay. **One rebase**: git's stops,
   abort and kill -9 state stay the only truth (ADR-0047's reason). Undo cost: the saved-plan format (`plan.json`).

## Where I'd differ from DESIGN.md

Pass-1 Proposed 1–5 approved 2026-10-02 and applied. **PROMPT.md still to edit** (handed to the conductor).
2b, **resolved 2026-10-02** (field notes :89, item 1, "go ahead with your recommendation"): keymap §2.2 `q` row
"confirms if an operation is in progress" read as *a write is running*; a stopped git operation quits without a box
(relaunch offers continue or abort, PROMPT §9). `q` during a write that can't stop waits for it, then quits (seen, not
overruled). Applied: keymap §2.2 and §2.6a `q` rows, dated 2026-10-02.
2c, **resolved 2026-10-02** (field notes :89, item 5, "go ahead with your recommendation": he said *move*): **a branch
started from one of the commits he rewrites moves onto the new commit when it can without conflict**, shown in the
preview first (ADR-0059, accepted). Replaces m7/37's `left behind`. Trade-off as weighed: no forgotten fork and no hand
rebase afterwards, vs. a branch he didn't name moving (shown before `y`, undone by one `u`; a conflicting, pushed or
elsewhere-checked-out one stays, in words). Specs `fork-follows-engine.md` and `fork-follows-preview-words.md` stand.

Palette find, proposed 2026-10-07, **approved 2026-10-07** (field notes :92, items 2 and 3, "your recommendation") —
ADR-0061; applied: keymap §2.12 `Ctrl+P` line and History, view mock-ups §1.6 (drawing's recent row, M8 paragraph).
The "Declined →" branches below and in specs 12/12b are not built.
- **P-find-1. The empty `⌃P` box.** §2.12 opens it on "actions, then branches, tags, files, commits, then recently run
  commands". Proposed: empty → `COMMANDS RUN RECENTLY` first, then `ACTIONS`; branches, tags, files and commits only
  once he types; the empty field reads, muted, `type to find actions, branches, tags, files, commits`. Why: JetBrains
  opens on recent (`palette-find` "JetBrains"); UX §1.4 order by likelihood; J31 2 keys, not 3; eight branches, tags,
  files and commits nobody asked for are noise on an empty box (H8). Trade-off: the box no longer *shows* that refs
  are findable — the placeholder *says* it (H6); `⌃P ⏎` reruns the newest command (it runs as its key would, preview
  and all). Declined → §2.12's order on an empty box, find groups showing their first rows for the empty query (the
  same jobs), J31 = 3.
- **P-find-2. The recent group's rows.** Header stays §1.6's `COMMANDS RUN RECENTLY` (not spec 12's earlier
  `RECENT`: §1.15 already calls the repo list `RECENT`, and the palette opens over it — one word, one meaning, H4).
  §1.6's sample row `git rebase main` (a command with its argument) is **not** built: a row is the action row as the
  M8 paragraph of §1.6 draws it (key, label, config id), and `⏎` asks the argument again. Why: §2.12's `Enter` runs
  "as its key would"; a stored argument makes a rerun a different action from the key's. Trade-off: rebase onto main
  again is `⌃P ⏎` and pick `main`, not `⌃P ⏎`.

Pixel ghosts (17b, spec-only pass 2026-10-07), **approved 2026-10-07** (field notes :92, item 1) — ADR-0060; applied:
view mock-ups §1.7. While a Ghost Preview is open at the Pixel tier the whole graph column draws at the Unicode tier;
pixel dashed ghost strokes move past v1. Departs §1.7's "dashed stroke in the real renderer". Why: strips are 32-row
bands, so text ghost rows inside them would draw a lane half pixel, half text — two pictures, which §1.7 rules out.
Trade-off: no pixel curves while a preview is open; strips return on close, no re-raster.

Look polish, **post hoc** (13l box C, 2026-10-08), **proposed — not decided, not in the build.** Prior art
(`m8-look-polish.md`) supports or is silent on all five shipped choices; it argues against none. Two questions:
- **P-look-1 (question). Does the whitespace choice outlive the run?** GitHub users ask "Hide whitespace" to persist
  (#65597, title only); lazygit keeps it in config (`git.ignoreWhitespaceInDiffView`). Ours: each screen's default is
  fixed (detail `config/types.rs:110`, history `update/place/history/open.rs:41`, blame `graft-git blame.rs:77` — lines
  from 17b's scout). Whether `w` writes anything back, and whether a `[diff]` key already sets detail's default, is
  **unverified** here (no source read). If neither: proposed, a `[diff] ignore_whitespace` key seeding detail and
  history, `w` per run as today; the tail rule (off-default only) then reads against the configured default. Trade-off:
  one more config key vs. pressing `w` every run on a repo full of reformat commits. No field note asks yet — his
  daily use decides; nothing filed until it does.
- **P-look-2 (question). `ui.overview` default.** Shipped off. The study couldn't read Araxis's or VS Code's defaults
  (§4: "question"). No change proposed until a source says what the best tools default to; the off default costs
  nothing to reverse (one config default, one capture set).

## New UI

- Chrome and cursor row: DESIGN §1.0. Empty state: §1.15. `?`: §1.16. Palette rows: §1.6.
- **First-launch line**: status row, muted: `first time? ? keys here · ⌃P anything · ⌃P "tour" to practise`; gone
  after `?`/`⌃P` once or 5 launches; a problem line wins.
- **Tour card**: top of the graph pane, 3 rows, warning border; header `◇ practice · <tmp>`.
  `╭ practice 3 of 8 · undo that commit ──── esc leave ╮` / `│ u takes back the last thing you did — nothing is lost
  ✓ stage ✓ commit · undo │`. Ticks by doing; < 100 cols drops the ticks; claims no key but `Esc`.
- **Palette sources** (`⌃P`): branches/tags/commits `⏎` seek the cursor (never switch — `C` does); files open history
  (`F`); `COMMANDS RUN RECENTLY` = palette actions run this session (P-find-2; was `RECENT`). ≤ 8 rows per group,
  `+N more`. Empty box: P-find-1. A source still reading draws one muted `reading…` / `searching…` row in its group.
- Bar doorway becomes `? keys`; `.` stays bound as an ordinary candidate `. actions`.
- (2c) **Fork rows** in every rewrite/rebase preview and the rewrite editor's right pane: see "Pass 2c".

## Genuinely risky tickets

- **G-pace** — Fable (signature): approach decided here; a wrong loop contract is structural; smoothness on his
  terminals can't be judged from diff or fixtures. Passes all three.
- **G-keymap 1** (`keymap-table-and-translate`) — Fable (signature): every later surface builds on it.
- Risky, not Fable: rename migration (Opus, alone); tour repo builder (path guard); look-chrome (row split moves every
  geometry mirror); askpass peer check (hygiene-code item 1, Opus).
- **Fable count: 2** (+ acceptance + the standing architecture pass).

## Expected groups — final order

| # | Group | Crates | Model | Spec |
|---|---|---|---|---|
| 1 | G-trace | graft event_loop, scripts | Opus | `01-trace-product-frame-trace.md` |
| 2 | G-pace | graft-term, graft-core caps, graft event_loop | **Fable (sig)** | `pace-fence-and-sync.md` |
| 3 | G-settle | graft update, jobs | Opus | `settle-work-at-rest.md` |
| 3b | G-trace follow-up (after his run) | graft view/update | Opus | `trace-followup-per-turn-cost.md` |
| 4 | G-pixel-mem | graft-graph, graft-term kitty, graft graphics | Opus | `pixel-memory-budget.md` |
| 5a | G-look chrome | graft-ui graph_screen + hints, graft view | Opus | `look-chrome-header-gutter-bar.md` |
| 5b | G-look cursor row + dark B | graft-ui theme + graph rows | Opus | `look-cursor-row-and-dark-b.md` |
| 5c | G-look outside repo | graft-ui empty_state, graft startup + recent, graft-ops init | Opus | `look-outside-repo-screen.md` |
| 6a | G-keymap table | graft-ui keymap, graft config + update entry | **Fable (sig)** | `keymap-table-and-translate.md` |
| 6b | G-keymap spell | graft-ui hints + labels, graft words | Opus | `keymap-spell-from-the-table.md` |
| 7a | G-palette help | graft-ui palette, graft update | Opus | `palette-help-and-actions.md` |
| 7b | G-palette find | graft update::palette, jobs, search; graft-git status (12b) | Opus | `palette-find-anything.md`, `palette-find-commits-and-files.md` |
| 8 | G-bar | graft-ui hints, graft view | Opus | `bar-content-pass.md` |
| 9a | G-theme | graft-ui theme, graft config + session | Opus | `theme-overrides.md` |
| 9b | G-look polish | graft-ui theme + rows, graft view/config | Opus | `look-polish-rows.md` |
| 10 | G-first-run | graft-ops clone, graft startup, graft-ui empty_state | Opus | `first-run-clone-and-first-launch.md` |
| 11 | G-tour | graft-ops tour, graft, graft-ui | Opus | `tour-practice-repo.md` |
| 12a | G-daily landing | graft update operate/undo | Opus | `daily-landing.md` |
| 12b | G-daily ops | graft-ops rewrite, graft-git, graft preview | Opus | `daily-ops-fixes.md` |
| 12c | G-daily fork (2c) | graft-ops rewrite + preview | (after scout) | `fork-follows-engine.md` |
| 12d | G-daily fork words (2c) | graft preview line rows, rewrite editor, banner | (after scout) | `fork-follows-preview-words.md` |
| 13 | G-words | graft-ui, graft, graft-ops (words) | Sonnet | `words-sweep.md` |
| 14a | G-hygiene code | graft-git, graft-ops raw, askpass | Opus/Sonnet | `hygiene-code.md` |
| 14b | G-hygiene scripts | scripts, fixtures, fuzz, shots | Sonnet | `hygiene-scripts-and-fixtures.md` |
| 15 | G-docs | graft cli, README, man | Opus | `docs-readme-and-man.md` |
| 16 | G-release | dist, CI (not pushed), scripts | Opus | `release-dist-and-matrix.md` |
| 17 | arch pass → G-health | all | Fable review → Opus/Sonnet | `fable-architecture-pass.md` (standing) |
| 18a | G-rename migration | graft-ops, graft, graft-git | Opus, alone | `rename-on-disk-migration.md` |
| 18b | G-rename sweep | all (names) | Sonnet, alone | `rename-names-sweep.md` |
| 19 | G-accept | — | Fable | `accept-m8.md` (+ `28-amendment.md`, 2026-10-08) |

Lanes: 1→2→3→3b; 4 after 2. 5a→5b; 5c after 5b (re-specced 2026-10-03: shares `graft-ui` `lib.rs`, MAP.md and the
`selection` paint with 5b). 6a→6b→7a→7b; 8 after 7a and 12b. 9a after 5b and 6b (re-specced 2026-10-04: shares graft
`config` files with 6b — serial); 9b after 9a.
10 after 5c and 7a; 11 after 10 and 6b. 12a, 12b, 14a, 14b beside the scroll lane (disjoint files; conductor checks
scouts). 12c → 12d, after 12b, before 17. 13 after 6b and 12b. 15 after 6a, 9a, 10, 11; 16 after 15. 17 after every
build; 18a → 18b → 19.
**Nothing is published or registered** (ADR-0057).

## Pass 2b — specced

All items of "Pass 2b remaining" are specced (table above). Still owed by others: G-health specs (from the
architecture review); his trace run (gates 3b); scene 1's feel.

## Pass 2c — spec-only, 2026-10-02 (ticket 14's two questions)

- **J19 = 4** (`a` edit `⏎` `y`, a branch built on HEAD). Build unchanged; the walk's "3 + the preview's `y`"
  allowance in `reword_head_with_a_line_built_on_it` should assert the fence's 4 directly (conductor: a one-line
  follow-up in ticket 14 or `accept-m8`).
- **Fork follows — approved 2026-10-02** (above; he said *move*):
  - **Problem**: "mid-chain branch left behind after `a`" (:86). **User**: him, amending or rewording on a line
    teammates' work hangs off. **Solution**: no extra step — the fork rides the same `y`.
  - **Journey**: `a` into an older commit (cursor on it, something staged), `y` → 2+1, fork included; jj 0 extra
    (`stacked-branches` §1.4), branchless +1 (`git restack`, §1.3). Today: + the fork's own rebase by hand (not
    counted here). Lands as the rewrite does today (UX §3.1); a stop inside a fork's section lands on M6's conflict
    screen, banner `feature-x, forked at a1b2c3d`.
  - **Fork rows** (UX §3.6): `feature-x follows · 2 commits from a1b2c3d` in the dependents' look; `feature-x stays on
    a1b2c3d · would conflict in src/a.rs` (warning colour) or `· pushed to origin/feature-x`, `· checked out in
    <path>`, `· has a merge`, `· the default branch`, `· not checked: <why>` (muted). Follows first, then stays,
    oldest fork point first; `+N more stay`. No new key.
  - Fable: neither spec meets all three tests — `fork-follows-engine` is structural (todo vocabulary), but its
    correctness is judged from git-truth tests and the oracle. Opus after scout is the expectation; conductor's call.
