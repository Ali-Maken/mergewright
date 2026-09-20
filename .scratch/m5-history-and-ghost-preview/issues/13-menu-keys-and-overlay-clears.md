# 13 — the menu's advertised keys, and overlays that own their rectangle

Status: done (uncommitted; conductor commits)
Blocked by: none (file-fenced against group 09)
Build with: **Opus** — the two defects group 12 found and correctly left alone,
plus the capture determinism and a golden snapshot, filed by the user.

Scout used: `Explore`, sonnet, for the key chain only (`keys.rs` → `context_menu.rs`
→ `graph_key`, and every read of `context_menu` state). The rest was read directly:
four `set_style` sites in one file, two widget `render` bodies.

## Fence

Mine: `crates/graft/src/update/context_menu.rs` (+ `context_menu/`),
`crates/graft/src/view/frame.rs`, `crates/graft/tests/it/preview.rs`,
`crates/graft-ui/src/context_menu.rs`, `crates/graft-fixtures/**`, and the crate
holding the date sort. Group 09 holds `update/keys.rs`, `update/undo.rs`,
`update/banner.rs`, `update/journal.rs`, `update/remote.rs`, `update/confirm.rs`,
`update/detail/keys.rs`, `update/mod.rs`, `view/rows.rs`, `view/journal.rs`,
`jobs/*`, `event_loop/*`. Nothing outside mine was touched.

---

## Job 1 — a verb key the menu advertises is live

`docs/design/02-keymap.md` §2.1: the `.` menu claims `j`/`k`/`Enter`/`Esc` and
nothing else; a key it displays must work when pressed.

**Fixed in `update/context_menu.rs` alone — `keys.rs` was not needed.** The
scout established why: nothing after `keys.rs:64` reads `next.context_menu` on
the key path (`graph_key`/`write_key` never look at it), and the one guard that
does — `remote::cancel_operating`'s `context_menu.is_some()`, `remote.rs:440` —
sees the `None` the handler has already written. So `false` is enough: the menu
closes *and* the key falls through, in the same call.

`context_menu_key` now returns `true` only for `j`/`k`/arrows/`Enter`/`Esc`.
Every other key sets `context_menu = None` and returns `false`.

### What the chain actually does, per key

Measured through the real app (`it::preview`), not reasoned:

| key | with the menu open | verified by |
|---|---|---|
| `y` copy SHA | hands on (menu closes, key falls through) | unit only — the test harness cannot run `CopyToClipboard` ("terminal output, never a job", `jobs/runner.rs:845`) |
| `x` compare | **live**, screen changes | `it::preview::every_verb_…` |
| `m` merge | **live**, lands on the same frame a bare `m` lands on | `it::preview::a_verb_key_the_menu_advertises_…` |
| `r` rebase | **live**, screen changes | `it::preview::every_verb_…` |
| `u` undo | **live**, screen changes | `it::preview::every_verb_…` |
| `f` fetch | hands on | unit only — pressing it starts a real network job, not fast tier |
| `p` pull | hands on | unit only, same reason |
| `P` push | hands on | unit only, same reason |
| `⏎` | claimed by the menu — dispatches the selected item | pre-existing `choose` helper |
| `Esc` | claimed by the menu — closes, runs nothing | `it::preview::esc_closes_the_menu_and_runs_nothing` |

Before the fix **every** row above except `⏎` read "swallowed".

For `f`/`p`/`P` the dispatch equivalence (menu item ⇒ same function the key
calls) was already pinned by
`update::context_menu::tests::the_menu_runs_what_the_dropped_keys_run`; what
this ticket adds for them is only that the key itself is handed on.

### Consequence the conductor should know about

`.` is not a verb and is not one of the four the menu claims, so it now falls
through too: it closes the menu and `graph_key` immediately reopens it for the
same row. Observed: cursor on item 1, press `.`, menu still open with cursor
back at 0. **`.` no longer toggles the menu shut** — `Esc` is the documented
closer. This follows from §2.1 read literally ("those four and nothing else");
it is not pinned by a test, because claiming `.` or not is a spec call, not a
build one. `q` also falls through, which is right: it quits.

