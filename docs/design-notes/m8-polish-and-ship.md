# M8: polish, final name and ship — pass 2a, 2026-10-02

**Status: pass 2a.** Decisions recorded (his answers 2026-10-02): ADR-0054 pacing, ADR-0055 keymap, ADR-0056 help +
tour, ADR-0057 Tributary + what ships, ADR-0058 chrome + cursor row + dark B. Specs for the scroll groups and the
look-parity group are in `.scratch/m8-polish-and-ship/issues/`; the rest in "Pass 2b remaining". Inputs: PROMPT.md §8
M8 row, §5, §9; research `scroll-smoothness.md`, `big-repo-scroll.md`, `themes.md`, `keymap-help-remap.md`,
`first-run.md`, `install-docs.md`; `docs/naming-research.md`; field notes :46, :48–50, :82–86 and 2026-10-02; the
prototype (`prototypes/m8-polish-and-ship/README.md`, `docs/shots/m8-proto-*.txt`); `real-vs-mockup.md`; MAP.md of
graft, graft-ui, graft-term. No source read.

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
| Readable in his theme | cursor row unreadable in dark, twice (:44, :48); 3.04:1 (:50); text on it 4.28:1 (M5/44); branch label on it "dim magenta on grey, hard to read" (real-vs-mockup 6) | daily, dark |
| Install like a real tool | runs a build from this repo today | once |
| Two daily rulings | "pull with nothing new → cursor stays; `a` on HEAD with nothing staged → reword" (:86) | daily |

## Prior art (T take · R reject · I invent)

