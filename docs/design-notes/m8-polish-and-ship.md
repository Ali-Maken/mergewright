# M8: polish, final name and ship — pass 2b, 2026-10-02

**Status: pass 2b — all specs written.** Decisions recorded (his answers 2026-10-02): ADR-0054 pacing, ADR-0055
keymap, ADR-0056 help + tour, ADR-0057 Tributary + what ships, ADR-0058 chrome + cursor row + dark B. Specs in
`.scratch/m8-polish-and-ship/issues/` (list: "Expected groups"). Inputs: PROMPT.md §8
M8 row, §5, §9; research `scroll-smoothness.md`, `big-repo-scroll.md`, `themes.md`, `keymap-help-remap.md`,
`first-run.md`, `install-docs.md`; `docs/naming-research.md`; field notes :46, :48–50, :82–86 and 2026-10-02; the
prototype (`prototypes/m8-polish-and-ship/README.md`, `docs/shots/m8-proto-*.txt`); `real-vs-mockup.md`; MAP.md of
graft, graft-ui, graft-term (2b: also graft-ops, graft-git headings). No source read.

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
| Fork left behind (2c) | "mid-chain branch left behind after `a` → fine until M8" (:86); ruled 2026-10-02 (:89 item 5): it **moves too** when it moves clean, shown in the preview first | with dependents |

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
| Reword HEAD with a branch on it (2c) | lazygit reword, no preview (`stacked-branches` §1.9) | M7's rule: previewed when a branch is built on HEAD (keymap `a` row, :219) | T (have) |
| **Fork off a rewritten commit** (2c, approved 2026-10-02) | jj: every descendant follows (`stacked-branches` §1.4); branchless `restack` + skip-on-conflict (§1.3); Graphite `sync` skips conflicting (§1.2) | follows when its replay forecasts clean, else stays and says why (ADR-0059, accepted) | T descendants · T skip-on-conflict (for forks only) |

**Dark pick: B** (`docs/shots/m8-proto-4a-dark-a-b.txt:21-24`): cursor row vs ground **3.14** (A 3.05, floor 3.0); text
on cursor row **5.14** (A 4.61, floor 4.5); muted text on ground **6.95** (A 6.15, floor 4.5). Blue lane on cursor row
1.20 (A 1.33): fails in every candidate, so the fill leaves the graph column — lanes on the cursor row sit on ground,
worst lane-3 `#AA3377` ≈ 2.97:1, as on every row today (ADR-0003 band). Ground `#14161B` unchanged. Floors as tests:
text/bg 4.5 · text/selection 4.5 · selection/bg 3.0 · text-muted/bg **4.5** (pass 1 said 3.0; B clears 4.5).

**Pipeline check after 2b.** Prior art: every new aspect has a row. Journeys: every id in the fence has an owning spec
and a walk (`accept-m8.md`). Prototype (step 5) ran, scenes 1–4 shot; scene 1's feel (blind 1/2, wheel step) still
his — the wheel step stays per event until then. Satisfied.

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

**J19 is 4, not 3 (2c).** The 2b row counted the field-note case (:86) as the plain amend. But `a` on HEAD with
nothing staged was refused only when a branch is built on HEAD (ticket 14 item 1) — the case where other branches
move. There the keymap already says "previewed (`y`) only when a branch is built on HEAD" (`docs/design/02-keymap.md:219`,
his M7 "all yes"). Kept: M5's promise and UX §2 H5 (see the result before confirming: which branches move is not on
the editor); H4 (every history-changing preview confirms with `y`, field notes :58). UX §3.5 (undo permits fewer
confirms) argues for 3, but the preview here shows something nothing else on screen does. The build is right; only the
count was wrong. Nothing built on HEAD stays 3, no preview, not in the fence (M7's path, unchanged by M8).

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
| `q`, git stopped mid-operation, nothing running | quits, no box; next launch keeps the banner: continue or abort (keymap §2.6a, 2b) |
| On-disk rename can't finish | old names stay in force; `rename: still using the old names · <reason>` (2b) |

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
impl Theme { pub fn with_overrides(self, overrides: &[(Token, Rgb)]) -> (Theme, Vec<ContrastMiss>); } // G-theme
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

// graft — recents and in-process switch (5c)
pub(crate) fn read_recent(config_dir: &Path) -> Result<Vec<PathBuf>, RecentError>;  // newest first, ≤ 20, deduped
pub(crate) fn remember(config_dir: &Path, repo: &Path) -> Result<(), RecentError>;
// Exit gains `Open(PathBuf)`; G-tour widens it to `Open(Open { path, cursor, practice, back })`

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
pub fn fuzzy_score(query: &str, text: &str) -> Option<u32>;        // one matcher (graft-core or where the scout says)

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

## New UI

- Chrome and cursor row: DESIGN §1.0. Empty state: §1.15. `?`: §1.16. Palette rows: §1.6.
- **First-launch line**: status row, muted: `first time? ? keys here · ⌃P anything · ⌃P "tour" to practise`; gone
  after `?`/`⌃P` once or 5 launches; a problem line wins.
- **Tour card**: top of the graph pane, 3 rows, warning border; header `◇ practice · <tmp>`.
  `╭ practice 3 of 8 · undo that commit ──── esc leave ╮` / `│ u takes back the last thing you did — nothing is lost
  ✓ stage ✓ commit · undo │`. Ticks by doing; < 100 cols drops the ticks; claims no key but `Esc`.
- **Palette sources** (`⌃P`): branches/tags/commits `⏎` seek the cursor (never switch — `C` does); files open history
  (`F`); RECENT = palette actions run this session. ≤ 8 rows per group, `+N more`.
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
| 5c | G-look outside repo | graft-ui empty_state, graft startup | Opus | `look-outside-repo-screen.md` |
| 6a | G-keymap table | graft-ui keymap, graft config + update entry | **Fable (sig)** | `keymap-table-and-translate.md` |
| 6b | G-keymap spell | graft-ui hints + labels, graft words | Opus | `keymap-spell-from-the-table.md` |
| 7a | G-palette help | graft-ui palette, graft update | Opus | `palette-help-and-actions.md` |
| 7b | G-palette find | graft update::palette, search | Opus | `palette-find-anything.md` |
| 8 | G-bar | graft-ui hints, graft view | Opus | `bar-content-pass.md` |
| 9a | G-theme | graft-ui theme, graft config | Opus | `theme-overrides.md` |
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
| 19 | G-accept | — | Fable | `accept-m8.md` |

Lanes: 1→2→3→3b; 4 after 2. 5a→5b; 5c beside 5b. 6a→6b→7a→7b; 8 after 7a and 12b. 9a after 5b and 6a; 9b after 9a.
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
