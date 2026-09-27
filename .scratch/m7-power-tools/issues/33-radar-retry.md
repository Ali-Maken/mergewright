# 33 radar-retry — a failed radar read is asked once, and says why

Status: draft
Blocked by: none by content. Crates graft + graft-ui: the conductor sequences it against whoever holds them.
Crates: graft (`update/radar.rs`, `view/radar.rs`, `tests/it/app_harness.rs`, `tests/it/budget_run.rs`, their tests,
`docs/shots/m7/`, MAP.md) + graft-ui (`detail.rs` `RadarLine`, `detail_pane/header.rs`, tests, MAP.md).
Build with: **Sonnet**. A one-condition cause fix, one new line of words on a decided pattern, a test-harness guard.
Every decision is below.
Scout: none. The planner checked every site named here against `main` (2026-09-27); re-read each before editing.
Needs: none.
Split by: planner, 2026-09-27, from 20f's NOTES "Shallow capture — blocked" (second bullet).

## The defect (verified, rule 7)
- `update/radar.rs:331`: `request_context` asks again when the context is `Loading::Failed(_)`.
- `update/mod.rs:178`: `settle` calls it at the end of **every** `update` — including the update that lands the
  failure (`context_landed`, `radar.rs:369-372`). So each failure dispatches the next read at once: an unbounded
  retry loop (rule 2). 20f's test spun at 100 % CPU for 9 min.
- The failure is never shown: `view/radar.rs:63` draws no finding on `Failed`, and no line says the radar didn't run.
  He reads "no findings" as "nothing wrong" (rule 3: a swallowed error, on screen).
- Why the test never timed out: `tests/it/app_harness.rs:204-220` `pump` waits `PATIENCE` **per event**, with no
  total deadline; a loop that keeps producing events never trips it. Same shape in `budget_run.rs:624-643`
  `pump_until_from`.

## Ruling
**Cause:** "failed" was treated as "not asked yet". A failed context is an answer. It stays `Failed` until he refreshes
(`Ctrl+R` → `reset_for_refresh`, `radar.rs:551`, sets `Idle`), exactly as the duplicates probe already does
(`radar.rs:519`: asked only from `Idle`). No retry count, no backoff, no timer: nothing is retried behind his back.
- `request_context` asks only from `Loading::Idle`. The doc at `radar.rs:319-325` and graft MAP.md `request_context`
  ("nothing is already `Pending`/`Ready`") are corrected to say `Failed` waits for `Ctrl+R`.
- **Words for him** (invented; pattern taken from the existing `evil merge: not checkable — <reason>` and
  `force-pushed: not checkable — <reason>` lines, graft-ui `detail_pane/header.rs:57-62`). While the context is
  `Failed(why)`, every commit's radar lines start with:
  `radar: not checked — <why> · ctrl+r to try again`
  `<why>` is the job's string, `sanitize_line`d. Only while `Failed`: never on `Idle`/`Pending`/`Ready` (a status must
  not read as an error when nothing is wrong). The evil-merge line, which does not need the context, still shows.
  `Ctrl+R` is the refresh key (`docs/design/02-keymap.md:178`); no key is added.
- **Audit the other tail-of-update requesters** in `settle` (`update/mod.rs:168-189`): `request_force_push_probes`,
  `request_duplicates_probe` (fine: `:519`), `journal::request`, `remote::request_at_launch`, `detail::open_explain`,
  and `probe_evil` via `selection::follow_selection` (`update/detail/ops.rs:184-186` re-probes on `Failed` for the same
  commit — whether that runs on every update is **unverified**; check where `selection.rs:117` is reached). For each:
  re-asks on failure every update, yes/no, with the line. A yes → the same fix in this ticket if it's in graft; say so.