| Aspect | Best answer (study) | Ours | |
|---|---|---|---|
| Paint only visible rows | VS Code ListView (`scroll-smoothness` §1) | row cache + strip cache (DESIGN §3.6, §3.8) | T (have) |
| One paint per display frame | VS Code `scheduleAtNextAnimationFrame`; ratatui frame-rate recipe (`big-repo-scroll`) | a turn's wakes share one frame (m7/43) | T (have) |
| **Never let the terminal fall behind** | nothing found | DSR fence per frame (ADR-0054) | **I** — studies cover input batching, not output pacing |
| Atomic frames | DEC 2026 (§3) | text **and** image commands inside one pair | T |
| Wheel bursts | lazygit 2 rows/event, zellij per event (§2) | a turn's wheel events sum; per-event step unchanged until scene 1's verdict | T step · I sum |
| Per-row work while moving | VS Code: scroll moves an offset (§1) | detail follow, radar probe, emphasis start at rest; stale ones cancelled | T (extended) |
| Kitty uploads | re-place by id, `d=p` (§3) | have (m7/46) | T (have) |
| Kitty Unicode placeholders | yazi; WezTerm doesn't render | — | R |
| Measure where he is | gitui #2823 | opt-in `GRAFT_PERF_TRACE`, numbers only | I (rule 1, :84) |
| Theme auto + pin | zellij, bat (`themes.md`) | OSC 11 + `ui.theme`, focus re-query | T (have) |
| Switch theme now | zellij ToggleTheme, helix `:theme`, VS Code ⌘K⌘T | palette rows `Theme: …`, no key | T |
| Change one colour | gitui 0.23, helix `inherits`, VS Code `colorCustomizations` | `[theme.dark] selection = "#…"` | T |
| Contrast floor | nobody checks (lazygit #4306/#508, gitui #922) | every theme tested against fixed floors; a missing override is named | **I** (we shipped the bug, :48) |
| Per-widget attrs; file-only themes; palette-inheriting default | lazygit; gitui; bat `ansi` | — | R |
| **Screen chrome** | the M8 mock-up (his pick, 2026-10-02); Sublime Merge / lazygit: name + branch on top, words in the bar | ADR-0058 §4, DESIGN §1.0 | T (mock-up) |
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
| **Empty state** | Sublime Merge opens on clone/new; VS Code welcome recent list | mock-up §1.15: o/i/c/t + RECENT, newest preselected | T |
| Binaries, tap, installer | dist v0.33 (`install-docs` §2) | 4 targets, tap formula, curl installer | T |
| homebrew-core; clap_mangen; completions; update notifier | all in core; ripgrep; Homebrew discourages | — | R |
| Man page | ripgrep `--generate man` | from our CLI table + keymap; staleness test | T |
| Notarisation | dist doesn't | none (ADR-0057) | R (his call) |

**Dark pick: B** (`docs/shots/m8-proto-4a-dark-a-b.txt:21-24`): cursor row vs ground **3.14** (A 3.05, floor 3.0); text
on cursor row **5.14** (A 4.61, floor 4.5); muted text on ground **6.95** (A 6.15, floor 4.5). Blue lane on cursor row
1.20 (A 1.33): fails in every candidate, so the fill leaves the graph column — lanes on the cursor row sit on ground,
worst lane-3 `#AA3377` ≈ 2.97:1, as on every row today (ADR-0003 band). Ground `#14161B` unchanged. Floors as tests:
text/bg 4.5 · text/selection 4.5 · selection/bg 3.0 · text-muted/bg **4.5** (pass 1 said 3.0; B clears 4.5).

**Pipeline check after 2a.** Prior art: every new aspect has a row (chrome, empty state taken from the approved mock-up;
fill exclusion invented, why given). Journeys: J12, J14 recounted against the empty state; J21–J23 added. Prototype
(step 5) ran, scenes 1–4 shot; scene 1's feel (blind 1/2, wheel step) still his — the wheel step stays per event until
then. Satisfied.

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
| 19 | Reword HEAD, nothing staged | `a`, edit, `⏎` | 3 | lazygit 2–3 |
| 20 | Undo, see what came back | `u` | 1; lands on the result | GitUp 1 |
| 21 | Make this folder a repo | `i` `y` (empty state) | 2+1 | Sublime Merge ~3 |
| 22 | Which repo and branch am I in | read the header | 0 | VS Code status bar 0 |
| 23 | Find the cursor row, read its branch | read the `▸` row | 0 | Sublime Merge 0 |

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
reword-head                   3     # +1 · lazygit 2–3 · a <edit> ⏎
undo-see-result               1     # +0 · GitUp 1 · u
```

Not walked in the pty (a person's step, a TESTING.md row): rebind (5+1), fix a colour (2+1), install (2), man (1).
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
impl Theme { pub fn with_overrides(self, overrides: &[(Token, Rgb)]) -> (Theme, Vec<ContrastMiss>); } // G-theme

// graft-ui::graph_screen — chrome (look-parity 5a)
pub const APP_NAME: &str = "graft";                                // G-rename changes this line
pub struct HeaderView<'a> { pub repo: &'a str, pub path: &'a str, pub branch: &'a str,
    pub tail: Option<&'a str>, pub practice: bool }
pub fn header_line(view: &HeaderView, width: u16, theme: &Theme) -> ratatui::text::Line<'static>;
pub fn spaced(rows: u16) -> bool;                                  // rows >= 30: blank row + rule
// ScreenRows gains `gap: Rect`, `rule: Rect` (zero-height under 30 rows); CHROME_ROWS becomes `chrome_rows(rows: u16) -> u16`.

// graft-ui::empty_state (look-parity 5c)
pub struct RecentRow { pub name: SanitizedText, pub path: SanitizedText, pub age: SanitizedText }
pub struct EmptyStateView { pub folder: SanitizedText, pub missing: bool, pub recent: Vec<RecentRow>,
    pub cursor: usize, pub prompt: Option<EmptyPrompt>, pub problem: Option<SanitizedText> }
pub enum EmptyPrompt { Open(FieldView), ConfirmInit }
pub struct EmptyState<'a> { pub view: &'a EmptyStateView, pub theme: &'a Theme }   // Widget

// graft — recents and in-process switch (5c)
pub(crate) fn read_recent(config_dir: &Path) -> Result<Vec<PathBuf>, RecentError>;  // newest first, ≤ 20, deduped
pub(crate) fn remember(config_dir: &Path, repo: &Path) -> Result<(), RecentError>;
// Exit gains `Open(PathBuf)`; main re-runs startup on it (tour and clone reuse it in 2b)

// graft-ui::keymap (G-keymap, 2b; ADR-0055)
pub enum ContextId { Global, Graph, Changes, Diff, Resolver, Banner, Finish, Rewrite, Picker, Blame,
    History, Stash, Journal, Map, Preview, Menu, Palette, EmptyState, Tour }
pub struct ActionId(pub &'static str);                 // "graph.merge", never renamed after release
pub struct KeyChord { pub code: KeyName, pub ctrl: bool, pub alt: bool, pub shift: bool }
pub struct Binding { pub action: ActionId, pub context: ContextId, pub default: &'static [KeyChord], pub label: &'static str }
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

// graft-ui::palette (G-palette, 2b)
pub enum PaletteMode { Here(ContextId), Everything }
pub struct PaletteRow { pub label: SanitizedText, pub key: Option<KeyLabel>, pub config: Option<ActionId>, pub kind: RowKind }
pub struct PaletteView { pub mode: PaletteMode, pub query: FieldView, pub groups: Vec<(SanitizedText, Vec<PaletteRow>)>,
    pub cursor: usize, pub warnings: Vec<SanitizedText> }

// graft-ops::tour (G-tour, 2b) — never touches his repo
pub struct PracticeRepo { pub work: PathBuf, pub remote: PathBuf }
pub fn build_practice_repo(under: &Path) -> Result<PracticeRepo, TourError>;
pub(crate) enum TourTask { StageHunk, Commit, Undo, StashAndBack, Branch, MergeWithPreview, ResolveConflict, Push }

// graft::cli (G-docs, 2b)
pub struct Flag { pub long: &'static str, pub value: Option<&'static str>, pub help: &'static str }
pub const FLAGS: &[Flag];
pub fn man_page(flags: &[Flag], keys: &[Binding]) -> String;
```

Callers: `event_loop::pump` owns `Pacer` and `FrameTrace`; the graph view builds `HeaderView`; every geometry mirror
(`update::geometry`, mouse row math, `pane_boundaries`) reads `ScreenRows`; row painting calls `legible_on` for any fg
drawn on `selection`; startup draws `EmptyState` when discovery finds no repo; `update` calls `Keymap::translate` once
before any `*_key` dispatcher (2b).

## Hard-to-reverse choices (decided)

1. Fence-paced frames — ADR-0054.
2. Keymap translate-at-the-edge; `ActionId` is his config's API — ADR-0055.
3. The name reaches disk; migrate once — ADR-0057.
4. The cursor-row fill excludes the graph column — ADR-0058 §2. Undo cost: every cursor-row capture and paint test.

## Where I'd differ from DESIGN.md

Pass-1 Proposed 1–5 approved 2026-10-02 and applied: DESIGN §1.0 (new), §1.6, §1.15, §1.16; keymap §2.1, §2.2, §2.12,
§2.13, §2.18 (new); terminal §4.1, §4.7 (new). **PROMPT.md still to edit** (handed to the conductor). Nothing new.
Process: parked m7/50 re-homed as `pixel-memory-budget.md`, on Opus (its spec decides the shape; a script judges RSS).

## New UI

- Chrome and cursor row: DESIGN §1.0. Empty state: §1.15. `?`: §1.16. Palette rows: §1.6.
- **First-launch line** (2b): status row, muted: `first time? ? keys here · ⌃P anything · ⌃P "tour" to practise`; gone
  after `?`/`⌃P` once or 5 launches; a problem line wins.
- **Tour card** (2b): top of the graph pane, 3 rows, warning border; header `◇ practice · <tmp>`.
  `╭ practice 3 of 8 · undo that commit ──── esc leave ╮` / `│ u takes back the last thing you did — nothing is lost
  ✓ stage ✓ commit · undo │`. Ticks by doing; < 100 cols drops the ticks; claims no key but `Esc`.
- Look-parity keeps the `. more` doorway; G-palette turns it into `? keys` (`every_key_the_bar_draws_is_answered`).

## Genuinely risky tickets

- **G-pace** — Fable (signature): approach decided here; a wrong loop contract is structural; smoothness on his
  terminals can't be judged from diff or fixtures. Passes all three.
- **G-keymap** — Fable (signature): every later surface builds on it.
- Risky, not Fable: rename migration (Opus, alone); tour repo builder (path guard); look-chrome (the row split moves
  every geometry mirror: the scout lists them, a sweep holds them).
- **Fable count: 2** (+ acceptance + the standing architecture pass).

## Expected groups — final order

| # | Group | Crates | Model | Spec |
|---|---|---|---|---|
| 1 | G-trace | graft event_loop, scripts | Opus | `trace-product-frame-trace.md` |
| 2 | G-pace | graft-term, graft-core caps, graft event_loop | **Fable (sig)** | `pace-fence-and-sync.md` |
| 3 | G-settle | graft update, jobs | Opus | `settle-work-at-rest.md` |
| 4 | G-pixel-mem | graft-graph, graft-term kitty, graft graphics | Opus | `pixel-memory-budget.md` |
| 5a | G-look chrome | graft-ui graph_screen + hints, graft view | Opus | `look-chrome-header-gutter-bar.md` |
| 5b | G-look cursor row + dark B | graft-ui theme + graph rows | Opus | `look-cursor-row-and-dark-b.md` |
| 5c | G-look outside repo | graft-ui empty_state, graft startup | Opus | `look-outside-repo-screen.md` |
| 6 | G-keymap | graft-ui keymap + hints, graft config + update | **Fable (sig)** | 2b |
| 7 | G-palette | graft-ui palette, graft update | Opus | 2b |
| 8 | G-bar | graft-ui hints, graft view | Opus | 2b |
| 9 | G-theme | graft-ui theme, graft config | Opus | 2b |
| 10 | G-first-run | graft startup, graft-ui | Opus | 2b |
| 11 | G-tour | graft-ops tour, graft, graft-ui | Opus | 2b |
| 12 | G-daily | graft-ops, graft update | Opus | 2b |
| 13 | G-words | graft-ui, graft, graft-ops (words) | Sonnet | 2b |
| 14 | G-hygiene | scripts, fixtures, graft-git | Sonnet | 2b |
| 15 | G-docs | graft cli, docs | Opus | 2b |
| 16 | G-release | dist, CI, scripts | Opus | 2b |
| 17 | arch pass → G-health | all | Fable review → Opus/Sonnet | `fable-architecture-pass.md` (standing) |
| 18 | G-rename | all | Opus, alone | 2b |
| 19 | G-accept | — | Fable | 2b |

Lanes: 1→2→3; 4 after 2 (kitty, graphics). 5a→5b (same files); 5c beside 5b (disjoint files). 12 and 14 can run beside
the scroll lane (disjoint crates). 6 after 5a/5b (hints). **Nothing is published or registered** (ADR-0057).

**Scout questions for 2b** (code, not intent): the `*_key` dispatchers and one entry point before them; how
`HintContext` maps to the dispatch context; TOML crate and `[keys]` parse today; `ThemeChoice` values; OSC 11 focus
re-query built?; is `filter_switch_rows`'s matcher reusable; every on-disk and env name containing `graft`.

## Pass 2b remaining

- **G-trace follow-up:** fixes the trace shows (per-turn cost vs history and ref count, cf. row 414), after his run.
- **G-keymap** (Fable sig): table, translate, spell, `[keys.*]`, warnings, sweep test.
- **G-palette:** `?`, `⌃P`, `:`; groups; sources; theme rows; tour row; doorway → `? keys`; rows 163, 538, 539, 553.
- **G-bar:** 1057, 1065, 1084 (if resident, else carry), stale 840.
- **G-theme:** `[theme.<kind>]` overrides + warnings; look rows 145 (close by design?), 156, 159, 162, 373, 557, 979, 983.
- **G-first-run:** clone `c` (505), first-launch line (558).
- **G-tour:** practice repo, 8 tasks, card, `t` on the empty state.
- **G-daily, G-words, G-hygiene, G-docs, G-release:** rows below.
- **arch pass → G-health** (standing spec exists; G-health specs follow its review).
- **G-rename** (ADR-0057). **G-accept** (Fable): the `journey-counts` fence above, TESTING.md, shots, contact sheet.

## KNOWN_GAPS rows naming M8 (grep 2026-10-02; plus 960, 998)

| Fate | Rows | Group |
|---|---|---|
| Fix — scroll | 1095, 1089, 1093 | 1–4 |
| Fix — look | 541, 542 | 5b |
| Fix — help, palette, bar | 163, 538, 539, 553, 1057, 1065, 1084; stale M7 840 | 7, 8 |
| Fix — theme | 145, 156, 159, 162, 373, 557, 979, 983 | 9 |
| Fix — first run, tour | 558, 505 | 10, 11 |
| Fix — daily | 960, 998, 509, 931, 1050, 1055, 1056, 1058, 1081 (check), 1083 | 12 |
| Fix — words | 376, 402, 403, 951, 953, 1042, 1059–1063, 1066, 1067, 1072, 1086; stale M7 766 | 13 |
| Fix — hygiene | 147, 164, 448, 959, 1000, 1045, 1049, 1068, 1069, 1074, 1076, 1077, 1087, 1090, 1091, 1094 | 14 |
| Fix — docs, release | 224, 458, 549, 550, 551, 980 | 15, 16 |
| Fix — code health | 935, 985, 991, 992, 1027, 1034, 1043, 1082 | 17 |
| M8 review run | 944, 963, 1044, 1071, 1078, 1079, 1080, 1088 | review |
| His eye at the stop | 562 | stop |
| Regenerated by acceptance | 1085 | 19 |
| Close | 795, 981, 1053, 1064 | — |
| Past v1 (ADR-0057) | 371, 372, 374, 205, 174, 248, 297–300, 866, 867, 910, 967, 976, 989, 990, 1001, 1047, 1048, 1051, 1052, 1054, 1073, 1075 | — |

Open item: a person's trademark search (USPTO classes 9 and 42) before anything public (ADR-0057).

## Answered 2026-10-02

Name Tributary / `trib`; no notarisation; Proposed 1–5 yes; look-parity group yes; dark A/B ours → B. Still owed by
him: scene 1's feel (blind 1/2, wheel step 1 vs 3) and one `GRAFT_PERF_TRACE` run after G-trace.
