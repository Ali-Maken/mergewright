# Scroll smoothness (wheel, trackpad, held key) — prior art

Builds on [`big-repo-scroll.md`](big-repo-scroll.md) (2026-10-01: loading/paging, commit-graph, drain-then-draw). Not repeated here.

Need (field notes 2026-10-02): 17,694 commits, 1,323 refs, Ghostty full screen, Kitty graphics tier. Lag on held `j` **and** wheel/trackpad. "VS Code never lagged, neither did Sublime Merge."

Rule: facts carry a link. **Guess** / **Question** = not confirmed. Researched 2026-10-02.

## Summary

- The smooth GUIs separate three things: **data** (paged), **layout** (row positions known up front), **paint** (only visible rows, at most once per display frame). VS Code's list widget is the clearest public example.
- **No terminal tool found coalesces wheel events.** lazygit and zellij handle each wheel event on its own; they scroll N rows per event instead. Coalescing would be ours (**invent**, from the ratatui frame-rate recipe already cited in big-repo-scroll).
- Ghostty already scales events: trackpad (precision) ×1, wheel (discrete) ×3. So a trackpad sends many 1-step events.
- Kitty graphics: moving a placement by re-sending the same image id + placement id is flicker-free and needs no retransmission. Unicode placeholders make images move with text for free.
- Synchronized output (DEC mode 2026) makes a frame paint atomically; Ghostty and Kitty support it.

## 1. GUIs: VS Code and Sublime Merge

### VS Code list widget (used by its trees and lists)
- `ListView` renders only the visible range: `rangeMap` holds every item's position; on scroll, `render()` computes the range from `renderTop` + `renderHeight` and inserts/removes only the changed rows. DOM rows are pooled in `rowCache` and reused — [listView.ts](https://github.com/microsoft/vscode/blob/main/src/vs/base/browser/ui/list/listView.ts).
- Scroll is applied via a `Scrollable` scheduled with `scheduleAtNextAnimationFrame` — so many scroll events in one frame give one render. Same file (summary via fetch; exact lines not quoted — **Question:** confirm line numbers before citing in a spec).
- **Question:** whether the SCM Graph view uses this `ListView` (via the workbench tree) — likely, not read.