- **Test harness:** `app_harness::pump` and `budget_run::pump_until_from` gain a total deadline (a constant beside
  `PATIENCE`, e.g. 60 s) and fail naming what they waited for and the last 10 events. This is a test guard, not a
  product retry cap. The four local copies (`commit_editor_writes.rs:124`, `changes_pane_writes.rs:113`,
  `m4_walk/run.rs:219`, `m5_walk/run.rs:475`) call the shared one or gain the same guard; name each.

## Read first
- graft MAP.md `## update::radar`, `AppState::radar`, `Loading`; graft-ui MAP.md `RadarLine`, `DetailHeader`.
- Source, only: `update/radar.rs:319-373`, `:500-560`; `update/mod.rs:166-190`; `view/radar.rs` (where commit radar
  lines are built, `:170-209`); graft-ui `detail.rs:100-121`, `detail_pane/header.rs:45-65`.

## Signatures (exact)
```rust
// graft::update::radar — signature unchanged; asks only from `Loading::Idle`
pub fn request_context(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut dyn FnMut() -> JobId);

// graft-ui::detail::RadarLine gains
/// The radar's default-branch context could not be read, and why: no finding that needs it was checked.
NotChecked(SanitizedText),
// graft-ui `radar_text`: "radar: not checked \u{2014} {why} \u{00b7} ctrl+r to try again"
```

## Acceptance (name the verifying test at tick time)
- [ ] **Red first**, through `update` (not by calling `request_context` directly):
      `a_failed_radar_context_is_not_asked_again` — land `Product::RadarContext(Err(..))` for the pending id; the
      commands from that update and from 5 following unrelated events (e.g. `j`) hold no `Command::RadarContext`.
      Red today.
- [ ] `ctrl_r_after_a_failed_context_asks_once` — then `Ctrl+R`, the refs answer: exactly one `Command::RadarContext`.
- [ ] `a_failed_context_says_radar_not_checked` (graft view) and graft-ui `radar_text` unit test: the exact words
      above; absent for `Idle`, `Pending`, `Ready`.
- [ ] The audit: one line per requester, with its line and verdict. Any fix has its own red-first test.
- [ ] Harness: `pump_fails_on_a_stream_that_never_settles` — a stub that emits events forever makes `pump` fail within
      the deadline naming the wait. Show it red (hang → killed) is not required; show the old code has no deadline by
      reading, with the line.
- [ ] Capture `docs/shots/m7/radar-not-checked-120x40.txt`, drawn by `view::view` via `frame_at` (as 20f's), state
      with the context `Failed("commit b32274b is not in the repository")` and a commit selected. **Right:** the detail
      pane's radar block starts `radar: not checked — commit b32274b is not in the repository · ctrl+r to try again`;
      the graph's marker column shows no marker. Colour unverified (rule 5).
- [ ] MAP.md: graft `request_context` (the rule), graft-ui `RadarLine::NotChecked`.
- [ ] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest, `test --doc`.

## Touches (check each; NOTES says how)
- **`RadarLine` matches** in graft and graft-ui: grep; each exhaustive one gains the arm. The existing radar
  snapshots/captures stay byte-identical (none has a failed context — grep `docs/shots` and the insta snapshots).
- **Radar consumers of `context`:** `request_force_push_probes`, `request_duplicates_probe` (gated on `Ready`, stay
  quiet on `Failed`), `refresh_ref_findings`, `update/map.rs` `request` (`previous_default_tips` empty unless `Ready`),
  `view/radar.rs:63` findings. Unchanged; show by test or grep.
- **Other screens drawing radar state:** the marker column, the map's radar marks, `.` context menu `marked`
  (graft MAP.md `open_for_history_row`): no marker on `Failed`, as today. Only the detail pane gains words.
- **Keymap:** none (`Ctrl+R` exists). **Journal/undo:** none (read-only).
- **For the conductor (question, not a defect):** after a write that moves the default branch, the context is not
  re-read — `reset_for_refresh`'s only caller is the `Ctrl+R` path (`update/keys.rs:479`). Whether that is intended is
  not established here; not this ticket.

## Size
~35k. One context.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.
