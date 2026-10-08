# 18d — the first-launch line

Status: done (builder 2026-10-08)
Build with: **Sonnet** — decisions fixed in R7; a counter file, one fallback draw, one hook on the palette's opener.
Blocked by: 11, 11b (palette opener and empty-state bar in place)
Needs: none from 18a–18c
Crate set: **graft**, **graft-ui** (`empty_state` render only). Runs beside 18a (graft-ops); before 18b (shares
`places.rs`, `empty_state.rs`).

## Read first
- `.scratch/m8-polish-and-ship/issues/18-rulings.md` R7
- Design note `m8-polish-and-ship.md`: New UI "First-launch line"; Journeys J11 (`first-launch 0`)
- `docs/adr/0056-help-is-the-palette-on-this-place-and-the-tour-is-a-practice-repo.md` §4
- `crates/graft/MAP.md` `lib.rs` (`recent`, `empty_state`), `## event_loop` lines on `places::drive` / `Arrival`,
  `update::palette`; `crates/graft-ui/MAP.md` `empty_state`, `graph_screen::hints` (ADR-0039 shedding)
- Scout `scouts/18-first-run.md` "First-launch" and "Merged since" (`palette::opener` at `empty_state.rs:124`)

## Scope
In: `first_launch` module (count, done); `drive` counts once per process and carries `first_launch: bool` into every
place's `Arrival`; graph status row Ready fallback; empty-state row above the rule; `done` on the first palette open
(graph and empty state); one warning on a read/write failure; capture scripts seeded; MAP.md.
Out: the tour clause's tour row (G-tour); headless `run`, gallery, `Harness` (never count, never show — R7).

## Signatures
```rust
// graft::first_launch (new, pub(crate)) — `<config_dir>/graft/launches`: `<n>\n` or `done\n`
pub(crate) const SHOWN_FOR: u32 = 5;
pub(crate) enum FirstRunError { Read { path: PathBuf, source: io::Error }, Garbled { path: PathBuf },
    Write { path: PathBuf, source: io::Error } }
impl FirstRunError { pub(crate) fn words(&self) -> String; }   // `first-launch: <path>: <reason>`
/// One launch: read, add one, write (temp + rename, as `recent::remember`). Missing file = 0.
/// `Ok(true)` while the count after adding is ≤ SHOWN_FOR and not `done`; `done` is never rewritten.
pub(crate) fn count_launch(config_dir: &Path) -> Result<bool, FirstRunError>;
pub(crate) fn first_launch_done(config_dir: &Path) -> Result<(), FirstRunError>;
// graft-ui::empty_state — EmptyStateView gains:
pub first_launch: Option<SanitizedText>,   // drawn muted on the row above the rule when free; else not drawn
```
Words (keys spelled by `Keymap::spell`, clauses dropped from the right whole, never a fragment):
`first time? ? keys here · ⌃P anything · ⌃P "tour" to practise` — the last clause only when the palette listing has a
tour row (`rg` it; NOTES says which). `app.help` unbound → the first clause drops; both unbound → no line.

## Acceptance (each tick names its test, at tick time)
- [x] `count_launch`: absent → `true`, file `1`; `4` → `true`, `5`; `5` → `false`, `6`; `done` → `false`, file
      unchanged; `x7` → `Garbled`; unwritable dir → `Write`. (`first_launch::tests::count_launch_table`)
- [x] J11 = 0: launch 1 shows the line on the graph status row; `?` → palette opens, `launches` = `done`, line gone
      the same frame; same for `⌃P` and `:`; launch 6 gone with no use.
      (`update::tests::first_launch::{shown_until_the_palette_opens, gone_after_five_launches}`)
- [x] Priority: a problem and a notice each replace it; it returns when they clear.
      (`…::first_launch::a_problem_or_notice_wins_the_row`)
- [x] Empty state: shown above the rule at 120×36 with 3 recents; not drawn when the list fills that row; `?` there
      writes `done`. (`graft_ui::empty_state::tests::first_launch_sits_above_the_rule_when_free`,
      `graft::empty_state::tests::opening_the_palette_ends_the_first_launch_line`)
- [x] Widths 20..=200: every drawn clause whole. (`graft_ui::…::first_launch_never_draws_a_fragment`)
- [x] Failure: unreadable file → line not shown, one status warning with `words()`, app runs. 
      (`…::first_launch::a_broken_file_warns_once_and_never_stops_the_app`)
- [x] Headless `run` leaves `launches` absent in a temp config dir. (`…::first_launch::headless_never_counts`)
- [x] Every existing capture unchanged: re-shoot diff shows only the two new files. (diff, in NOTES)
- [x] Captures: `docs/shots/m8/first-launch-120x36.txt` (graph), `docs/shots/m8/first-launch-empty-state-120x36.txt`.
      **Colour unverified** (muted ink, rule 5). His to judge (rule 6).