### Stale comment left for the conductor to file

`crates/graft/src/update/keys.rs:61-63` still reads "A menu open … is modal:
its own keys win over the detail pane's and the graph's alike." That is now
false, and `keys.rs` is group 09's file, so it was left alone.

---

## Job 2 — an overlay owns the rectangle it is tinted into

**The cause is the tint, not any one widget.** `frame.buffer_mut().set_style`
repaints colour and leaves every cell's *symbol*, so tinting a rectangle
`SurfaceRaised` is a claim of ownership that nothing backs. The widgets happen
to blank their own rows at their own size — which is why the normal path looked
right — but their degraded paths do not, and the leftover cells show the graph
row underneath wearing the overlay's background.

Fix: one private helper `view::frame::clear_and_tint(frame, clear, tint, theme)`
— `Clear` first, then the tint. All four overlays go through it. `clear` is
wider than `tint` only for the `.` popup, which keeps group 12's band.

### The sizes, checked rather than assumed

| overlay | smallest size it still draws at | what it shows there | stale before this fix? |
|---|---|---|---|
| Ghost Preview | any rect ≥ 1 row | `preview needs 40x12 · esc closes`, padded to the full pane width | **no** — the user's report does not match the code. `preview_box_area` caps the degraded rect at exactly 1 row (`usable.min(1)`) and `fit` pads it, so there is never a second row to go stale. The clear here is prophylactic. |
| credential prompt | any rect ≥ 1 row | `too small for the prompt`, **centred and deliberately not padded** | **yes** — both ends of the tinted row kept the graph. Reachable whenever the terminal is under 40 columns or under 7 rows. |
| confirm box | 2 usable rows | below that the widget returns having written **nothing** (`confirm.rs:177`) | **yes** — the whole tinted row was the graph, recoloured. |
| `.` popup | unchanged | unchanged | fixed in group 12 |

**Nothing was fixed by refusing to draw.** One case remains where an overlay
draws nothing at all: the confirm box at fewer than two usable rows (a terminal
2 rows tall). With the clear it is now a blank tinted row instead of a
misleading one, but the user still cannot read what they are confirming. Making
it degrade to a one-line warning like the other two belongs in
`crates/graft-ui/src/confirm.rs`, which is outside this fence.

**Still open, not fixed:** the confirm box and the credential prompt are centred,
so graph content survives *beside* them — the band problem group 12 fixed for the
popup. Left alone deliberately: band-clearing a centred box changes what the
screen looks like, which is the user's call, not a build decision.

---

## Determinism, and the golden snapshot

**The test half.** `forked()` committed through `app_harness::git_as_graft`,
which fixes the identity and leaves the clock alone. Five commits in a burst all
land in the same second; the tie-break is the commit id, and a wall clock gives a
different id every run — so the rows, and the captures taken from them, came out
in a different order each time. `forked()` now stamps a fixed date per commit
(one minute apart) through a new `graft_fixtures::git::run_at`, and the capture
draws its ages against `Clock::Fixed(FORK_NOW)` so it does not age either.
Verified: three fresh runs of the capture test, each building its own scratch
repository, produce byte-identical files.

**The product half — it does not exist, and the code is the evidence.** The date
sort is `graft_git::history::node::Key` (`crates/graft-git/src/history/node.rs:135-141`):

```rust
self.time.cmp(&other.time).then_with(|| other.id.cmp(&self.id))
```

