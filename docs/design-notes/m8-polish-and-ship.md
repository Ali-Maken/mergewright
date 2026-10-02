# M8: polish, final name and ship — pass 1 (pipeline steps 1–4 + prototype brief), 2026-10-02

**Status: pass 1.** No ticket specs. Pass 2 writes them after he has tried the prototype (step 5) and the scouts
below have answered, same as M7. Inputs: PROMPT.md §8 M8 row, §5, §9; research `scroll-smoothness.md`,
`big-repo-scroll.md`, `themes.md`, `keymap-help-remap.md`, `first-run.md`, `install-docs.md`; `docs/naming-research.md`;
field notes :46, :48–50, :82–86; KNOWN_GAPS rows naming M8; MAP.md of graft, graft-ui, graft-term. No source read.

**Promise (M8):** *"It stays smooth on your biggest repo, installs with one command, and teaches itself from inside the
app."* The acceptance walk proves each half: held-key and wheel tails on MONOREPO, every journey below at or under its
count, `brew install` → run on a clean account, the tour finished with no shell.

**Order: scroll first** (his first item, field notes :85). Nothing else starts in the graft `event_loop` until the
pacing group lands.

## Need

| What | His words / evidence | Weight |
|---|---|---|
| Scroll on his work repo (17,694 commits, 1,323 refs, Ghostty full screen, Pixel tier) | "i stopped pressing and it kept going" (:82); after 41–48 "still a bit laggy" (:84); "not just j but scroll as well … vscode never lagged neither did sublime merge … up to you" (:85) | daily, every minute |
| Learn the app | "He did not know dragging existed … I'll definitely need a tutorial when the architecture is finished" (:46; KNOWN_GAPS M5/user) | once, then rare |
| Keys he can't remember | usability walk: "`?` does nothing", `⌃P` and `:` too (KNOWN_GAPS M6/u2, still open); PROMPT §9: a new user does the 7 daily tasks "using only the hint bar and the command palette" | daily |
| Readable in his theme | cursor row unreadable in dark, twice (:44, :48); fixed at 3.04:1 (:50); text on it 4.28:1, short of 4.5 (M5/44) | daily, Zed + VS Code + Ghostty, dark |
| Install like a real tool | today he runs a build from this repo; M8 row: `cargo install`, tap, binaries, man page | once |
| Two daily rulings | "pull with nothing new → cursor stays where he was; `a` on HEAD with nothing staged → open the message editor (reword)" (:86) | daily |

## Prior art (T take · R reject · I invent)

