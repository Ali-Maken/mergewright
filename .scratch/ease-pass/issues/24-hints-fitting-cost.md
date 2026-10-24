# 24 — The hint bar re-measures on every step of its ladder

Status: done · Build with: Sonnet · Blocked by: 23 (merged)
Source: 23 NOTES (`.scratch/ease-pass/issues/23-fast-tier-speed.md:221-250`, `docs/perf/23-fast-tier.txt`).

## Need
The fast tier runs 387–391 s against 180 s. The biggest single cost is product code, not test code: 8 hints tests
(`graph_screen/hints/tests.rs:90`, `:1282`, ~212 test-seconds) and `update/tests/bar_keys.rs:56` sweep `hints_for`
(`graph_screen/hints.rs:119`), which clones and re-measures the bar on every ladder step (`hints/fitting.rs:129-216`).
The same path runs on every frame the bar draws, so it is also frame cost in the app.

## Scope
1. Measure first: `hints_for` cost per call (unit bench or timed test, script → `docs/perf/24-hints-fitting.txt`) and the
   8 tests' seconds.
2. Fix at cause: measure each entry once and fit the ladder from the cached widths (no clone per step). Output must be
   byte-identical: every existing hints/bar test and capture unchanged (none re-accepted).
3. Then probe the workspace profile: `[profile.dev.package.mergewright-ui] opt-level = 1` (and graph, ops) — record cold build
   seconds and fast-tier run seconds, before and after, two runs each. Adopt only if build + run is a net win on a cold and a
   warm build; else record and reject.
4. Fast tier measured twice with `scripts/23-fast-tier.sh` at the end.

## Acceptance
- [x] `hints_for` per-call cost 83.4/82.3 us before, 29.0 us after (dev profile): `scripts/24-hints-fitting.sh` -> `docs/perf/24-hints-fitting.txt` `== before ==` / `== after3 ==`.
- [x] Every hints/bar test unchanged and green, no snapshot accepted: `cargo nextest run -p mergewright-ui -p mergewright --no-fail-fast` 4254/4254 (`graph_screen::hints::tests`, `hints::spell_sweep`, `empty_state::tests::bar_keys_follow_the_form`, `update::tests::bar_keys`), `git status` shows no .snap change.
- [x] opt-level probe recorded, REJECTED: `scripts/24-opt-probe.sh` + `scripts/23-fast-tier.sh` (`BUILD` lines in `docs/perf/24-hints-fitting.txt`, `24-opt1-*` in `docs/perf/23-fast-tier.txt`).
- [x] Fast tier two runs recorded (355.2 s / 357.4 s, slow_over_10s 6 / 6): `scripts/23-fast-tier.sh 24-after-1`, `24-after-2`. NOT at 180 s; not this ticket's bar.
- [x] Five gates + `scripts/walks.sh`; Touches: (see NOTES).

## NOTES (builder)
No scout (small, one fitter). Did not compact.

**Cause (rule 2: cause, not symptom).** `fit_list` cloned the whole slot vector and re-ran `hints_text` (spell every key, build a String, measure) per ladder step; `spelled_bar` re-spelled the whole list a second time to test the doorway. Fix: the row width is the sum of its hints (every join is one separator), so `Row` adds/removes one hint's cells per step; each key measured once per call, rungs measured without cloning (`Hint::verb_at`, `fitting::{key_width,width_at}`). `doorway::spell` returns the labels so the doorway test reuses them. Output byte-identical: same tests, no snapshot touched. Cost to user: none.

**Numbers (dev profile, 683k calls, `docs/perf/24-hints-fitting.txt`).** hints_for 83 us -> 29 us per call (2.8x). Of the 29: `spelled_bar` ~16 (whole_bar ~7.7, label Strings), fit ~13. The 8 tests (s, run with 4 neighbours): no_context_hides_every_key 19.8 -> 7.1; a_words_only_bar 20.1 -> 8.1; no_hint_loses_ground 36.3 -> 14.2; a_tail_hint_gains 37.4 -> 15.8; three_most_relevant 17.3 -> 6.7; bare_keys_are_a_tail 35.0 -> 13.9; never_draws_a_fragment 39.3 -> 18.2; empty_state bar_keys 0.02. In the full fast tier: 18.7 / 16.1 / 14.3 / 13.9 s, **still over 10 s** (3 of the sweeps are over 10 s even after the fix, 4 slow); `update::tests::bar_keys::every_key_the_bar_draws_is_answered` 10.1 s. Next lever, not done (out of scope): `whole_bar` and keymap `live_labels` allocate Strings per hint per call; caching spelled bars per (context, keymap) is a larger change.

**opt-level probe (mergewright-ui, -graph, -ops = 1): REJECTED.** Run 355.2/357.4 s -> 334.1/337.0 s (-21 s). Cold build 52.5 -> 82.8 s (+30); warm (touch ui lib.rs) 18.3 -> 54.2 s (+36). Cold build+run 408 -> 418 s; warm 374 -> 389 s. Net loss both. (Load ~5-6 throughout.)

**Fast tier:** after-1 355.2 s, after-2 357.4 s, slow_over_10s 6, 6 (was 387-391 s, 9). Ticket 23's 180 s target is still NOT met.

**Touches:** callers of `hints_for` (every footer/bar): same output, checked by the 4254 ui+mergewright tests incl. all bar snapshots/captures, none re-accepted. `spelled_bar` other callers: only `hints_for` (grep). `fits` kept for `stop.rs` and `place_footer`. `drawn_verb` now delegates to `verb_at`, same match. MAP.md: no public item changed. Keymap/journal/undo untouched. `over_400`: hint.rs 383 -> 389, fitting.rs 316 -> 393, both < 400; no row raised. `published_repo`: scripts/perf hold no machine paths (the probe scripts take the fresh dir as an argument). Scripts added: `scripts/24-hints-fitting.sh`, `24-hints-bench.rs`, `24-opt-probe.sh`; the bench is copied in and removed again, never built by the gates.
