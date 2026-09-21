# Review: M5 groups 12 + 13 — the `.` context-menu overlay

Reviewed at `836eb6c` (union of merges `276e3f8` group 12 and `836eb6c` group 13),
read-only, in a detached worktree under `/tmp` with its own `CARGO_TARGET_DIR`
(both deleted afterwards). Tickets:
`.scratch/m5-history-and-ghost-preview/issues/12-context-menu-overlay.md`,
`13-menu-keys-and-overlay-clears.md`. Not judged: how the captures look (rule 6);
colour (rule 5, unverified by construction).

Already filed elsewhere and **not** re-reported: `.` not closing the menu it
opened (ticket 15); the §2.1/§2.6 `M` contradiction; the confirm box drawing a
blank tinted row below 2 usable rows; confirm/credential being centred rather
than band-cleared (user's call, by eye).

## Counts

Critical 1 · Major 3 · Minor 5. Ticks spot-checked: 12, held 11, false 1.

## Gates (worktree at 836eb6c, fresh target dir)

| gate | result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo test` | 2154 passed, 0 failed, 34 ignored — 125 s wall **cold** (includes the compile), 740 MB peak RSS |
| `cargo test --doc` | ok |
| `cargo nextest run` (warm, for per-test timings) | see "Heavy tier and timings" below |
| heavy tier | see below |
| release perf | see below |

`target/` after the fast gates: 3.1 GB.

## Findings

### Critical

**C1. The `(key)` rule is half-applied, and the report says it is whole.**
`crates/graft/src/update/context_menu/verbs.rs:321`:

```rust
&format!("Fix: revert the merge (-m {mainline})"),
```

Group 12's report (ticket 12, §3) says it found *three* parenthesised
qualifiers, "fixing only the two would have left the rule half-applied and the
tick false", and changed all three. There is a fourth. The marked-merge row's
first Fix item still wears the shape that, by the rule the same ticket wrote
into `docs/design/02-keymap.md` §2.1 and `crates/graft/MAP.md` (`update::context_menu::verbs`:
"a qualifier that is not a key — a reset mode, `detached`, `-m 1` — is written
after a comma, never parenthesised"), means "this is a live key". The MAP.md
sentence is false as written. Worse, the suite pins the forbidden shape:
`verbs/tests.rs:348` and `:398` assert `"Fix: revert the merge (-m 2)"`, so the
tests cannot go red on this.

Why Critical: a false tick is Critical whatever its size (CLAUDE.md). By the
builder's own standard the §3 tick is false. Practical impact is small — nobody
reads `-m 2` as a key — but the rule was the whole point of the change.

Fix: `Fix: revert the merge, -m {mainline}`; update the two assertions and the
MAP.md sentence; also `docs/design-notes/m5-history-and-ghost-preview.md:245-246`
still quotes every old parenthesised form (`(mixed)`, `(hard)`, `(-m 1)`).

### Major

**M1. Right-click menu + fall-through runs the verb on the wrong row.**
`crates/graft/src/update/mouse.rs:76-84` opens the menu for the *clicked* row
(`open_for_history_row(next, row)`) without moving `history.cursor`.
`crates/graft/src/update/context_menu.rs:138-141` (group 13) now hands every
verb letter down to `graph_key`, which acts on the **cursor** row. So the label
promises one row and the letter acts on another.

Reproduced through the real `update` in the review worktree (probe test,
deleted with the worktree): `forked()` fixture, cursor on row 0 (`feature`'s
tip), right-click on `main`'s row (menu.row = 2, labels include
`Merge main into feature (m)`), press `m`:

```
menu closed: true   preview: None   status: Sticky { reason: "already on it" }
```

versus `.` then `m` on the same row: preview of `Merge main into feature`.
Same mechanism for `y` (copies the cursor row's sha, silently, while the menu
said `Copy SHA (y)` for the clicked one), `x`, `r`. Before group 13 the keys
were swallowed — wrong, but harmless; now they run against the wrong target.
Not data loss (`m`/`r` end in a preview; `y`/`x` are not writes), but a wrong
result the user cannot see coming, which is why it is Major and the conductor
may reasonably escalate it.

Fix the cause: make the menu's row and the cursor agree. Either a right-click
moves the cursor to the clicked row before opening (the design's own model —
§2.1 "applied to the cursor where the verb has one"), or the fall-through in
`context_menu_key` moves `history.cursor`/`on_working_tree` to `open.row`
before returning `false`. Pin it with an `it::preview` test that right-clicks a
row away from the cursor and presses `m`, asserting the preview names that
row's branch. (`App` has no mouse helper today; the probe added a five-line
`probe_mouse` next to `press` in `app_harness.rs`.)

**M2. Files well past the 400-line rule, and these groups grew one by a third.**
PROMPT.md §6.2: files under about 400 lines. `crates/graft/src/view/frame.rs`
is 869 lines (group 12 +125, group 13 +193, mostly an inline `mod tests` with
its own dirty-screen harness); `crates/graft/src/update/context_menu.rs` 925;
`crates/graft/tests/it/preview.rs` 1022. The overage predates these groups,
but the pattern for the fix already exists in the same crate
(`update/context_menu/verbs/tests.rs`): move `view::frame`'s tests to
`view/frame/tests.rs`, and split `it/preview.rs` into `preview.rs` and
`context_menu.rs` — the context-menu tests are a distinct section already.

**M3. The band-with-a-split is pinned only by geometry units, never through a
frame.** `overlay_band`'s three unit tests check arithmetic on hand-made rects.
`primary_pane_area` (frame.rs) is what decides *which* half of the split the
band runs to, and nothing exercises it: `the_context_menu_is_opaque_on_every_row_it_draws`
runs with `detail.shown == false` at both sizes, and the golden snapshot too.
Swapping `split_area(...).0` for `.1` (clearing across the detail pane instead
of stopping at it) would pass the whole suite. Add a third case to the opacity
test with the detail pane open at 160x48 (`DetailLayout::Right`), asserting the
menu rows end at the label **and** the detail pane's first column is untouched.

### Minor

**m1. `GRAFT_UPDATE_SHOTS=1` turns a regression green.** Verified: with
mutation (c) below applied (band not widened), the normal run fails
`capture_context_menu_for_docs_shots`; the same run with the variable set passes
and rewrites both shots. That is the intended regenerate path, but a variable
left exported in a shell makes every later run green. Suggest: in update mode
write the file **and then still assert** (the diff is on disk for review and the
run is red until it is committed), or accept only the literal value `1`.

**m2. The 80x24 capture's header is a layout artifact.** Line 47 of
`docs/shots/m5-ui-context-menu-verbs-80x24.txt` reads `graft /code/api  8…`.
The header was fitted against the scratch repository's long temporary path and
the path replaced afterwards, so it shows a truncation a real `/code/api` would
not (`160x48` shows `8 commits  date order, sorting…`). Two consequences: the
caption's "nothing else is touched" is technically true and visually
misleading; and the snapshot is tied to the temp-path length of this machine —
on Linux (`/tmp/.tmpXXXXXX`) the header fits differently and the golden test
fails. **Unverified** (no Linux run). Also both frames are captured while the
status still reads `sorting…`; a "what right looks like" frame should be
settled. Suggest pumping until the sort lands, and either opening the repo at
a fixed-length path or dropping the path from what the header fits.

**m3. `every_verb_the_menu_advertises_is_live_while_it_is_open` proves
"something changed", not "the verb ran".** It asserts the frame after the key
differs from the frame after `Esc`. For `u` on this fixture the difference is
a refusal on the status line. Mutation (b) does turn it red, so it earns its
tick today; but a future generic "key not bound" status would keep it green.
Harden by asserting the specific effect per key (`x` → compare mark set, `r` →
`preview.is_some()`, `u` → the status mentions undo).

**m4. Four KNOWN_GAPS rows these groups closed are still open on `main`**
(rows 511 M5/08 overlays transparent, 512 M5/12 early-return stale, 513 M5/12
captures not reproducible, 514 M5/12 keys swallowed). Move to
`docs/gaps/closed.md` with the verifying tests named.

**m5. Stale prose.** `crates/graft/src/update/keys.rs:61-63` "is modal" (in
ticket 15 already); `docs/design-notes/m5-history-and-ghost-preview.md:245-246`
(see C1); `crates/graft/MAP.md` `verbs` entry (see C1).

## Spot-checks the conductor asked for

**`clear_and_tint` (item 2).** All four sites go through it — `draw_preview`
(frame.rs:448), `draw_context_menu` (:548), `draw_confirm` (:565),
`draw_credential` (:594) — and `grep set_style crates/graft/src/view/frame.rs`
finds exactly one call, inside `clear_and_tint` itself. No site reintroduces a
bare tint on a rect it then draws into. Only the popup passes a wider `clear`
than `tint`.

**Key liveness table (item 3).** Honest. `x m r u` and `m`-vs-bare-`m` are
observed through the real `update` (`every_verb_…`, `a_verb_key_…`), `Esc` by
`esc_closes_the_menu_and_runs_nothing`, and mutation (b) turns exactly those
three red. `y f p P` are claimed as contract only, and the contract test
(`context_menu_key_closes_and_hands_a_verb_key_on`) asserts only `false` +
closed + no command — it does not claim the verb ran. The one thing the table
does not say is M1: "live" was only ever tested from the `.` path.

**Golden snapshot (item 4).** Fails on drift: mutations (a) and (c) each turn
it red. `GRAFT_UPDATE_SHOTS=1` masks — see m1.

**Determinism (item 5).** Three fresh runs of the capture test in update mode:
80x24 `a71d68db…`, 160x48 `29216852…` all three times, and both equal the
committed files at `836eb6c`.

**Rule 3 (item 6).** Nothing in either diff swallows an error: `run_at` returns
`Result`; `dated_commit` panics with the reason; the snapshot's read panics
naming the path; no `let _ =`, no `unwrap_or_default()`, no empty `Err(_)`.
`unwrap()`/`expect()` only inside `#[cfg(test)]` and `tests/`.

**Mutations run (final tree at 836eb6c, `cargo test -p graft --no-fail-fast`):**

| mutation | red |
|---|---|
| (a) `Clear` removed from `clear_and_tint` | 4: `view::frame::tests::the_confirm_box_never_shows_what_was_under_it`, `…the_credential_prompt_never_shows_what_was_under_it`, `it::preview::the_context_menu_is_opaque_on_every_row_it_draws`, `it::preview::capture_context_menu_for_docs_shots` — matches the builder's table |
| (b) fall-through reverted (`_ => close`, `true`) | 3: `context_menu_key_closes_and_hands_a_verb_key_on`, `a_verb_key_the_menu_advertises_closes_it_and_runs_the_verb`, `every_verb_the_menu_advertises_is_live_while_it_is_open` — matches |
| (c) band not widened (`clear_and_tint(frame, rect, rect, …)` in `draw_context_menu`) | 2: the opacity test and the golden snapshot; the three `overlay_band` units stay green, as the builder said they would |

## Acceptance ticks spot-checked (12 checked, 11 held)

| tick | verifying test | would it go red? | held |
|---|---|---|---|
| 12 §1 menu opaque | `the_context_menu_is_opaque_on_every_row_it_draws` | yes — mutations (a), (c) | yes |
| 12 §2 capture writer asserts nothing → real test named | same | yes | yes |
| 12 §3 label shape, rule fully applied | `verbs/tests.rs` | tests pin the old shape for `Fix:` | **no — C1** |
| 12 §4 caption width-dependent | none named; checked both files by eye | n/a | yes |
| 13 `x` live | `every_verb_…` | yes — (b) | yes |
| 13 `m` live, same frame as bare `m` | `a_verb_key_…` | yes — (b) | yes (from `.` only — M1) |
| 13 `r`, `u` live | `every_verb_…` | yes — (b) | yes |
| 13 `y f p P` handed on (contract only) | `context_menu_key_closes_and_hands_a_verb_key_on` | yes — (b) | yes, honestly labelled |
| 13 `Esc` runs nothing | `esc_closes_the_menu_and_runs_nothing` | asserts `preview.is_none()` | yes |
| 13 confirm / credential own their rect | the two `frame::tests` | yes — (a) | yes |
| 13 preview warning row opaque | `the_preview_warning_row_is_opaque_…` | no — builder says so, "guard" | yes, as labelled |
| 13 determinism + golden | `capture_context_menu_for_docs_shots` | yes — (a), (c); 3 runs identical | yes |

## Heavy tier and timings

- **Heavy tier** `cargo test --features heavy-tests -- --test-threads=2`:
  2185 passed, 0 failed, 34 ignored; 278.8 s wall, 743 MB peak RSS — inside
  7.1's 10 min / ~3 GB.
- **Release perf** `cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1 --nocapture`:
  13 passed, 0 failed; 73.5 s of tests (142.6 s wall with the release build),
  560 MB peak RSS. Input-to-frame at 50k refs, 420 keys each: Pixel closed
  p50 4.9 ms / p99 8.2 ms / max 8.8 ms; Unicode closed p50 4.3 / p99 8.2 / max
  8.7 ms; Unicode diff-scroll p50 5.1 / p99 8.0 / max 8.4 ms; Unicode pane
  open p50 5.5 / p99 8.2 / max 8.4 ms — all under the 16 ms budget. First
  paint on 100k rows: 14 ms (commit-graph), 68 ms (50k refs), 81 ms
  (realistic), 101–102 ms (no commit-graph, provisional); bottom 52–62 ms with
  a commit-graph, 485–541 ms without; peak RSS 31–112 MB.
- **Fast suite** `cargo test`: 125 s wall cold (compile included), 740 MB
  peak RSS; `cargo nextest run` warm 74.2 s.
- Slowest five, benches, fuzz: *pending*.

## For KNOWN_GAPS.md

| item | state | expiry |
|---|---|---|
| Right-click menu + verb letter acts on the cursor row, not the menu's row (M1) | **broken** (reproduced) | M5 — before the acceptance group; the label promise is the milestone's |
| Golden snapshot depends on the temp-path length of the machine (header fit at 80 columns) | **unverified** (no Linux run) | M8 (publish) |
| `GRAFT_UPDATE_SHOTS` exported in a shell turns snapshot regressions green | **unverified** as a real incident; mechanism verified | M8 |
| Band-with-split (`primary_pane_area`) untested through a frame (M3) | **unverified** — reasoned correct from `split_area`'s `(graph, detail)` order | M6 |
