# m1-the-graph/35b — Graph gallery part b: TESTING.md entries and captures

Status: partial — seam (a) done (TESTING.md entries); seam (b) captures blocked by a gallery bug, see Handoff
Build with: **Fable** (high) — remainder of 35 (M1's acceptance ticket), same scope and model; defining what right looks like per repo is the judgment
Blocked by: 35
Leaves runnable: yes — TESTING.md says concretely what right looks like at every gallery stop; captures for every stop under docs/shots/35-*.

**M1's promise:** *the shape of history is obvious at a glance.*

Why: 35 compacted after part (a) (gallery command, footer expectations, availability, coverage matrix; commit 80bdb63). Part (b) remains, with exact steps in 35's "Handoff: part b".

## Read first
- Ticket 35: whole "Scope", "Guard (context)", "Acceptance", and "Handoff: part b" (the steps)
- `crates/graft/tests/it/gallery.rs::expectation` (the one-line expectations to expand)
- Ticket 01's fixture facts (`crates/graft-fixtures/src/shapes/`, via scout), PROMPT.md §1.1–1.3, §7.2 item 7
- Existing TESTING.md: section headings only, then "The manual terminal checklist"

## Scope
In: 35's handoff steps 1–3 exactly — TESTING.md "Graph gallery" section (one concrete entry per stop, per-terminal block), pty captures of all stops for Unicode and ASCII, the two-step capture, tick 35's remaining boxes, re-run `--matrix`.
Out: filling snapshot gaps the matrix shows (owners in KNOWN_GAPS); building synthetic-100k.

## Guard (context, from the user)
Captures go straight to files under `docs/shots/35-*`. **Never print capture contents into your own context** (check with `wc -c`, `grep -c`). Report file paths and the coverage matrix only.

## Acceptance
- [x] TESTING.md "Graph gallery": one entry per stop, concrete (rows, node shapes, badges, straight lanes, curves and colours, foxtrot vs normal merge, fold / first-parent / Focus, tier invariants); no "looks correct".
- [x] Per-terminal block with "walk the gallery" as the procedure.
- [ ] `docs/shots/35-<repo>-{unicode,ascii}.txt` for all stops; `docs/shots/35-gallery-two-steps.txt`.
- [ ] Matrix re-run and committed; ticket 35's boxes ticked except the user's by-eye box.
- [x] Gates pass (fmt, clippy, 27 suites green, 2026-09-13).

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam: (a) TESTING.md entries, (b) captures + matrix. Stop at the seam with a handoff note rather than compacting.

## Handoff (2026-09-13, Fable, no compaction)

**Done (seam a):** TESTING.md "Graph gallery" (before "Out of scope"): key legend, node glyphs per tier, badge forms, colour rule, 23 concrete entries in `--list` order, per-terminal "walk the gallery" block. `tests/it/gallery.rs::expectation(Tags)` now says what the screen shows (annotated tags peeled onto their commits; the badge does not mark them annotated). `.scratch/m1-the-graph/pty-gallery.py` written (waits for each stop's own footer `name · n/23 ·`, exits non-zero if it never comes).

**Blocked (seam b): the gallery loses keys.** Measured under a pty (`/tmp`-style probe, 80x24, 1 s / 3 s / 0.3 s settle): from stop 3 on, `]` presses are lost at random (e.g. lost, lost, seen; not time-bound). Cause: `event_loop::run` (crates/graft/src/event_loop/mod.rs ~line 208) calls `InputMux::on_stdin()` per run; the mux's stdin thread never stops (`crates/graft-term/src/input/mux.rs` line 33 says so), so after k gallery steps k threads race for each byte and only the newest run's `spawn_reader` delivers. A first capture run produced mislabeled frames (stop 4's frame under five names); those files were deleted, none is committed. A capture script that re-presses `]` would hide the bug, so it was not written.

**Exact next step (ticket 35c, before captures):** make the mux one per process. Smallest fix: `InputMux::on_stdin` keeps a process-wide reader (`OnceLock`) and a `Mutex<Sender<..>>` slot; each call swaps in a fresh channel's sender and returns its `Inputs`; the old sender drops, so the old run's forward thread ends without eating a key. Keys typed during the restart gap go to the old channel and are lost — say so in KNOWN_GAPS, or split `run` into terminal setup + per-repo loop (the KNOWN_GAPS "blink" row) which removes both. Test: two `on_stdin` calls on a socket pair, bytes written after the second arrive only on the second `Inputs`. Update graft-term MAP.md (`InputMux::on_stdin` semantics). Then: `cargo build -p graft --example gallery`; `STOPS=$(cargo run -q -p graft --example gallery -- --list | awk '{print $2}')`; `python3 .scratch/m1-the-graph/pty-gallery.py unicode $STOPS` (zsh: `${=STOPS}`), then `ascii`; verify with `for f in docs/shots/35-*-unicode.txt; do grep -o "[a-z0-9-]* · [0-9]*/23 ·" $f | head -1; done` (each file names its own stop; never cat); `cargo run -p graft --example gallery -- --matrix`; tick 35's capture boxes; commit.
