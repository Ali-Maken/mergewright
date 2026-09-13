# m1-the-graph/35c — Gallery loses keys (one stdin reader per run); then the captures

Status: ready
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
- [ ] Test red before fix, green after; pty run steps all 23 stops without a lost key (both tiers).
- [ ] `docs/shots/35-<repo>-{unicode,ascii}.txt` for all stops, each naming its own stop; `docs/shots/35-gallery-two-steps.txt`.
- [ ] Matrix re-run and committed; MAP.md updated.
- [ ] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam: (a) fix + test, (b) captures + matrix.
