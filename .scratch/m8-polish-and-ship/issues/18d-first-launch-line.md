# 18d — the first-launch line

Status: ready (conductor 2026-10-08)
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
- [ ] `count_launch`: absent → `true`, file `1`; `4` → `true`, `5`; `5` → `false`, `6`; `done` → `false`, file
      unchanged; `x7` → `Garbled`; unwritable dir → `Write`. (`first_launch::tests::count_launch_table`)
- [ ] J11 = 0: launch 1 shows the line on the graph status row; `?` → palette opens, `launches` = `done`, line gone
      the same frame; same for `⌃P` and `:`; launch 6 gone with no use.
      (`update::tests::first_launch::{shown_until_the_palette_opens, gone_after_five_launches}`)
- [ ] Priority: a problem and a notice each replace it; it returns when they clear.
      (`…::first_launch::a_problem_or_notice_wins_the_row`)
- [ ] Empty state: shown above the rule at 120×36 with 3 recents; not drawn when the list fills that row; `?` there
      writes `done`. (`graft_ui::empty_state::tests::first_launch_sits_above_the_rule_when_free`,
      `graft::empty_state::tests::opening_the_palette_ends_the_first_launch_line`)
- [ ] Widths 20..=200: every drawn clause whole. (`graft_ui::…::first_launch_never_draws_a_fragment`)
- [ ] Failure: unreadable file → line not shown, one status warning with `words()`, app runs. 
      (`…::first_launch::a_broken_file_warns_once_and_never_stops_the_app`)
- [ ] Headless `run` leaves `launches` absent in a temp config dir. (`…::first_launch::headless_never_counts`)
- [ ] Every existing capture unchanged: re-shoot diff shows only the two new files. (diff, in NOTES)
- [ ] Captures: `docs/shots/m8/first-launch-120x36.txt` (graph), `docs/shots/m8/first-launch-empty-state-120x36.txt`.
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