| Aspect | Best answer (study) | Ours | |
|---|---|---|---|
| Paint only visible rows, positions known up front | VS Code ListView (`scroll-smoothness` §1) | already: row cache + strip cache (DESIGN §3.6, §3.8) | T (have) |
| One paint per display frame however many events | VS Code `scheduleAtNextAnimationFrame` (§1); ratatui frame-rate recipe (`big-repo-scroll` "Terminal techniques") | a turn's wakes share one frame (m7/43, graft MAP `joins_frame`) — kept | T (have) |
| **Never let the terminal fall behind** | nothing found: no studied TUI paces its output on the terminal | each frame ends with a DSR fence (`CSI 5 n`); the next frame waits for its answer `CSI 0 n` (or a timeout), and every input in between lands in state, not in the pipe. Bytes waiting in the pipe are then bounded by one frame, on any terminal, deflate or not | **I** — the studies cover input batching (done, m7/43), not output pacing. Whether his residual lag is output backlog is unknown until the trace (G-trace); the fence bounds it either way |
| Atomic frames | DEC 2026, Kitty/Ghostty/WezTerm/iTerm2 (§3) | wrap text **and** image commands of each frame; DESIGN §4.1 already says emit unconditionally — scout confirms it's built | T |
| Wheel bursts | lazygit 2 rows/event, zellij per event, no batching (§2); Ghostty ×1 trackpad, ×3 wheel | wheel events in one turn sum into one scroll; per-event step unchanged unless the prototype says otherwise | T step · I sum |
| Per-row work while moving | VS Code: scroll only moves an offset (§1) | detail follow, radar probe, row emphasis jobs start when the cursor **rests** (a frame drawn with no motion queued); stale ones cancelled | T (extended) |
| Kitty uploads | same id re-place, `d=p`, no retransmit (§3) | have since m7/46 | T (have) |
| Kitty Unicode placeholders | yazi (§3); WezTerm doesn't render them | not in M8: strip design differs, pacing covers the symptom | R |
| Measure where he is, not on fixtures | gitui #2823 (per-tick cost hid until a big repo) | an opt-in frame trace in the shipped binary: numbers only, no names or shas | I (rule 1: green fixture gates did not match his screen, :84) |
| Theme auto + pin | zellij `theme_dark/light` + `explicit_theme_hue`, bat pair (`themes.md`) | have: OSC 11 + `ui.theme`, re-query on focus (DESIGN §4.5) | T (have) |
| Switch theme now | zellij ToggleTheme, helix `:theme`, VS Code ⌘K⌘T | palette rows `Theme: dark / light / high contrast` (this run). No key: keymap allocates none and it's rare (§2.1 "rare verbs behind the palette") | T |
| Change one colour | gitui 0.23 partial `theme.ron`, helix `inherits`, VS Code `colorCustomizations` | `[theme.dark] selection = "#…"`, token names as PROMPT §5 spells them | T |
| Contrast floor | nobody checks; lazygit #4306/#508, gitui #922 are this bug | every theme tested against fixed floors; a user override that misses one is kept and named in a warning | **I** (we shipped the bug once, :48) |
| Per-widget attribute lists; file-only themes; palette-inheriting default | lazygit; gitui; bat `ansi` | — | R (the first two caused the complaints; the third loses the `selection` floor) |
| `?` = live keys for here, grouped | lazygit `?` menu; #3141 "contextual to the current mode" (`keymap-help-remap.md`) | `?` lists the focused place's keys, an **operation in progress** group first when one is | T |
| Search by name **and** by key | VS Code shortcuts editor, JetBrains Find by Shortcut | typing filters names; a typed key that is bound ranks its row first | T |
| Help drawn from the live table | which-key "display instantly" | hint bar, `?`, palette, `.` labels, tour and man page all spell keys from one table | T |
| Remap collisions | JetBrains warns at assign; gitui #2704, lazygit #5763 silent | checked at load, both actions named, defaults included | T |
| Unbind | tig #816 can't clear a key | `"none"` unbinds everywhere | T |
| Config name beside each key | nobody | `?` shows `graph.merge` beside `m merge`, so rebinding needs no docs | **I** |
| In-app key recorder | JetBrains, VS Code | file only | R (heavy for a TUI; he hasn't asked) |
| which-key delay popup | emacs which-key | Graft has almost no prefixes (§2.1) | R |
| First launch | lazygit versioned one-shot popup | one status-row line, gone after he uses `?` or `⌃P` once or after 5 launches; never a popup | T (shrunk) |
| Tour form | helix/vimtutor copy; JetBrains learning project; VS Code walkthrough ticks on doing | a **practice repo** Graft builds in a temp dir; 8 tasks done for real, each ticks when done; Esc leaves to where he was | T |
| Tour shows his keys | helix #13483 complaint | tour words spelled from the keymap | T |
| Exit always visible | zellij #5064 trap | `esc leave` on the tour card, every step | T |
| Undo as the tour's hook | GitUp sells undo, no tour found | task 3: "undo that commit" | **I** |
| Tips, tip of the day, wizard, video | lazygit, zellij #1730, JetBrains, zellij wizard | — | R |
| Empty state | Sublime Merge opens on clone/new | DESIGN §1.15 Open / Init / Clone / Recent | T (designed, unbuilt per MAP) |
| Binaries, tap, installer | dist v0.33 maintained (`install-docs.md` §2) | dist: 4 targets, tap formula, curl installer | T |
| homebrew-core | all ten studied tools are in core | later: needs the public name and a source build | R (now) |
| Man page | ripgrep `--generate man`, jj `util completion` | generated from our own CLI table + keymap; committed; a test fails when stale | T |
| clap_mangen | — | would mean adopting clap; our parser is hand-written (`install-docs` §5) | R |
| Shell completions | ripgrep, jj | two flags; nothing to complete | R |
| Notarisation | dist doesn't (`sign/macos.rs:21`) | default: none; brew and the curl installer aren't quarantined (Guess, `install-docs` §4) | **his call** (money) |
| Update notifier | none studied; Homebrew discourages | — | R |

## Journeys (steps = keys + decisions, UX §1.3; start: his repo, graph, cursor on HEAD)

| # | Task | Graft | Steps | Best tool |
|---|---|---|---|---|
| 1 | Hold `j` ~300 rows, let go | hold `j` | 1+0; stops within the held_key budget (100 ms) of release | VS Code / Sublime Merge: stop on release |
| 2 | Trackpad / wheel flick through history | gesture | 1+0; stops when fingers stop | same |
| 3 | Jump to the oldest commit | `G` | 1+0; ≤ 16 ms after layout (`g_after_idle`) | same |
| 4 | Find the key for an action he knows by name | `?` type name | 2+1 | lazygit 2–3, VS Code 3 |
| 5 | What does key `x` do here | `?` `x` | 2+1 | JetBrains Find by Shortcut ~3 |
| 6 | Run an action he can't recall the key for | `⌃P` name `⏎` | 3+1 | VS Code 3 |
| 7 | Rebase stopped; "how do I go on?" | `?` | 1+1 (operation group on top) | lazygit: not answered (#3141) |
| 8 | Rebind an action | `?` name → read config name, edit 1 line, restart, `?` | 5+1 | VS Code 5, JetBrains ~7 |
| 9 | Light theme for this run | `⌃P` `theme` `⏎` | 3+1; auto = 0 | VS Code 3, zellij 1 |
| 10 | Fix one unreadable colour | edit 1 line, restart | 2+1 | VS Code 2+, lazygit 2 |
| 11 | First launch in his repo | — | 0 extra | lazygit 1 (dismiss) |
| 12 | Start the tour | `⌃P` `tour` `⏎` (empty state: `t`) | 3 (1) | helix `:tutor` 2 |
| 13 | Leave the tour | `Esc` | 1; back in his repo, same cursor | zellij #5064: trapped |
| 14 | Open a recent repo, outside any repo | `graft`, `j`, `⏎` | 3 | Sublime Merge 2+ |
| 15 | Clone | `c` URL `⏎` | 3+1 | Sublime Merge ~4 |
| 16 | Install on his Mac | `brew install <owner>/tap/<name>`, `<name>` | 2 | homebrew-core 2 |
| 17 | Read the manual | `man <name>` | 1 | ripgrep 1 |
| 18 | Pull, nothing new | `p` | 1; cursor stays (his ruling) | lazygit 1 |
| 19 | Reword HEAD, nothing staged | `a`, edit, `⏎` | 3 | lazygit `r` 2–3 |
| 20 | Undo, see what came back | `u` | 1; lands on the result (u-m2) | GitUp 1 |

Tour tasks are PROMPT §9's list plus undo; each keeps the count already in the M3–M6 notes. Pass 2 writes the
`journey-counts` fence with these ids once the prototype settles them.

**Where he lands** (UX §3.1):

| Event | Lands |
|---|---|
| Key released / fingers off | the frame with the last applied input; nothing after it |
| Startup with `[keys]` or `[theme]` problems | graph as normal; status row `config: 2 problems · ? shows them` |
| Tour step done | card ticks, next task's words; cursor where the task left it |
| Tour finished or `Esc` | his repo, cursor where he left it, `practice done ✓ · ⌃P tour goes again` (nothing on `Esc`) |
| Tour repo can't be built (no git, disk) | stays in his repo; status `tour: <reason>`; nothing written in his repo |
| Clone fails (auth, URL, exists) | prompt stays open, URL kept, reason + `⏎ retry · esc cancel` |
| Clone done | the new repo's graph, cursor on HEAD |
| Pull, nothing new | cursor where it was, `up to date ✓` |

## Interfaces and data shapes (proposed; pass 2 fixes them after the scouts)

```rust
// graft-term — fence and frame bracket
pub const SYNC_BEGIN: &[u8] = b"\x1b[?2026h";
pub const SYNC_END: &[u8] = b"\x1b[?2026l";
pub fn write_fence(out: &mut impl std::io::Write) -> std::io::Result<()>;   // CSI 5 n
pub fn fence_answers(bytes: &[u8]) -> usize;                                 // count of CSI 0 n, like `replies`

// graft::event_loop (pub(crate)) — one frame in flight at most
pub(crate) struct Pacer { /* sent_at: Option<Instant>, misses: u8, fenced: bool */ }
impl Pacer {
    pub(crate) fn new(fence_timeout: std::time::Duration) -> Pacer;
    pub(crate) fn may_draw(&self, now: std::time::Instant) -> bool;
    pub(crate) fn drew(&mut self, now: std::time::Instant);        // frame + fence written
    pub(crate) fn answered(&mut self, now: std::time::Instant);    // Wake::Fence
    pub(crate) fn deadline(&self) -> Option<std::time::Instant>;   // wake for the timeout
}
// Wake gains `Fence`; after 3 straight timeouts the pacer stops fencing and draws at most every 16 ms.

// graft — product frame trace (GRAFT_FRAME_TRACE=<file>; numbers only)
pub struct FrameRecord { pub wakes: u16, pub motions: u16, pub update_us: u32, pub commands: u16,
    pub draw_us: u32, pub bytes: u32, pub write_us: u32, pub fence_us: Option<u32> }

// graft-ui::keymap — the 02-keymap.md tables as data; everything that prints a key reads it
pub enum ContextId { Global, Graph, Changes, Diff, Resolver, Banner, Finish, Rewrite, Picker, Blame,
    History, Stash, Journal, Map, Preview, Menu, Palette, EmptyState, Tour }
pub struct ActionId(pub &'static str);                  // "graph.merge": the config name, stable
pub struct KeyChord { pub code: KeyName, pub ctrl: bool, pub alt: bool, pub shift: bool }
pub struct Binding { pub action: ActionId, pub context: ContextId, pub default: &'static [KeyChord], pub label: &'static str }
pub fn defaults() -> &'static [Binding];
pub struct Keymap { /* defaults + overrides, per context */ }
pub enum Translated { Default(KeyChord), Unbound, Pass }
pub enum KeyWarning { UnknownAction(String), BadKey { action: String, text: String },
    Collision { key: KeyChord, context: ContextId, yours: ActionId, displaced: ActionId }, Reserved(KeyChord) }
impl Keymap {
    pub fn with_overrides(lines: &[(String, Vec<String>)]) -> (Keymap, Vec<KeyWarning>);
    pub fn translate(&self, ctx: ContextId, pressed: KeyChord) -> Translated;  // graft calls before every dispatcher
    pub fn spell(&self, action: &ActionId) -> KeyLabel;                       // "m", "⌃P", "—" when unbound
    pub fn in_context(&self, ctx: ContextId) -> Vec<(&Binding, &[KeyChord])>;
}

// graft-ui::theme
pub fn contrast(fg: Rgb, bg: Rgb) -> f32;
pub struct ContrastRule { pub fg: Token, pub bg: Token, pub min: f32 }
pub const CONTRAST_RULES: &[ContrastRule];   // text/bg 4.5 · text/selection 4.5 · selection/bg 3.0 · text-muted/bg 3.0
pub struct ContrastMiss { pub rule: ContrastRule, pub ratio: f32 }
impl Theme { pub fn with_overrides(self, overrides: &[(Token, Rgb)]) -> (Theme, Vec<ContrastMiss>); }

// graft-ui::palette — one widget, two entry modes (`?` = this place, `⌃P` = everything)
pub enum PaletteMode { Here(ContextId), Everything }
pub struct PaletteRow { pub label: SanitizedText, pub key: Option<KeyLabel>, pub config: Option<ActionId>, pub kind: RowKind }
pub struct PaletteView { pub mode: PaletteMode, pub query: FieldView, pub groups: Vec<(SanitizedText, Vec<PaletteRow>)>,
    pub cursor: usize, pub warnings: Vec<SanitizedText> }

// graft-ops::tour — never touches his repo
pub struct PracticeRepo { pub work: std::path::PathBuf, pub remote: std::path::PathBuf }
pub fn build_practice_repo(under: &std::path::Path) -> Result<PracticeRepo, TourError>;  // a temp dir only
// graft (pub(crate))
pub(crate) enum TourTask { StageHunk, Commit, Undo, StashAndBack, Branch, MergeWithPreview, ResolveConflict, Push }
pub(crate) struct TourState { pub task: TourTask, pub done: [bool; 8], pub return_to: Option<(std::path::PathBuf, RowAnchor)> }

// graft::cli — one table feeds --help and the man page
pub struct Flag { pub long: &'static str, pub value: Option<&'static str>, pub help: &'static str }
pub const FLAGS: &[Flag];
pub fn man_page(flags: &[Flag], keys: &[Binding]) -> String;   // roff; `--generate-man` prints it (hidden)
```

Callers: `event_loop::pump` owns `Pacer`; `update` calls `Keymap::translate` once at its key entry, before any
`*_key` dispatcher; `graft_ui::graph_screen::hints` tables move from key strings to `ActionId` and render through
`spell`; config `[keys.<context>]` and `[theme.<kind>]` parse into the two `with_overrides` calls.

## Hard-to-reverse choices

1. **Frame pacing contract: at most one unanswered frame in the pipe.**
   - Options: status quo (one frame per turn, written whenever); fixed 60 fps cap; **fence-paced** (DSR per frame,
     16 ms fallback when a terminal never answers).
   - **Recommend fence-paced.** Why: the only option that bounds pipe backlog by the terminal's real speed — Ghostty
     with deflate, a Kitty terminal without (M7/46-kitty-no-deflate's tail is measured through a rate-limited drain,
     `held_key` at 40 MB/s: the case a fence bounds), SSH (rate = 1/RTT).
   - Changes the event loop's contract every later frame depends on. ADR (proposed): *the loop never writes ahead of
     the terminal*.
2. **Remap mechanism and config names.**
   - Options: rewrite ~50 `*_key` dispatchers to match actions; **translate at the edge**: the pressed chord becomes
     the action's default chord before dispatch, a displaced default becomes `Unbound`.
   - **Recommend translate at the edge.** Why: one seam, dispatchers untouched; risk is a context the translator gets
     wrong, held by a sweep (remap every action to a free chord, walk every context, assert no default spelled and
     every remapped key acts).
   - `ActionId` strings become his config file's API: `<context>.<verb>`, snake_case, from 02-keymap.md's words;
     never renamed after release.
3. **The final name reaches disk.** `refs/graft/backup/**` (seen in the M7 usability walk), `<common_dir>/graft/`,
   the config dir, `GRAFT_*` env vars.
   - Options: keep internal `graft` names forever; **rename all, migrate once at launch** (journaled, idempotent,
     old copy kept until the new one reads back).
   - **Recommend rename + migrate.** Why: "graft" is another product's name; his live journal and backups must survive
     the switch. Amends ADR-0010. The rename runs alone, last before acceptance.

## Where I'd differ from DESIGN.md (Proposed changes; tickets follow DESIGN.md until approved)

1. **Tour = practice repo, by doing** (DESIGN §1.15: four 30 s callouts on his real repo; PROMPT §5 "30-second").
   Why: vimtutor "if you only read the text, you will forget"; undo and merge can't be practised safely on his repo.
   Cost: ~3–5 min, opt-in, one status line instead of an automatic tour; a repo builder in graft-ops.
2. **`?` is help everywhere, the empty state included; `t` starts the tour there** (§2.13 gives `?` the tour). Why:
   same key, same meaning (UX H4). Cost: none; `t` is free on the empty state (no repo vocabulary there).
3. **`?` is the palette opened on "this place"** (§1.16 draws a separate key grid). One widget, rows runnable with
   `⏎`, config name shown. Why: one surface to learn and build; the palette already spells keys (§1.6). Cost: a list
   shows fewer keys at once than the grid at 80×24 (~18 rows; scrolls).
4. **§4.1 gains the per-frame DSR fence**; the diagnostic DECRQM stays. Why: choice 1.
5. **Carried past v1** (needs his OK; PROMPT §4/§5 scope): pixel-tier image diffs (row 371), Map view mouse and remote
   rows (372, 374), undo-backup pruning (205). Why: none is in his daily use; each is a screen of its own.
6. Process, not DESIGN: parked `50-scroll-pixel-memory-budget` drops from Fable (signature) to **Opus** — its own spec
   decides the shape and a script judges the RSS (fails Fable test 3).

## New UI (each: Problem · User · Solution, UX §3.7)

**`?` / `⌃P` palette** (DESIGN §1.6 layout stands; additions here).
- Problem: "`?` does nothing" (M6/u2). User: him, any place, 187-col Zed, Unicode. Solution: journeys 4–8.
- Box over the focused pane, ≤ 96 cols wide, centred; at < 100 cols full width. Top row: `› query▏`. Groups (Hick):
  `IN PROGRESS` (only during an operation) → `HERE` (focused place) → `EVERYWHERE`; `⌃P` adds BRANCHES, TAGS, FILES,
  COMMITS, then RECENT.
- Row: key (accent, right-padded 6 cells) · label · config name (muted, right-aligned; dropped below 100 cols). Eye
  lands on the key column. A bound key typed alone ranks its row first. Unbound rows show `—`.
- `↑↓` move, `⏎` runs, `esc` closes. Empty filter result: `nothing matches · esc close`. Config problems: a `YOUR
  CONFIG` group on top, one row each (`m given to graph.fetch — merge has no key`).

**First-launch line.** Status row only, muted: `first time? ? keys here · ⌃P anything · ⌃P "tour" to practise`. Gone
after `?` or `⌃P` is used once, or after 5 launches. Never covers a problem line (problem wins).

**Tour card.** Top of the graph pane, 3 rows, warning-bordered; the header path reads `practice · <tmp>` with `◇`, so
the practice repo never looks like his.
```
╭ practice 3 of 8 · undo that commit ───────────────────────────────────── esc leave ╮
│ u takes back the last thing you did — nothing is lost        ✓ stage ✓ commit · undo │
╰─────────────────────────────────────────────────────────────────────────────────────╯
```
Ticks by doing, never by reading. At < 100 cols the right-hand ticks drop. Tour keys are the app's keys; the card
claims none but `Esc`.

**Empty state** (DESIGN §1.15 stands) adds `t  practice tour`; `c Clone…` opens a one-field prompt `clone › <url>`
with `→ <cwd>/<name>` under it, progress in place, errors on the prompt.

**Themes:** no new screen. Palette rows `Theme: …`; config warnings as above.

## Genuinely risky tickets

- **Pacing (G-pace)** — Fable (signature): approach decided here; a wrong loop contract is structural; smoothness on
  his terminals can't be judged from diff or fixture tests. Passes all three.
- **Keymap table (G-keymap)** — Fable (signature): every later surface (bar, palette, tour, man) builds on it.
- Risky, not Fable: the **rename migration** (his live journal and backup refs; judged by tests on fixtures — Opus,
  run alone); the **tour repo builder** (must never write outside its temp dir — a path guard + test).
- **Fable count: 2** (+ acceptance + the standing architecture review, which don't count).

## Expected groups (pass 2 specs)

| # | Group | Crates | Model | Contents |
|---|---|---|---|---|
| 1 | G-trace | graft (event_loop), scripts | Opus | product frame trace; summariser into `docs/perf/m8-trace-<date>.txt` (also logs `pmset` power state). **Then he runs it once** on his repo in Ghostty and Zed. |
| 2 | G-pace | graft-term, graft event_loop | **Fable (sig)** | fence, sync bracket, `Pacer`, wheel sum; 1089's tail, 1095 |
| 3 | G-settle | graft update, jobs | Opus | per-row work at rest; per-turn cost independent of history/ref count (from the trace) |
| 4 | G-pixel-mem | graft-graph, graft-term kitty, graft graphics | Opus | parked 50, after 2 (shares kitty + graphics) |
| 5 | G-keymap | graft-ui keymap + hints, graft config + update keys | **Fable (sig)** | table, translate, spell, `[keys.*]`, warnings |
| 6 | G-palette | graft-ui palette, graft update::palette | Opus | `?` and `⌃P`, sources, theme rows, tour row |
| 7 | G-bar | graft-ui hints, graft view | Opus | bar-content pass: 163, 538, 539, 553, 1057, 1065, 1084 |
| 8 | G-theme | graft-ui theme, graft config | Opus | floors + tests, dark nudge (541/542), overrides, look rows |
| 9 | G-first-run | graft (startup, recent), graft-ui | Opus | empty state, clone, first-launch line |
| 10 | G-tour | graft-ops tour, graft, graft-ui | Opus | practice repo, tasks, card |
| 11 | G-daily | graft-ops, graft update | Opus | his two rulings + daily rows |
| 12 | G-words | graft-ui, graft, graft-ops (words only) | Sonnet | wording rows |
| 13 | G-hygiene | scripts, fixtures, graft-git | Sonnet | hygiene rows |
| 14 | G-docs | graft cli, docs | Opus | README, CHANGELOG, `--help`, man page + staleness test |
| 15 | G-release | dist config, CI files, scripts | Opus | dist (aarch64/x86_64 macOS, x86_64/aarch64 Linux gnu), tap, binstall, CI (macOS+Linux, git 2.38 + latest), 549/550/551 |
| 16 | arch pass → G-health | all (review), then by file | Fable review → Sonnet/Opus | standing ticket; then the >400-line rows |
| 17 | G-rename | all | Opus, alone | after he picks the name |
| 18 | G-accept | — | Fable | walk, TESTING.md, shots, contact sheet |

≈18 groups, ≥22 tickets: pass 2 needed. Scroll lane 1→2→3→4 first; 5, 11, 12, 13 run beside it on disjoint crates.
**Nothing is published or registered** under any name: dist runs `plan`/`build` locally; tag, push, tap repo and
crates.io are a human's, after the trademark search (`naming-research.md` §6).

**Scout questions** (brownfield; code, not intent):
- Scroll: is every frame (text + Kitty commands) inside DEC 2026, file:line? Wheel: step per event in the graph, and is
  a turn's wheel applied per event or summed? What does one `j` emit (commands, jobs) with the detail pane shown and
  hidden, and which are cancelled on the next `j`? Any per-frame work scaling with refs or commits (cf. row 414)? Does
  `pump` block in `write` on a full pty? How do terminal replies reach the loop mid-session (`ReplyWaiter` → `Wake`)?
- Keys: the list of `*_key` dispatchers; one entry point before them? How `HintContext` maps to the dispatch context.
- Startup: what `graft` does outside a repo; can the app close one repo and open another in-process (tour, recent,
  clone)?
- Config: TOML crate, `[keys]` parse today, `ThemeChoice` values, OSC 11 focus re-query built?
- Palette: is `graft_ops::switch_list::filter_switch_rows`'s matcher reusable?
- Rename: every on-disk and env name containing `graft` (refs, dirs, env vars, socket names).

## KNOWN_GAPS rows naming M8 (114 rows whose expiry cell names M8, grep 2026-10-02; plus 960 and 998, his rulings)

| Fate | Rows (line numbers in KNOWN_GAPS.md) | Group |
|---|---|---|
| Fix — scroll | 1095, 1089, 1093 | 1–4 |
| Fix — help, palette, bar | 163, 538, 539, 553, 1057, 1065, 1084 (if a per-row pushed fact can be made resident; else carry); stale M7 row 840 | 6, 7 |
| Fix — theme and look | 541, 542, 145 (decide: by design → close), 156, 159, 162, 373, 557, 979, 983 | 8 |
| Fix — first run, tour | 558, 505 (clone) | 9, 10 |
| Fix — daily | 960, 998, 509, 931, 1050, 1055, 1056, 1058, 1081 (check), 1083 (his "fine until M8") | 11 |
| Fix — words | 376, 402, 403, 951, 953, 1042, 1059, 1060, 1061, 1062, 1063, 1066, 1067, 1072, 1086; stale M7 row 766 | 12 |
| Fix — hygiene, before publishing | 147, 164, 448, 959, 1000, 1045, 1049, 1068, 1069, 1074, 1076, 1077, 1087, 1090, 1091, 1094 | 13 |
| Fix — docs, release | 224, 458, 549, 550 (decide: no protocol bytes committed → remove, keep digest), 551, 980 | 14, 15 |
| Fix — code health (after the arch pass) | 935, 985, 991, 992, 1027, 1034, 1043, 1082 | 16 |
| M8 review run, not a ticket | 944, 963, 1044, 1071, 1078, 1079, 1080, 1088 | review |
| His eye at the stop (stays unverified until he looks, rule 5) | 562 | stop |
| Regenerated by acceptance | 1085 | 18 |
| Close | 795 (accepted by design), 981 (below 30 cols), 1053 (ADR-0052 bound), 1064 (his "keep it") | — |
| Carry past v1 (Proposed 5 where scope) | 371, 372, 374, 205, 174, 248, 297–300, 866, 867, 910, 967, 976, 989, 990, 1001, 1047, 1048, 1051, 1052, 1054, 1073, 1075 | — |

The conductor re-expires rows still naming M7 that no M8 group takes.

## Prototype brief (step 5) — `prototypes/m8-polish-and-ship/`, throwaway

- Fake data, real keys, his three terminals (Ghostty, Zed, VS Code). The prototype logs `$TERM_PROGRAM`, `COLORTERM`
  and the OSC 11 reply itself, so theme and terminal are on record without asking. Keys and pauses to `picks.txt`.
- Tasks in his words, no keys named (UX §4.1). One question at the end of each scene.

**Scene 1 — scroll feel + terminal probe** (do first).
- 17,694 graph-like Unicode rows. Task: "go back about two weeks fast, stop on the commit that says *bump deps*", by
  `j` held, then by trackpad, then wheel.
- Two builds behind blind labels `1`/`2`: unpaced, and fence-paced with sync. Wheel step 1 vs 3 (`s` toggles,
  named on screen).
- Records per terminal: DSR round trip p50/p99, whether DEC 2026 is answered, events/s during a flick.
- Judge: did it stop where he let go? Which of 1/2 felt smoother? Which wheel step? Do all three terminals answer the
  fence (one that doesn't makes the 16 ms fallback its daily path)?
- It measures the terminals, not Graft; Graft's own costs come from the G-trace run.

**Scene 2 — `?` and `⌃P`.**
- Tasks: "put your changes aside"; "what does `x` do?"; a stopped rebase: "how do you go on?"; "you want merge on
  another key: what would you write in the config?"
- Judge: steps vs journeys 4–8; did his eye find the key column first; was the config name noticed unasked?

**Scene 3 — first run and tour.**
- Outside a repo: "open the project you worked on yesterday", then "clone this URL".
- In a repo for the first time: does he notice the line? Then three tour tasks (stage a hunk, commit, undo it) and a
  drag, then `Esc` mid-task.
- Judge: stuck > 30 s anywhere; does the card cover anything he needed; after `Esc`, does he know he's back in his own
  repo?

**Scene 4 — themes, by eye.**
- Dark, light, high contrast; two dark candidates (bg darker / text brighter) side by side: cursor row, a lane on the
  cursor row (542), conflict tints, muted text. Same screen in VS Code (its 4.5 minimum-contrast setting repaints
  foregrounds, `themes.md`).
- Judge: which dark reads better; does VS Code change the lane colours visibly?

Ask at the end, one line: where did two different things still look the same?

## Questions for him (batched, his terms)

1. **Name.** "Tributary, typed `trib`" (recommended: the graph is a river system, every registry free for `trib`) or
   "Batten, typed `batten`" (says *safe*)? Before anything public a person must run a trademark search (USPTO classes
   9 and 42); we register nothing.
2. **Mac download warning.** Pay Apple $99/year so a binary downloaded in a browser opens without a warning?
   Recommended: no — `brew install` and the install script don't trigger it.
3. **One timing run** (after group 1): start the app with one setting on your work repo, scroll as you do for a
   minute, quit. It records timings only, no code or names.
4. **Try the prototype** (scenes 1–4, ~15 min).