time, then the *smallest* commit id, documented against DESIGN.md §3.2 with
exactly the reason the user gives ("commit dates tie constantly in fast-import
repositories, in rebases and in scripted history"). All three walks order by it:
`sorted.rs:55` (`ready: BinaryHeap<Key>`), `simple.rs:66` (`Queued`),
`topological.rs:55` (`Candidate`); `divergence.rs:134` uses a
`(i64, Oid, CommitHandle)` tuple, likewise total. **Two commits sharing a second
in a real repository therefore keep the same order between launches** — the
commit id does not change. Nothing was changed here. The wobble was entirely the
fixture's ids changing run to run, which is the test half above.

**The snapshot.** `capture_context_menu_for_docs_shots` was `#[ignore]`d and
asserted nothing. It now runs in the suite and asserts the frame equals what is
committed under `docs/shots/`, with `GRAFT_UPDATE_SHOTS=1` to regenerate on
purpose. Both captures regenerated: 3460 B and 4051 B (cap 200 KB).
**Colour ships unverified** — TestBackend records a character grid with no colour
attributes (rule 5). **Not ticked as looking right: the user judges by eye**
(rule 6). The frames changed shape, because fixed dates put `main`'s row third
instead of wherever that run's ids happened to put it.

---

## Mutations — what the tests pin, and what they miss

Each mutation was applied to the final tree and the **workspace** suite run.
Baseline: 2120 passed.

1. **Job 1 reverted** (`_ => close` then an unconditional `true`): **3 failed**
   — `update::context_menu::tests::context_menu_key_closes_and_hands_a_verb_key_on`,
   `it::preview::a_verb_key_the_menu_advertises_closes_it_and_runs_the_verb`,
   `it::preview::every_verb_the_menu_advertises_is_live_while_it_is_open`.
   `esc_closes_the_menu_and_runs_nothing` and
   `the_context_menu_is_opaque_on_every_row_it_draws` stayed green, correctly:
   `Esc` behaves identically either way, and the defect was never about drawing.

2. **`Clear` removed from `clear_and_tint`** (so no overlay clears anything):
   **4 failed** —
   `view::frame::tests::the_confirm_box_never_shows_what_was_under_it`,
   `view::frame::tests::the_credential_prompt_never_shows_what_was_under_it`,
   `it::preview::the_context_menu_is_opaque_on_every_row_it_draws`,
   `it::preview::capture_context_menu_for_docs_shots`.

   Because each test exercises one site, that attributes cleanly: confirm 1,
   credential 1, popup band 2, **preview 0**. `the_preview_warning_row_is_opaque_
   when_the_box_cannot_fit` stayed green with the clear gone — which is the
   evidence for the table above that the preview never had a stale case to
   begin with. It is a guard, not a conviction, and it is named here as such.

3. **`forked()` reverted to wall-clock commits**: the golden snapshot failed
   6 runs out of 6. Honest caveat: that mutation changes the ages as well as the
   order, so it convicts the two together. I could not isolate the *order* half,
   because any fixed date — even one shared by all five commits — also fixes the
   ids, and fixed ids are exactly what makes the order stable.

**What the new tests pin, and what they do not.**

- `context_menu_key_closes_and_hands_a_verb_key_on` pins the **contract**
  (`false` + closed) for all eight advertised letters. It does not prove any of
  them then runs — a downstream handler could be missing and this would stay
  green.
- `every_verb_…` and `a_verb_key_…` pin the **effect**, through the real app, for
  `x m r u` and for `m` respectively. They are the ones that would catch a verb
  that falls through into nothing. They cover five of the nine; `y f p P` are
  contract-only, for the harness and network reasons in the table.
- The two `frame.rs` unit tests pin that **the overlay's own rectangle holds
  nothing that was under it**, at six sizes each including the degraded ones.
  They do not pin what survives *beside* a centred box — that is the open item
  above, and they would stay green if it got worse.
- `the_preview_warning_row_is_opaque_…` pins a property that is currently true
  for a second reason (the rect is one row and `fit` pads it). It would not
  notice the clear being removed.
- The golden snapshot pins the whole frame, so it catches almost any drift — and
  will therefore go red on unrelated deliberate changes. `GRAFT_UPDATE_SHOTS=1`
  is the intended path; the failure message says so.

## Gates

Workspace-wide, isolated worktree and target dir: `cargo fmt --check` clean,
`cargo clippy --all-targets --message-format=short -- -D warnings` clean,
`cargo nextest run` **2120/2120** (2111 on main, +9), 47.0 s warm,
`cargo test --doc` ok.

MAP.md: `crates/graft/MAP.md` for `context_menu_key`'s new contract and for
`clear_and_tint`; `crates/graft-fixtures/MAP.md` for `run_at`.
`crates/graft-ui/MAP.md` needed no change — nothing in `graft-ui` was modified.
