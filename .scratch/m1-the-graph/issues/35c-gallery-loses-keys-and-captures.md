# m1-the-graph/35c — Gallery loses keys (one stdin reader per run); then the captures

Status: done (2026-09-13, Fable, no compaction)
Build with: **Fable** (high) — remainder of 35/35b (M1's acceptance ticket), same model
Blocked by: 35b
Leaves runnable: yes — every `]` in the gallery steps exactly once; captures for every stop under docs/shots/35-*.

**M1's promise:** *the shape of history is obvious at a glance.*

Why: 35b found, under a pty, that from stop 3 on `]` presses are lost at random: `event_loop::run` calls `InputMux::on_stdin()` per run and the mux's stdin thread never stops, so after k steps k threads race for each byte. The user can't walk the gallery reliably by eye either. Captures were blocked, not routed around.

## Read first
- Ticket 35b "Handoff" (cause, smallest fix, the test, exact capture commands)
- Ticket 35 "Guard (context)" and "Acceptance"
- `crates/graft-term/MAP.md`, `crates/graft/MAP.md`; KNOWN_GAPS "blink" row; scouts for source

## Scope
In:
1. Fix the cause: one stdin reader per process (35b's `OnceLock` + sender-swap), or split `run` into terminal setup + per-repo loop. Choose, and say why. Test first (two `on_stdin` calls; bytes after the second arrive only on the second `Inputs`). State whether the shipped binary could ever call `run` twice (if yes, it had the bug too).
2. Captures per 35b's handoff: both tiers, all stops, two-step capture, verify each file names its own stop (never cat), re-run `--matrix`, tick 35's capture boxes.

Out: snapshot gaps from the matrix (owners in KNOWN_GAPS).

## Guard (context, from the user)
Captures go straight to files under `docs/shots/35-*`. **Never print capture contents into your own context.** Report file paths and the coverage matrix only.

## Acceptance
- [x] Test red before fix, green after; pty run steps all 23 stops without a lost key (both tiers).
- [x] `docs/shots/35-<repo>-{unicode,ascii}.txt` for all stops, each naming its own stop; `docs/shots/35-gallery-two-steps.txt`.
- [x] Matrix re-run and committed; MAP.md updated.
- [x] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam: (a) fix + test, (b) captures + matrix.

## Built (2026-09-13, Fable, no compaction)

**Choice: one reader per process, not the `run` split.** `InputMux::on_stdin` starts the stdin reader on its first call (`static STDIN`) and every call returns a fresh `Inputs` (`InputMux::inputs`); the reader delivers to the current `Inputs` and *holds* events while there is none. Chosen over splitting `run` because the split changes the shipped binary's structure for a harness-only need; the blink stays (KNOWN_GAPS row updated). The shipped binary calls `run` once (`main.rs`), so it never had the bug.

**A second loss window, found by the capture's own `]q` ending:** keys the forwarder had already pushed past the loop when it stopped (`]q` in one read) died with the wake channel — the first capture run hung on its final `q`. Fixed at the cause: `run` → `hand_back_input`: `mux.hold()`, `Forwarder::finish` (join, bounded: the sender is gone), `ChannelWakes::unhandled_input` (the core events queued after the ending wake, in order) → `mux.give_back` (ahead of anything held since). Tests: `mux/tests.rs` (fresh `Inputs` takes over; keys with no listener go to the next; `hold`; `give_back` ordering), `event_loop::tests::the_input_a_finished_loop_never_handled_is_returned_in_order_without_the_rest`. Not unit-testable end to end (needs a tty); observed under the pty: 23 stops × 2 tiers, every footer seen, `]q` ending quits.

**Captures:** `python3 .scratch/m1-the-graph/pty-gallery.py <tier> $STOPS`, 80x24, 46 stop files + `35-gallery-two-steps.txt`; each verified by grep to name its own stop; ASCII files have `*` nodes and `|` lines, Unicode files `●`/`│`. The `◆` in ASCII files is the `[◆ main]` badge icon (a setting, not the tier). Matrix: every stop has `C` in unicode/dark and ascii/dark (size `?`: the file names don't carry it; the header line inside each file does). **By eye (user):** the captures and the gallery walk in Ghostty and VS Code; nobody here ticked "looks right".