## Touches
- **Every pty capture script** that runs the binary on a tty (`drive` path): find how 5c isolated recents
  (`rg -n 'XDG_CONFIG_HOME|config_dir|HOME=' scripts .scratch crates/graft/tests`); seed `launches` = `done` there,
  except for the two first-launch shots. No isolation found (captures hit his real config dir) → **stop, report**.
- The user's real `~/.config/graft/launches` is never written by a test: every test passes a temp config dir.
- Status row draw (graft `view`), `draw_status_or_hints` stand-in footers: the line is graph-only — check none of them
  draw it (`rg first_launch crates/graft/src/view`).
- Palette opener sites: graph (`update::palette`), empty state (`empty_state.rs:124`).
- G-rename list: `launches` (config dir names). MAP.md graft, graft-ui.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide.

## Size
One context, ~45k tokens.

## NOTES (builder, 2026-10-08)

Scout: used `scouts/18-first-run.md` (First-launch, Merged since); no new scout.

**Tests, as ticked** (names as they exist):
- count table: `first_launch::tests::count_launch_table` (+ `ending_asks_for_done_once_and_the_default_never_does`). Red first: the unwritable case failed with `Read` (a file as config dir); reworked to a 0o555 `graft` folder.
- J11: `update::tests::first_launch::{shown_until_the_palette_opens, gone_after_five_launches}` (`?`, `⌃P`, `:`; line gone the same frame; `Command::FirstLaunchDone(dir)` emitted); the write: `event_loop::tests::the_loop_writes_first_launch_done_and_says_a_failure`; real binary: `graft::it first_launch::a_terminal_launch_counts_and_shows_the_line_until_the_palette` (pty: file `1`, then `done`).
- Priority: `update::tests::first_launch::a_problem_or_notice_wins_the_row` (Failed, Waiting). Red first: it failed when drawn through a settling key.
- Empty state: `graft_ui::empty_state::tests::first_launch_sits_above_the_rule_when_free`, `graft::empty_state::palette_tests::opening_the_palette_ends_the_first_launch_line` (module is `palette_tests`).
- Widths: `graft_ui::empty_state::tests::first_launch_never_draws_a_fragment` — shipped keymap only; the unbound-key drop is in `first_launch_words` but **not tested** with an unbound keymap.
- Failure: `update::tests::first_launch::a_broken_file_warns_once_and_never_stops_the_app` (folder in place of the file; `x7`).
- Headless: `graft::it first_launch::headless_never_counts` (real binary, piped).
- Captures unchanged: every existing capture is drift-checked in the suite (`graft_fixtures::check`), all passed; the only new files under `docs/shots` are the two below. pty walks seed `done` in `pty::Isolated::new()` (`Isolated::first_launch()` opts out); Python scripts in `graft_sandbox.sandbox_env` (`first_launch=True` opts out; `scripts/check-scripts.sh` ok and checks the seed). Isolation existed (m8/22), so the stop rule did not fire.
- Captures: `graft::it first_launch::first_launch_captures` → `docs/shots/m8/first-launch-120x36.txt`, `docs/shots/m8/first-launch-empty-state-120x36.txt`. Colour unverified; his to judge.

**Deviations from Signatures (said, not silent):**
- `EmptyStateView::first_launch` is `bool`, not `Option<SanitizedText>`: the widget fits the words to its width (`graft_ui::empty_state::first_launch_words`); a prebuilt string could only be cut into a fragment.
- `Arrival` carries `FirstLaunch` (shared `Arc` + dir), not `bool`: `done` in one place must hide the line in every later place, and `drive` cannot see the graph's state after `run_repo`.
- Added `for_this_launch` (drive's one call; returns the warning) and `FirstLaunch::end() -> Option<PathBuf>`.
- `update` does not write the file (`update_cannot_reach_the_world` forbids `std::fs`): new `Command::FirstLaunchDone(PathBuf)`, carried by the loop like `SaveLayout`; a failed write is `Status::Failed(words)`.

**For the user / conductor:**
- R7's "Ready fallback" means the line **replaces the key hints** on the graph's last row for up to 5 launches (until the first palette open). It yields to problem, search, notice, stop, compare line and reveal (`view/frame.rs::status_row_notice`). It also shows on the `opening…` frame.
- Tour clause: no tour row exists (`rg -i tour crates/graft/src/update/palette` → nothing), so never drawn.
- A count-read warning is `Status::Sticky`, like the `recent:` warning — it stays until replaced, not cleared by the next key.
- The graph capture shows ages `7y` from the Linear fixture; it drifts when that turns 8y (regenerate).
- G-rename: config file name `launches`.