### VS Code Source Control Graph
- Paged: `scm.graph.pageSize` sets commits loaded initially and per "load more" — [VS Code docs: source control history](https://code.visualstudio.com/docs/sourcecontrol/history).
- **Complaint:** "the source control panel becomes unresponsive" with thousands of commits — [vscode#227475](https://github.com/microsoft/vscode/issues/227475). Fix not visible on the page fetched. **Question:** what fixed it.

### Git Graph / GitLens
- Git Graph pages ("Initial Load", "Load More Automatically") — see big-repo-scroll. Whether its webview virtualises rows: **Question**, not read.
- GitLens Commit Graph slowness complaint — see big-repo-scroll (#4585).

### Sublime Merge
- Closed source. Nothing new beyond big-repo-scroll (own Git reader, Sublime Text UI engine). Virtualisation / frame throttling: **Question**, nothing public.

## 2. TUIs and wheel/trackpad bursts

- **lazygit:** `HandleScrollUp/Down` on `MouseWheelUp/Down` scrolls by `gui.scrollHeight` (default 2) per event; no batching seen — [list_controller.go](https://github.com/jesseduffield/lazygit/blob/master/pkg/gui/controllers/list_controller.go), [Config.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md).
- **zellij:** each mouse event dispatched immediately, no accumulation or throttling — [input_handler.rs](https://github.com/zellij-org/zellij/blob/main/zellij-client/src/input_handler.rs).
- **Ghostty:** `mouse-scroll-multiplier` with `precision:` / `discrete:` prefixes; defaults precision 1, discrete 3 — [Ghostty config reference](https://ghostty.org/docs/config/reference).
- **crossterm:** `poll(Duration::ZERO)` "must return immediately"; `Ok(true)` guarantees the next `read` won't block — [crossterm event.rs](https://github.com/crossterm-rs/crossterm/blob/master/src/event.rs). Whether it checks the internal buffer first: docs don't say. **Question** (still open from big-repo-scroll; read `src/event/read.rs`).
- gitui, tig, helix, bottom/btop wheel handling: **not read this pass.** **Question.** (helix `scroll_lines` option: fetch did not surface it.)

## 3. Terminal graphics while scrolling

From the [Kitty graphics protocol spec](https://sw.kovidgoyal.net/kitty/graphics-protocol/):
- Same image id + placement id sent again "will replace the first. This can be used to resize or move placements around the screen, without flicker." Image data stays; no retransmit.
- `d=p` (lower case) deletes placements, keeps image data. Upper case also frees data once unreferenced.
- **Unicode placeholders** (`U+10EEEE`, virtual placement `U=1`, kitty ≥0.28): image cells are text, so images "move around as needed when they redraw their screens". Yazi uses them on Kitty and Ghostty — [superfile#1665](https://github.com/yorukot/superfile/pull/1665) (note: WezTerm accepts graphics but does not render placeholders).
- Local clients should use file / shared-memory transmission rather than inline base64. Same spec.
- Image-heavy TUIs' complaints are about **slow first display**, not scroll: yazi + kitty + tmux 15–20 s — [yazi#3102](https://github.com/sxyazi/yazi/discussions/3102); yazi `image_delay` — [yazi#2363](https://github.com/sxyazi/yazi/issues/2363).
- **Synchronized output** (`CSI ?2026h` … `l`): terminal holds paint until the frame ends; query with `CSI ?2026$p` — [spec gist](https://gist.github.com/christianparpart/d8a62cc1ab659194337d73e399004036). Supported in Kitty, Ghostty, WezTerm, iTerm2, Alacritty — [otty terminal comparison](https://docs.otty.sh/vt/terminal-comparison) (secondary). VS Code's terminal: requested — [vscode#284592](https://github.com/microsoft/vscode/issues/284592). **Question:** does Graft already wrap frames in 2026?
- timg, chafa, termpdf: not read. **Question.**

## 4. Praise / complaints

- VS Code SCM graph unresponsive at thousands of commits — [#227475](https://github.com/microsoft/vscode/issues/227475).
- Sublime Merge praise / tree slowness — see big-repo-scroll.
- No scroll-smoothness complaints found for lazygit/zellij specifically this pass. **Question.**

## Take / reject / invent candidates

- **Take** (VS Code ListView): row positions precomputed once; paint only the visible window; reuse row buffers/strips across frames.
- **Take** (VS Code `scheduleAtNextAnimationFrame`): scroll events only move an offset; one paint per frame.
- **Invent** (none of lazygit/zellij do it): sum all wheel deltas pending in one drain into one offset change. Keep N-rows-per-event (lazygit) as the step; Ghostty already multiplies discrete wheels ×3.
- **Take** (Kitty spec): move graphics by re-placing same id/placement id; never retransmit on scroll; `d=p` not data delete. Consider Unicode placeholders so images scroll as text (**Question**: fits Graft's strip design?).
- **Take**: DEC 2026 synchronized output around each frame (Ghostty/Kitty support).
- **Reject**: per-event immediate redraw (zellij-style) for the history pane.

## Addendum (2026-10-03): Per-row work while the cursor moves

Fixes review M2 (`docs/reviews/m8-03-settle.md:125-137`). Question: while a movement key is held, what does each tool do with the detail/preview work for the row under the cursor?

| Tool | What it does | Timings | Source |
|---|---|---|---|
| **lazygit** | Every cursor move starts the new diff command at once (leading edge, no debounce). `ViewBufferManager.NewTask` closes the previous task's `stop` channel and kills its process; a task whose id is older than the newest queued exits without touching the view. If starting a command took longer than a threshold (machine under load), the *next* spawn is throttled. "loading..." shown only if output is slow and differs from what is on screen. | `THROTTLE_TIME = 30 ms`, `COMMAND_START_THRESHOLD = 10 ms`, loading after 200 ms | [pkg/tasks/tasks.go](https://github.com/jesseduffield/lazygit/blob/master/pkg/tasks/tasks.go) (comment: "if the user flicks through a heap of items, with each one spawning a process … we want to throttle the spawning of processes") |
| **fzf `--preview`** | Each focus change cancels the running preview (`refreshPreview` → `cancelPreview`) and enqueues the new one immediately; no debounce. Cancel is graceful: the old command gets up to 500 ms to finish, killed at once if it has already rendered partial output. Old preview stays on screen; after 500 ms a "Loading .." badge appears. Output is painted in 100 ms chunks. | `previewCancelWait = 500 ms`, `previewDelayed = 500 ms`, `previewChunkDelay = 100 ms` | [src/constants.go:26-28](https://github.com/junegunn/fzf/blob/master/src/constants.go), [src/terminal.go](https://github.com/junegunn/fzf/blob/master/src/terminal.go) (`cancelPreview`, `printPreviewDelayed`, goroutine 3 "responsible for cancelling running preview command") |
| **tig** | The diff view does not follow the main-view cursor by default; the user moves with up/down *in the diff view*, which moves the parent cursor and reloads the diff synchronously. No debounce or cancel found in docs. | none documented | [manual](https://jonas.github.io/tig/doc/manual.html); request to auto-follow: [#588](https://github.com/jonas/tig/issues/588); slowness: [#324](https://github.com/jonas/tig/issues/324), [#253](https://github.com/jonas/tig/issues/253); 2.1 deferred `git describe` until after the diff loads ([NEWS](https://jonas.github.io/tig/NEWS.html)) |
| **VS Code Timeline** | Not cursor-driven (follows active editor), but the pattern: a new request cancels the pending one's `CancellationTokenSource`; if items are already visible and other requests are pending, the refresh is **trailing-debounced**. | `@debounce(500)` on `refreshDebounced` | [timelinePane.ts](https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/timeline/browser/timelinePane.ts) (`tokenSource.cancel()`, `@debounce(500)`) |
| **GitLens Commit Details** | Service API takes an `AbortSignal` for "cooperative cancellation" of `getCommit`. Whether selection changes are debounced was **not confirmed** (webview file not read in full). | ? | [commitDetailsService.ts:105-107](https://github.com/gitkraken/vscode-gitlens/blob/main/src/webviews/commitDetails/commitDetailsService.ts) |
| **Sublime Merge** | Closed source; no documentation found on preview deferral. **Question**, not finding. | ? | — |

**Praise / complaints.** lazygit: "Scrolling through big commits really slow" ([#435](https://github.com/jesseduffield/lazygit/issues/435)); "Jittering and laggy scrolling" ([#3504](https://github.com/jesseduffield/lazygit/issues/3504)); lag "more apparent if you hold j or k" ([LazyVim #3194](https://github.com/LazyVim/LazyVim/issues/3194)); fix direction was streaming output instead of loading whole ([PR #590](https://github.com/jesseduffield/lazygit/pull/590)). tig: users want the diff to follow the cursor (#588) and complain of hangs on large commits (#324). No praise for the debounce approach found specifically; fzf's always-responsive preview is not cited here as praised (not searched).

**Verdict for ticket 03.** Found: *leading edge + cancel-on-move* (lazygit, fzf), *trailing debounce 500 ms + cancel* (VS Code Timeline), *load-shed throttle when the machine is slow* (lazygit 30 ms). Ticket 03's rule — leading edge follows, burst defers, settle 150 ms / 50 ms, cancel on move — combines lazygit/fzf's leading edge and cancel with a trailing settle during a burst; no single tool found does that combination, and none uses 150/50 ms. So the row at `docs/design-notes/m8-polish-and-ship.md:41` should read **T (lazygit, fzf) for leading edge + cancel-on-move · I for burst-defer and the 150/50 ms settle** — "nothing fits, because lazygit/fzf never defer (they spawn per move, throttling only under load) and VS Code's 500 ms trailing debounce is too slow for a held key at repeat rate; the numbers come from key-repeat rates, not prior art." Not "T (extended)" on §1.
