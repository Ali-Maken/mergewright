# Capabilities — M1 The Graph

M1 turns a bare repo-open into the thing the user actually lives in day to
day: a commit graph that draws lanes and merges, streams in without
blocking, degrades cleanly to whatever the terminal can draw (pixels,
Unicode box-drawing, or plain ASCII), stays fast and memory-bounded from a
handful of commits to a million, and gives enough interaction — scroll,
select, search, fold, focus, copy — to actually read a real history. A large
share of this milestone's tickets are not new capabilities but hardening of
a small number of them: the same "commits render as a lane graph" and "stays
under budget while scrolling" promises get re-proven at ever larger scale
and against ever more real repository shapes, and the rows below fold those
chains together rather than listing each fix separately.

**Invisible foundations / test-infra:** the `graft-fixtures` corpus (13
deterministic repository shapes plus real-scale synthetic ones, checked
with two independent builds compared byte-for-byte); the heavy test tier,
gated by Cargo features (`heavy-tests`, `heavy-1m`) rather than `#[ignore]`
or an environment variable (ADR-0018), so the standing `cargo test`/
`cargo nextest run` commands never silently skip anything; a dozen
"split this 400+-line file" refactoring tickets (70–76b, 79) that changed no
public API and no test behaviour; a purity-guard fix (78) and an update to
which files a lint scans, neither user-visible.

## Capabilities

### M1-01. The graph draws in the tier the terminal can actually support, and never in a worse one than necessary

```meta
id: M1-01
area: terminal-rendering
milestone: m1-the-graph
status: works
unverified_aspect: real tmux and real SSH rendering — tmux not installed on the build machine
```

- **How:** automatic at startup (reusing M0's capability probe); `F3`
  manually cycles down through the tiers and back; `graph.renderer` in
  config always wins over detection.
- **Behaviour:** tier order is Kitty (pixel, via the Kitty graphics
  protocol) → Unicode (box-drawing glyphs, the default/daily tier) → iTerm2
  (pixel, via iTerm2's own image protocol) → Sixel → ASCII. Inside tmux,
  every pixel protocol — iTerm2's environment-hint shortcut included — fails
  closed unless a re-sent, passthrough-wrapped probe actually answers;
  otherwise the tier is Unicode, never ASCII, because tmux draws Unicode
  glyphs fine even when it can't carry image bytes. A Sixel-capable terminal
  with no truecolor and outside tmux falls to ASCII (no Sixel encoder exists
  at all — a named, permanent gap, not a bug). `F3` cycles
  settled-tier → next lower → … → ASCII → back to the top; re-entering pixel
  re-transmits every image from scratch, freeing old ids first (no leaks).
  The footer shows `renderer: <tier> (F3 cycles)` whenever not at the top
  tier.
- **Proven by:**
  `graft::view::tests::every_graph_renderer_config_value_draws_as_it_says`
  `graft_term::detect::tests::tmux_too_old_to_pass_anything_through_falls_back_to_unicode`
  `graft_term::detect::tests::tmux_with_passthrough_reaches_the_outer_terminal`
  `graft_term::detect::tests::ssh_buys_the_terminal_an_extra_round_trip`
- **Looks like:** `docs/shots/19-ascii-tier-drop-key.txt`,
  `docs/shots/33-pty-tier-cycle.txt`, `docs/shots/08-unicode-graph-column.txt`
- **Decided in:** `docs/design/04-terminal-capability-detection.md` §4.2–4.3;
  ADR-0016.
- **Status:** works for the decision logic and byte streams (heavily
  tested); **unverified by eye** for real tmux and real SSH — tmux isn't
  installed on the build machine, so `KNOWN_GAPS.md`'s "tmux passthrough and
  SSH unseen" row (expiry: M1 checkpoint) is still open. Whether a pixel
  image lands correctly inside a tmux split pane, and whether tmux forwards
  the Kitty protocol's error replies, are both unknown.

### M1-02. Commits render as a lane graph — deterministic, gap-free, with merges drawn where they happened

```meta
id: M1-02
area: graph
milestone: m1-the-graph
status: works
```

- **How:** automatic, on every history walk.
- **Behaviour:** every parent edge is drawn; no edge is ever drawn through a
  node it isn't connected to; among the lanes waiting for a commit, it takes
  the lowest-indexed one (lane 0 whenever lane 0 is among them); a freed
  lane column is never reused on the same row that freed it, only the next
  one; a reserved lane (the default branch, or an integration branch) is
  never handed to another branch even while temporarily idle, and never
  closes with a dangling edge bent in from a column it doesn't own; an
  octopus merge opens one lane per extra parent. Two layout runs over
  exactly the same commits produce byte-identical output, including when
  dates tie exactly (broken by commit id). This holds under every chunking
  strategy the streaming layout might use — splitting the same history into
  different-sized chunks changes nothing about the rows produced.
- **Proven by:**
  `graft_graph::layout::tests::linear_chain_stays_in_lane_zero_for_its_whole_life`
  `graft_graph::layout::tests::root_commit_reached_by_two_branches_lane_zero_wins`
  `graft_graph::layout::tests::freed_column_is_not_reused_on_the_freeing_row_only_the_next_one`
  `graft_graph::layout::tests::reserved_lane_root_never_reused`
  `graft_graph::layout::tests::reserved_integration_lane_never_reused_via_convergence`
  `graft_graph::layout::tests::a_reserved_lane_closing_at_its_own_tip_bends_no_edge_in`
  `graft_graph::layout::tests::octopus_merge_opens_a_distinct_lane_per_extra_parent`
  `graft_graph::layout::tests::two_layout_runs_over_the_same_commits_are_byte_identical`
  `graft_graph::layout::tests::chunk_boundaries_do_not_change_the_rows_produced`
  `graft_graph::layout::tests::equal_dates_change_nothing_and_two_runs_still_agree`
  plus property tests `rows_replay_cleanly_from_the_outside`,
  `the_default_branch_is_lane_zero_and_straight`,
  `open_lanes_are_only_ever_touched_by_the_row_they_are_waiting_for`,
  `output_is_deterministic_and_independent_of_chunking`,
  `every_incoming_column_is_a_lane_the_row_draws`,
  `every_invariant_holds_under_every_inclusion` (`graft_graph::layout::
  tests::properties`).
- **Decided in:** `docs/design/03-graph-layout-algorithm.md` §3.1–§3.3, §3.7
  ("Invariants → tests").
- **Status:** works. One regression closed late: a reserved lane whose tip
  converged into another column used to draw a dangling `─` edge bent in
  from an empty column; fixed (test above now asserts `incoming == []`
  where it used to assert `[1]`).

### M1-03. History streams in topological order, falling back to date order when there's no commit-graph file — and never shows a parent above its own child

```meta
id: M1-03
area: history
milestone: m1-the-graph
status: works
```

- **How:** automatic on repo open.
- **Behaviour:** with a commit-graph file present, uses git's own generation
  numbers for an incremental topological sort that can start emitting rows
  before the whole visible range has been walked; without one, falls back
  to date order and the UI reports the order as provisional. When multiple
  commits share the exact same timestamp under date order (no
  commit-graph), the walk reads the whole tied-date group at once and emits
  children before parents — a plain tie-break alone couldn't do this
  because it doesn't see an unseen same-date child coming.
- **Proven by:**
  `graft_git::history::tests::the_topological_walk_emits_before_it_has_walked_the_range`
  `graft_git::history::tests::a_repository_with_no_commit_graph_file_falls_to_date_order_and_says_so`
  `graft_git::history::tests::a_repository_with_a_commit_graph_file_runs_the_topological_path`
  `graft_git::history::tests::tied_dates_never_put_a_parent_above_its_child_before_or_after_promotion`
  `graft::repos::tests::a_repository_is_opened_once_however_often_it_is_asked_for`
  `graft::repos::tests::jobs_on_different_threads_share_one_open_repository`
- **Decided in:** `docs/design/03-graph-layout-algorithm.md` §3.2 (walk
  strategy selection); ADR-0008.
- **Status:** works. Fixing the tied-date case cost nothing measurable: 1M
  commit, no-commit-graph first paint moved from ~103 ms to 104–125 ms
  warm, well inside budget. A related, harder case is accepted as-is by
  user decision rather than fixed: a repository where an unusually large
  number of commits share one exact second (e.g. a mass rebase) can make
  the walk read a whole multi-thousand-commit tie class before first paint
  — recorded in `KNOWN_GAPS.md`/ADR-0008's amendment, accepted, not capped.

### M1-04. Provisional order is promoted to topological order mid-session without disrupting what's on screen

```meta
id: M1-04
area: history
milestone: m1-the-graph
status: works
```

- **How:** automatic, once the commit-graph read finishes after the first
  paint (which used date order to get something on screen faster).
- **Behaviour:** a notice appears; the selected commit and the screen's
  scroll offset are preserved across the swap (by commit identity, not row
  number); a superseded background job's result is never applied on top of
  a newer one; pressing `Esc` while promotion is in flight keeps the
  provisional order and says so, rather than silently finishing the swap
  underneath the user. A follow-on regression (an in-flight far jump, e.g.
  from search, being silently dropped by the promotion swap) was found and
  fixed: both orderings — jump-then-promote and promote-then-jump — now
  re-apply the pending jump correctly.
- **Looks like:** `docs/shots/18-promotion-skewed-dates.txt`
- **Decided in:** `docs/design/03-graph-layout-algorithm.md` §3.6.
- **Status:** works.

### M1-05. Each row shows subject, author, relative date and ref badges — safely, even from hostile input

```meta
id: M1-05
area: graph
milestone: m1-the-graph
status: works
```

- **How:** automatic, on every visible row.
- **Behaviour:** columns degrade gracefully by terminal width (checked at
  80×24, 120×40 and 200×60). A subject or author name carrying a raw ANSI
  escape sequence renders inert, never reaching the terminal live.
  `.mailmap` merges two recorded identities into one author/colour. The
  selection-marker gutter is exactly the same width whether it's showing a
  marker or empty, so text never shifts sideways when the cursor moves.
  Branch, tag, `HEAD` and stash badges each have their own visual state
  (in sync / ahead-only / behind-only / diverged / detached / tag-only /
  stash / unborn); `ui.icons` (`plain`/`nerd`) changes how badges render;
  ahead/behind counting always runs on a background worker, never on the
  event-loop thread.
- **Proven by:**
  `graft_term::sanitize::tests::an_author_name_carrying_an_escape_sequence_comes_out_inert`
  `graft::view::tests::a_branch_name_with_an_escape_sequence_renders_inert`
  `graft_git::mailmap::tests::two_identities_resolve_to_one_canonical_author`
  `graft_ui::graph_screen::tests::the_marker_slot_is_the_same_width_empty_or_filled`
  `graft_ui::graph_screen::tests::ui_icons_changes_how_badges_render`
  `graft::jobs::tests::ahead_behind_is_counted_on_a_worker_never_the_loops_thread`
  `graft_git::repo::tests::ahead_behind_counts_the_ref_states_branches`
- **Looks like:** `docs/shots/10-row-columns.txt`, `docs/shots/11-ref-badges.txt`
- **Decided in:** `docs/design/01-view-mockups.md`.
- **Status:** works.

### M1-06. Relative dates round the way git's own `--date=relative` does, not by floor-dividing

```meta
id: M1-06
area: graph
milestone: m1-the-graph
status: works
```

- **How:** automatic, on the date column.
- **Behaviour:** matches git's own cascade at every unit boundary — e.g. 89
  and 90 minutes both read "89 minutes ago" (git rounds `(seconds+30)/60`,
  truncating; "90 minutes ago" never actually appears in real git output;
  the boundary flips to "2 hours ago" only at 5,400 s). This was a genuine
  bug, not a cosmetic one: Graft originally floor-divided each unit
  independently (46 hours showed "1d" where git shows "2 days ago"); fixed
  to match git's cascading rounding at every checked boundary, verified
  directly against real `git log --date=relative` output. The date read is
  also now immune to wall-clock drift in tests (a fixed clock, not
  `SystemTime::now()`, on the render path).
- **Proven by:**
  `graft_ui::graph_screen::columns::tests::matches_git_log_date_relative_at_every_boundary`
- **Looks like:** `docs/shots/48-relative-dates.txt`,
  `docs/shots/53-relative-dates-fixed-clock.txt`
- **Decided in:** design finding from a live user bug report, not a
  pre-written spec section.
- **Status:** works, verified against real git, not just internal
  self-consistency.

### M1-07. Lanes stay distinguishable without relying on colour — at full colour, 16 colours, and with colour off entirely

```meta
id: M1-07
area: graph
milestone: m1-the-graph
status: partial
unverified_aspect: lane-colour distinguishability by eye at truecolor/256, and by an actual colour-vision-deficient viewer
```

- **How:** automatic, adapting to the terminal's detected colour depth or
  `NO_COLOR`.
- **Behaviour:** at 16 colours and under `NO_COLOR`, odd-numbered lane
  columns draw with a dashed pattern (alternating `╎`/`╌`-style glyphs)
  instead of a solid line, so two same-hue neighbouring lanes are never
  drawn identically; corners stay undashed so joins still read cleanly.
  Truecolor and 256-colour terminals are unchanged (lanes are told apart by
  hue). At 16 colours, the highlighted/current row gets a left-edge accent
  rule (`▏`) instead of a raised background, because a background-shade
  step disappears at that depth.
- **Looks like:** `docs/shots/13-foxtrot-merge-16.txt`,
  `docs/shots/13-foxtrot-merge-no-color.txt`,
  `docs/shots/13-criss-cross-merge-16.txt`,
  `docs/shots/13-criss-cross-merge-no-color.txt`,
  `docs/shots/12c-palette16-chrome.txt`
- **Decided in:** ADR-0019.
- **Status:** partial. The dashing mechanism is corpus-tested (same-colour
  neighbours never share a glyph) and pty-replayed, but two related
  judgments stay explicitly open in `KNOWN_GAPS.md`, expiry M1 checkpoint:
  at truecolor/256 colours, neighbouring lanes can still share a hue (a
  1-in-12 hash collision, told apart only by column position); and the
  12-hue palette itself was chosen numerically (OKLab distance +
  colour-vision-deficiency simulation) but has never been judged as thin
  graph lines by an actual colour-vision-deficient viewer. Per this
  project's rules, a captured terminal stream can't prove colour either
  way — these stay open until someone looks.

### M1-08. Too many lanes for the width fold down to a budget, without losing the graph's shape

```meta
id: M1-08
area: graph
milestone: m1-the-graph
status: works
unverified_aspect: whether the folded rail reads as history or decoration at real widths — left open for the user to judge
```

- **How:** automatic whenever a window's open lanes exceed `graph.max_lanes`
  (unset default: 8 lanes below 100 columns, 10 from 100–159, 12 at 160 and
  above — the ceiling matches the 12-hue palette).
- **Behaviour:** folding is a priority budget, not a flat veto set (ADR-0025,
  replacing an earlier version that let a single wide selected thread pin
  every one of its columns and bust the cap): lane 0 first, then reserved
  lanes, then the selected commit's own ancestry column, then the rest by
  how recently a row touched them. Since ease-pass 06b a column draws only
  while a commit of its own is in view (lane 0 excepted while open), and
  since 06u also while it connects two commits in view (main merged into a
  feature branch, a criss-cross): such a connector ranks after every column
  holding a commit and, out of budget, is dropped rather than folded. A lane
  passing through with no commit on screen and no connection between two is
  dropped, never folded, and only columns holding a commit fold behind the
  rail — a folded column's commit draws on the rail. Folded lanes never draw as raw digit
  wallpaper (an earlier bug showed `+54` on every single row regardless of
  content) — they draw as short coloured stubs plus one `⋮` rail column,
  with the count in the header (`lanes drawn: N of M`, M counting every
  column open in view) only while some open column is not drawn. In the pixel tier, a folded edge fades to a short tail (about 19
  px, stopping before it would cross the next column's centre) rather than
  reaching for the rail, and the rail itself is a faint dotted line with no
  digits. Toggling a fold makes zero new layout calls; unfolding restores
  exactly the pre-fold columns of the affected rows and touches no other
  row. A user-configured cap larger than the terminal can actually hold is
  clamped so it never truncates the graph column.
- **Proven by:**
  `graft_graph::fold::tests::a_window_whose_every_open_column_holds_a_node_draws_as_laid_out`
  `graft_graph::fold::tests::the_least_recently_touched_node_columns_fold_first_and_the_marker_fits_the_limit`
  `graft_graph::fold::tests::a_column_open_in_no_row_or_with_no_node_is_dropped_without_counting`
  `graft_graph::fold::tests::in_view::under_the_cap_only_columns_holding_a_node_are_drawn`
  `graft_graph::fold::tests::in_view::over_the_cap_node_columns_fold_and_node_less_ones_stay_dropped`
  `graft_graph::fold::tests::edges_into_folded_lanes_leave_the_row_as_stubs_in_their_colour`
  `graft_graph::fold::tests::a_fold_that_changes_nothing_borrows_the_row_and_folds_no_edge`
  `graft_graph::fold::tests::the_budget_admits_selected_then_reserved_then_preferred_over_a_more_recently_touched_column`
  `graft_graph::fold::tests::lane_zero_draws_alone_when_the_budget_allows_only_the_rail`
  `graft_graph::fold::tests::a_thread_of_twelve_columns_still_folds_down_to_the_budget`
  `graft_graph::fold::tests::properties::node_columns_draw_or_fold_connectors_draw_or_drop_and_pass_throughs_never_draw`
  `graft_graph::fold::tests::in_view::a_column_joining_two_commits_in_view_is_drawn`
  `graft_graph::fold::tests::in_view::a_connector_past_the_budget_drops_and_never_folds`
  `graft_ui::graph_screen::tests::lanes_in_view::a_thread_of_twenty_columns_still_fits_the_configured_cap`
  `graft_ui::graph_column::tests::a_node_whose_column_folded_draws_on_the_rail_in_its_place`
- **Looks like:** `docs/shots/14-many-lanes.txt`,
  `docs/shots/14-many-lanes-expanded.txt`, `docs/shots/39-git-git-80x24.txt`,
  `docs/shots/40-raster-folded-on-dark.png`,
  `docs/shots/47-git-git-80x24-after-jj.txt`,
  `docs/shots/44-git-git-160x40.txt`
- **Decided in:** `docs/design/03-graph-layout-algorithm.md` §3.5
  (compaction); ADR-0022, ADR-0025.
- **Status:** works structurally and by test, but the visual judgment is
  explicitly left open in the source tickets: whether the folded rail
  "reads as history or decoration" at real widths was a stated question
  for the user to answer by eye on `git/git` at 120 columns, and the
  tickets record that check as pending. The old residual (a window whose
  node columns alone exceeded `max_lanes` drew wider than the limit) is
  gone since ease-pass 06b: node columns fold too, so the budget holds for
  any cap of 2 or more.

### M1-09. Folding a merge, viewing first-parent-only, and hiding stashes are independent toggles

```meta
id: M1-09
area: graph
milestone: m1-the-graph
status: works
```

- **How:** merged branches start folded behind their merge (ease-pass 06);
  `Space` opens/closes the merge under the cursor; `1` switches to
  a first-parent-only view; `H` hides/shows stash commits.
- **Behaviour:** first-parent view collapses a history with merges to one
  straight lane, following only the default branch's own parent chain.
  A folded merge hides its branch down to the fork point; hiding a stash
  takes its own commits with it but leaves its base commit alone. Each
  toggle only re-lays-out the rows it actually invalidates — switching back
  to a previous view rebuilds no chunks. These are pure inclusion-time
  predicates applied before layout, so the §1.2 graph invariants (M1-02)
  hold identically with any combination on.
- **Proven by:**
  `graft_graph::inclusion::tests::fold_by_default::opening_a_merge_invalidates_only_the_rows_from_the_merge_down`
  `graft_graph::inclusion::tests::a_hidden_stash_takes_its_own_commits_with_it_but_not_its_base`
  `graft_graph::inclusion::tests::first_parent_view_follows_the_default_branch_only`
- **Looks like:** `docs/shots/15-first-parent.txt`,
  `docs/shots/ease-pass/main-folded-120.txt`,
  `docs/shots/ease-pass/main-opened-120.txt`,
  `docs/shots/15-stash-hidden.txt`
- **Decided in:** `docs/design/02-keymap.md` §2.3 (keys added by this
  ticket, not pre-existing in the keymap doc).
- **Status:** works. A release binary was checked byte-identical between
  "unfold" and the original full view under a pty.

### M1-10. Selecting a commit highlights its ancestry; Focus mode dims everything else

```meta
id: M1-10
area: graph
milestone: m1-the-graph
status: works
```

- **How:** moving the cursor selects a row; `F` toggles Focus mode on the
  current selection.
- **Behaviour:** the selected commit's thread (its ancestors and
  descendants currently on screen) draws bold/full-strength; every other
  row keeps its shape but dims. Thread lanes are never folded by M1-08's
  budget, even under pressure. Moving the selection while Focus mode is on
  recomputes the fade from the new row; `Esc` leaves Focus mode before
  anything else `Esc` might do. Selection survives a chunk being evicted
  from memory and rebuilt later — it stays on the same commit id, not a row
  number.
- **Looks like:** `docs/shots/16-criss-cross-focus.txt`,
  `docs/shots/16-criss-cross-focus-esc.txt`,
  `docs/shots/16-criss-cross-highlight.txt`
- **Decided in:** `docs/design/05-visualization.md` §5.2 (Focus mode); the
  pixel-tier rule that dimming is alpha-only, never a heavier stroke,
  traces to ADR-0024.
- **Status:** works. Thread highlighting is scoped to rows currently on
  hand (not the whole history) and to bold/dim only in the pixel tier — a
  narrower implementation than the original ambition, recorded as a known
  limitation rather than a defect.

### M1-11. Mouse: click to select, wheel to scroll

```meta
id: M1-11
area: graph
milestone: m1-the-graph
status: works
```

- **How:** click a row to select it; the scroll wheel moves the visible
  window, not the cursor.
- **Behaviour:** click selects the row under the pointer; wheel scrolling
  never wraps back to the top mid-load — an early bug let a wheel scroll,
  arriving before the initial history walk finished, snap the view back to
  row 0 every time a new chunk answered (because the clamp always pulled
  the top row toward the cursor, and wheel scrolling never moves the
  cursor); fixed so the view only follows the cursor when the cursor itself
  had to move. On exit, mouse reporting is confirmed disabled in the raw
  byte stream (no leftover mouse-tracking mode left on in the user's real
  terminal).
- **Proven by:** `graft::history::tests::a_wheel_scroll_during_a_far_seek_withdraws_the_seek`
  (a related fix: a wheel scroll arriving during an in-flight search/far
  jump used to strand the "seeking" indicator forever, recoverable only by
  `Esc`; fixed to withdraw the pending jump the same way selecting a row
  does).
- **Looks like:** `docs/shots/17-mouse-select-and-tooltip.txt`,
  `docs/shots/32-wheel-before.txt`, `docs/shots/32-wheel-after.txt`
- **Decided in:** `PROMPT.md` §3 "Mouse".
- **Status:** works.

### M1-12. Hovering or peeking a truncated row shows its full subject in the footer

```meta
id: M1-12
area: graph
milestone: m1-the-graph
status: works
```

- **How:** hover the mouse over a row whose subject was cut with `…`; or
  press `Enter` on the cursor row (keyboard-only equivalent).
- **Behaviour:** the footer swaps its hint bar for the row's full,
  untruncated subject; moving off the row (or a second `Enter`) restores
  the hints. A row whose subject was never truncated has nothing to reveal
  and leaves the hints alone. `Esc` closes an active peek before anything
  else `Esc` might do; any other key clears a peek — it's one-shot, never
  sticky. A revealed subject containing a raw escape sequence renders
  inert, the same as anywhere else repo text reaches the screen. **This is
  the final, shipped M1 answer to "how do I see the whole commit
  message"** — an earlier design (a full popup tooltip with author, date,
  parents and a pin-with-`Enter` state) was built, then deliberately
  removed after a user review found it covered half the screen with no
  border and no visible way to dismiss it (ADR-0023, superseding ADR-0020).
  Author, date, parents and body detail are deferred to a later
  milestone's detail pane.
- **Proven by:**
  `graft_ui::graph_screen::tests::a_reveal_replaces_the_hints_with_the_full_subject`
  `graft_ui::graph_screen::tests::revealing_a_row_that_was_never_truncated_leaves_the_hints_alone`
  `graft_ui::graph_screen::tests::no_reveal_shows_the_ordinary_hints`
  `graft_ui::graph_screen::tests::escape_sequences_render_inert_in_the_reveal`
  `graft::view::tests::a_reveal_renders_an_escape_sequence_inert`
  `graft::view::tests::hover_names_the_reveal_row`
- **Looks like:** `docs/shots/41-hover-reveal.txt`, `docs/shots/41-peek.txt`,
  `docs/shots/41-no-reveal.txt`
- **Decided in:** ADR-0023 (supersedes ADR-0020, the original tooltip
  design).
- **Status:** works.

### M1-13. `y` copies the selected commit's full SHA to the system clipboard

```meta
id: M1-13
area: graph
milestone: m1-the-graph
status: partial
unverified_aspect: whether the OS clipboard actually received the copied SHA — OSC 52 doesn't report back
```

- **How:** select a commit, press `y`.
- **Behaviour:** sends an OSC 52 clipboard-set escape sequence
  (`ESC ] 52 ; c ; <base64 of the 40-character SHA> ESC \`), wrapped in
  tmux's passthrough sequence with every embedded `ESC` doubled when
  running inside tmux. The footer shows `<7 hex>… → clipboard (osc 52)`
  until the next key is pressed. A write failure (e.g. output already
  closed) propagates as a real error, never silently dropped.
- **Proven by:** `graft::event_loop::tests::a_failed_clipboard_write_ends_the_loop_with_its_error`
  and the `graft_term::clipboard` module's own tests
  (`abc_is_the_exact_sequence`, `base64_matches_the_rfc_vectors`,
  `a_full_sha_encodes_to_fifty_six_characters`,
  `inside_tmux_it_is_wrapped_alone_with_every_escape_doubled`,
  `directly_it_is_written_as_is`).
- **Looks like:** `docs/shots/45-y-copies-sha.txt`, `docs/shots/45-notice-cleared.txt`
- **Decided in:** ADR-0023 (added as the replacement for the tooltip's
  full-hash display, M1-12).
- **Status:** partial. The send side — the exact bytes Graft writes — is
  proven; whether the terminal's clipboard actually received them is not
  something OSC 52 reports back, so it's genuinely unknowable from inside
  Graft. Pasting in a real terminal to confirm is an explicit, still-open
  user check (`KNOWN_GAPS.md`).

### M1-14. Search jumps to a matching commit by message, author, committer, SHA or content, wrapping with `n`/`N`

```meta
id: M1-14
area: history
milestone: m1-the-graph
status: works
unverified_aspect: the 100k-search responsiveness claim — not independently re-measured, only ticket text and a replay
```

- **How:** `/` opens search; `n`/`N` jump to the next/previous hit and wrap
  around.
- **Behaviour:** a scan finds the last-matching commit by each searched
  field, including the commit body; a search with no hit finishes cleanly
  with nothing selected; a hit that's beyond history already streamed in
  triggers a walk out to it. A search started while history is still
  streaming shows a partial count that settles once the walk catches up. A
  result from a superseded (re-started) search is discarded, never applied
  late. A subject containing a raw escape sequence still matches correctly
  and still renders inert. On a 100k-commit fixture, time from pressing
  `Enter` to a far hit landing on screen was measured at 641 ms; toggling
  tiers (`F2`) 401 times during an active search had a median response of
  3.5 ms, max 9.2 ms, no keys lost.
- **Proven by:**
  `graft::search::tests::a_scan_finds_the_last_commit_by_each_field`
  `graft::search::tests::a_scan_finds_a_commit_by_its_body`
  `graft::search::tests::a_scan_with_no_hit_finishes_with_nothing`
  `graft::search::tests::a_cancelled_scan_stops_after_the_chunk_in_hand`
- **Looks like:** `docs/shots/26-search-far.txt`, `docs/shots/26-search-typing.txt`
- **Decided in:** `PROMPT.md` §4 "History" (search by message, author,
  committer, SHA, path, content or date range — M1 covers the row-level
  fields; path/content pickaxe search and date-range filtering aren't part
  of this milestone).
- **Status:** works for the tested fields; performance numbers and the
  "input stays responsive during a 100k search" claim come from the ticket
  text and a pty/pyte replay, not re-measured independently here. Known,
  named limitations: only the current hit is marked, a hidden (folded-away)
  hit says "(not in this view)" rather than jumping to it, and `n` before
  any hit exists is a no-op.

### M1-15. The pixel-tier graph renders as a real image, stays aligned, and scrolls within budget

```meta
id: M1-15
area: terminal-rendering
milestone: m1-the-graph
status: works
unverified_aspect: pixel-tier rendering at the application level in a real terminal — only the protocol/byte-stream is tested
```

- **How:** automatic in the Kitty-protocol pixel tier.
- **Behaviour:** an RGBA rasteriser draws lane lines, curves and commit
  nodes at a pinned per-cell size; different commit kinds (root, merge,
  HEAD, stash) differ in ring shape, not just colour, so the graph stays
  legible without colour. Pixels outside every drawn line or node are fully
  transparent. Lane 0 draws at the same stroke width as every other lane —
  an earlier bug made it look like a thick slab, because the "this is the
  selected commit's ancestry" thread emphasis was a 1.6× stroke-width
  multiplier, and the cursor starts on row 0 at launch, so lane 0 always
  looked artificially thick; fixed to one uniform stroke width everywhere,
  with the thread differing only in alpha. A strip cache holds rasterised
  rows keyed by content, cell size and theme, so scrolling one row
  rasterises at most one new strip, scrolling back is a cache hit, and a
  theme or cell-size change flushes the whole cache. The Kitty byte
  protocol handles chunked transmission, image ids, per-scroll-step crop
  and placement, and frees every uploaded image on exit; a dropped-image
  reply (`ENOENT`) triggers an automatic re-transmit rather than a
  permanently broken tile. Forcing the pixel tier on a terminal that never
  acknowledges the protocol still attempts it, with `F3` recovering to
  Unicode if it looks wrong. After a chain of performance fixes (rasterise
  by lane-run instead of per-row, wait for the cursor's own row before
  drawing any strip, idle-prefetch off-screen strips), a released build
  keeps scrolling a 100,000-commit history under the 16 ms input-to-frame
  budget: pixel-tier p99 well under 16 ms across repeated counted runs
  (final release table: max 5.9–11.6 ms across 7 runs). Debug builds are
  slower and are recorded, never asserted against the budget (ADR-0021 —
  frame budgets are a release-only promise).
- **Proven by:**
  `graft_graph::strip_cache::tests::the_selection_never_misses_a_backdrop`
  `graft::view::strips::tests::every_strip_waits_while_the_cursor_row_has_not_arrived`
  `graft::history::tests::a_request_lays_out_only_the_rows_it_names`
  `graft::history::tests::every_key_draws_from_rows_on_hand_while_the_worker_answers_a_key_late`
  `graft_graph::layout::stream::tests::a_viewport_before_every_kept_checkpoint_still_evicts_the_rows_before_it`
  `graft_graph::raster_pixels::tests::strips_keep_every_byte`
  `graft::event_loop::tests::no_strip_is_prefetched_while_input_is_queued`
  `graft::event_loop::tests::a_key_arriving_mid_prefetch_waits_for_at_most_one_strip`
  `graft::event_loop::tests::page_keys_find_their_strips_already_rasterised`
- **Looks like:** `docs/shots/22-raster-criss-cross-merge.png`,
  `docs/shots/22-raster-foxtrot-merge.png`, `docs/shots/22-raster-octopus-merge.png`,
  `docs/shots/22-raster-rings-on-dark.png`, `docs/shots/42-raster-rings-on-dark.png`,
  `docs/shots/42-raster-thread-on-dark.png`, `docs/shots/24d-kitty-check-report.txt`
- **Decided in:** `docs/design/03-graph-layout-algorithm.md` §3.6, §3.8
  (strip cache / 16 ms budget); ADR-0012, ADR-0024 (thread emphasis by
  alpha, not width).
- **Status:** works at the protocol/byte-stream level, tested very
  thoroughly (unit tests, byte-guards, release-mode counted timing runs).
  **Unverified by eye at the application level** — a real Ghostty run
  proved the Kitty encoder round-trips synthetic strips with zero protocol
  errors, but whether the app's *actual* strips look correct on a real
  screen is explicitly left to the user (`KNOWN_GAPS.md`; this project's
  own rule that captures can't prove colour or a rendered pixel result
  applies directly here). One accepted, unfixed residual: about 8% of
  strips can be rasterised again shortly after eviction from the cache
  (cache sized smaller than ideal) — recorded, not tuned.

### M1-16. iTerm2's own inline-image protocol renders the pixel tier where Kitty's isn't available

```meta
id: M1-16
area: terminal-rendering
milestone: m1-the-graph
status: unverified
```

- **How:** automatic in iTerm2 (detected via the environment-hint exception,
  §M1-01) when the Kitty protocol isn't reachable.
- **Behaviour:** an unchanged band of rows is never re-encoded or
  re-transmitted; resizing the terminal doesn't retransmit the whole
  viewport image (a release measurement recorded 0 transmits and 0 cache
  flushes across a resize once the resize-handling fix below landed).
- **Looks like:** `docs/shots/25-iterm2-capture-band.png`
- **Decided in:** ADR-0012 addendum (resize handling).
- **Status:** unverified by eye. The byte stream is unit-tested and
  pty-captured, but `KNOWN_GAPS.md` carries an explicit, still-open row
  saying the iTerm2 tier has not been seen rendered in a real iTerm2
  window by the user for this milestone's work specifically (an
  M0-era note elsewhere records an earlier, separate iTerm2 session that
  is not the same verification).

### M1-17. Resizing the terminal, or regaining focus, keeps the graph correct without a full redraw storm

```meta
id: M1-17
area: terminal-rendering
milestone: m1-the-graph
status: works
```

- **How:** resizing the window; switching focus away and back.
- **Behaviour:** a resize re-queries the terminal's cell size; if the
  terminal stays silent, the previous size is kept and marked "assumed"
  rather than trusted outright. Regaining focus re-queries the background
  colour for the theme hint; a silent terminal leaves the current theme
  unchanged rather than resetting it. Launching at a degenerate size (0×0
  or 1×1 cells) never panics. A resize retransmits rows via a row-by-row
  clear rather than treating every resize as a full-screen clear (an
  earlier version's "clear everything on resize" behaviour turned out to
  be the terminal library's own default behaviour, not something Graft
  asked for; fixed to a targeted clear, and the fix closed three related
  image-tier gaps at once).
- **Looks like:** `docs/shots/21-redetect-resize-and-focus.txt`,
  `docs/shots/21-tiny-sizes-launch-0x0.txt`, `docs/shots/21-tiny-sizes-launch-1x1.txt`
- **Decided in:** `docs/design/04-terminal-capability-detection.md` §4.5
  (re-detection triggers); ADR-0012 addendum.
- **Status:** works. One timing edge case remains open, not closed:
  `KNOWN_GAPS.md` notes a terminal reply split across more than a 50 ms
  gap between reads can be misread as ordinary keystrokes rather than a
  probe answer — a real but rare race, expiry M1 heavy tier.

### M1-18. Typed keystrokes are never lost or mistaken for a terminal's protocol reply

```meta
id: M1-18
area: terminal-rendering
milestone: m1-the-graph
status: works
```

- **How:** automatic — any key pressed while Graft is mid-probe (startup,
  resize, focus regain) or during normal operation.
- **Behaviour:** Graft owns its own input decoder (keys, SGR/X10 mouse
  reports, bracketed paste, focus events) rather than relying on a generic
  terminal-event library, so it can tell a terminal's own protocol reply
  apart from a real keystroke that happens to start with the same bytes. A
  batch of keys typed during a capability-probe window all arrive
  afterward, in the order they were typed. A reply split across two reads
  from the terminal is still routed correctly to whatever is waiting for
  it. A lone `Esc` (which could be the start of a reply) is delivered as a
  real keystroke once the reply timeout (50 ms) passes without more bytes
  arriving. Two real concurrency bugs found and fixed during this
  milestone: `event_loop::run` used to spawn a fresh stdin-reading thread
  every time it ran (harmless for a single session, but it made every
  gallery-example step spawn another one, so after a few steps several
  threads raced to read the same byte and dropped most keystrokes) — fixed
  to one stdin reader per process; and input forwarded past a just-finished
  inner run loop could be lost entirely if nothing was listening for it
  yet — fixed by holding it, joining the forwarding thread, then handing
  any unhandled input back to the caller.
- **Looks like:** `docs/shots/20-typeahead-f2-during-probe.txt`
- **Decided in:** ADR-0013 (own input decoder, not `crossterm::event::read`).
- **Status:** works. One decoder gap named and accepted, not planned unless
  it's actually seen: `urxvt`'s own `1015` mouse-reporting variant isn't
  decoded (xterm, SS3, Kitty's `CSI u`, and SGR/X10 mouse all are).

### M1-19. Scrolling stays smooth (under a 16 ms input-to-frame budget) at real scale, in every tier

```meta
id: M1-19
area: graph
milestone: m1-the-graph
status: works
```

- **How:** automatic — `j`/`k`/`PageUp`/`PageDown`/wheel scrolling.
- **Behaviour:** this is the same 16 ms-per-frame promise as M1-15, stated
  for both tiers together and for the interaction rule underneath it: **no
  key ever blocks waiting on a background answer.** Keys always draw from
  whatever rows are already on hand; the history view keeps 3 screens'
  worth of rows loaded either side of what's visible and tracks its own
  outstanding requests so it never re-asks for the same rows twice. This
  was a deliberate spec-compliance fix, not a speed tweak, made after the
  user reviewed the graph gallery and found the app could still visibly
  wait on a slow answer. On a released binary, 100k-commit and
  50,000-ref fixtures both stay under budget across repeated counted runs
  in both tiers (final figures: Unicode 100k max 5.4–9.9 ms, Unicode
  50k-refs max 11.2–14.6 ms). One accepted, reported residual:
  idle-triggered background work (prefetching, redraw after an idle pause)
  can occasionally exceed 16 ms at 50k refs — 14 of 140 sampled keys did,
  across 7 runs, worst case 19.1 ms; cause named (CPU power-state recovery
  after idle) but not removed.
- **Proven by:**
  `graft::history::tests::every_key_draws_from_rows_on_hand_while_the_worker_answers_a_key_late`
  (shared with M1-15; this is the same window/prefetch machinery, proven
  once and applied to both tiers)
- **Decided in:** ADR-0021 (frame budgets fail only in release builds; a
  release perf run is part of every milestone review, not just this one).
- **Status:** works, release-only by design; debug is recorded, not
  gated. The idle-wake residual above is reported in `KNOWN_GAPS.md`, not
  fixed.

### M1-20. Reading a repository with tens of thousands of refs stays within budget

```meta
id: M1-20
area: graph
milestone: m1-the-graph
status: works
```

- **How:** automatic — opening or refreshing a repository with a very large
  number of branches/tags.
- **Behaviour:** on a 50,000-ref fixture, reading tips takes 165 ms in a
  release build (was 2.94 s before three fixes: memoized commit-tip lookups
  by direct target plus an object-header check instead of a full peel; a
  hash-set instead of a linear scan for badge lookup; short-circuiting
  ahead/behind counting when two branch tips are already equal, without
  opening the commit graph at all). First paint with 50,000 refs is 57 ms
  in release, under the 150 ms budget.
- **Proven by:**
  `graft::many_refs::tests::reading_tips_from_50k_real_refs_is_within_budget`
  `graft_git::repo::tests::refs_sharing_commits_or_naming_no_commit_peel_the_same_loose_or_packed`
- **Decided in:** part of the same ADR-0021 release-only budget policy as
  M1-19.
- **Status:** works in release. Debug-build first paint at 50k refs stays
  over budget (494 ms); explicitly won't-fix — release is the gate,
  debug numbers are recorded only, and re-measuring debug further was
  judged to buy nothing (`KNOWN_GAPS.md`).

### M1-21. Every event-loop tick stays cheap even with a large ref set on screen

```meta
id: M1-21
area: graph
milestone: m1-the-graph
status: works
```

- **How:** invisible — a fix underneath M1-19/M1-20's frame-budget numbers.
- **Behaviour:** every UI event used to deep-clone the entire application
  state, including a full copy of every ref badge (measurable as 1.0–1.9 ms
  of clone-plus-drop cost per event at 50k refs, and a key press usually
  triggers at least two state updates, four in the pixel tier). Fixed by
  wrapping the ref-badge set in a reference-counted pointer so the clone is
  O(1) instead of O(refs). This alone cut release-mode Unicode 50k-refs
  frame times from a 13–15 ms max to a 5–6 ms max.
- **Proven by:**
  `graft::update::tests::the_next_state_shares_the_ref_badges_instead_of_copying_them`
- **Decided in:** finding from the M1 review's first round, not a
  pre-written ticket.
- **Status:** works. One small, honestly reported oddity: the pixel tier's
  *max* frame time at 50k refs rose slightly after this fix (its p99 fell)
  — noted as unexplained but still comfortably under budget, not chased
  further.

### M1-22. The graph never freezes on a blank frame while scrolling in the pixel tier

```meta
id: M1-22
area: terminal-rendering
milestone: m1-the-graph
status: works
```

- **How:** automatic — wheel/trackpad scrolling in the pixel tier.
- **Behaviour:** an earlier bug left the pixel graph blank indefinitely
  after a wheel scroll moved the view away from the cursor's row and then
  stopped — only pressing `j`/`k` (which moves the cursor) recovered it,
  because the strip-drawing logic assumed the cursor was always on
  screen. Fixed so a strip only waits for the cursor's row when the cursor
  is actually visible; a scroll that leaves the cursor off-screen draws
  normally.
- **Looks like:** `docs/shots/37-wheel-scroll-placements.txt`,
  `docs/shots/user-37-git-git-pixel-after-scroll.png` (the original bug
  report capture)
- **Decided in:** bug found during the user's gallery walkthrough, not a
  pre-written ticket.
- **Status:** works.

### M1-23. A fixed gallery walks 23 real and synthetic repository shapes end to end, with a capture of each

```meta
id: M1-23
area: graph
milestone: m1-the-graph
status: works
unverified_aspect: whether each gallery stop reads correctly by eye — explicitly left to the user, not yet done
```

- **How:** `cargo run -p mergewright --example gallery` (`]`/`[` to step,
  `q` to quit, `--list` to print every stop, `--matrix` for a coverage
  table, `--start <name>` to jump to one stop). Not compiled into the
  shipped binary.
- **Behaviour:** every graph key works inside it identically to the real
  app (`j`/`k`, fold, first-parent, hide-stash, Focus, unfold, `y`, `F3`).
  A three-line banner sits above the rows from the very first frame: the
  stop's number/name/group/keys/tier, what right looks like at this stop,
  and what sets it apart from its neighbours — added after the first
  version shipped with only a footer, which a first-time viewer found
  gave no sense of where they were. Every banner line fits 80 columns. A
  real concurrency bug was found and fixed by this harness itself: because
  each gallery step restarted the event loop, and the event loop used to
  spawn a fresh stdin-reading thread every restart, by the third or fourth
  step several threads were racing to read the same keystroke and losing
  most of them — fixed by making the stdin reader a true singleton (see
  M1-18).
- **Looks like:** one `docs/shots/35-<stop>-unicode.txt` and
  `docs/shots/35-<stop>-ascii.txt` pair per stop (23 stops × 2 tiers, e.g.
  `docs/shots/35-trivial-unicode.txt`), `docs/shots/35-coverage-matrix.txt`,
  `docs/shots/38-gallery-first-frame-and-two-steps.txt`, `docs/shots/38-list.txt`
- **Decided in:** `PROMPT.md`'s M1 promise, "the shape of history is
  obvious at a glance" — this is the mechanism the checklist in
  `TESTING.md` uses to check it, one repository shape at a time.
- **Status:** the mechanism works and is heavily captured; the actual
  judgment — does each stop read correctly — is **explicitly left to the
  user**, and the source tickets record that walk-through as not yet
  done. Known, accepted coverage gaps: no light-theme snapshot exists for
  any stop; only 3 of the 23 shapes have a pixel-tier capture; one shape
  (a non-UTF-8 ref name) has no real graph at all because the fixture
  can't be built on this machine's filesystem.

### M1-24. The graph offers to build a missing commit-graph file, and using it visibly shrinks memory and speeds things up

```meta
id: M1-24
area: history
milestone: m1-the-graph
status: works
```

- **How:** the footer shows an offer when a repository has no
  `.git/objects/info/commit-graph` file; `B` builds it, `X` declines (and
  the decline is remembered for that repository).
- **Behaviour:** `git commit-graph write --reachable` runs as a background
  job; on success the offer becomes "written"; on failure, the error and
  git's exit code are shown, never swallowed. Verified end to end: after
  writing, `git commit-graph verify` exits 0, and the same repository
  re-measured afterward moves from the no-commit-graph memory/speed
  numbers to the with-commit-graph ones (see M1-27).
- **Proven by:** `graft_git::commit_graph::offer_state` and its state
  transitions (`Absent`/`Offered`/`Writing`/`Written`/`Declined`/`Failed`)
  are covered by the module's own test suite; `graft_fixtures::copy_into_temp`
  supports the test scaffolding that exercises the write path on a real
  temp repository.
- **Looks like:** `docs/shots/54-commit-graph-offer-80.txt`,
  `docs/shots/54-commit-graph-offer-120.txt`
- **Decided in:** ADR-0008 (amended to add the offer, replacing an earlier
  version where a missing commit-graph was silently accepted).
- **Status:** works, verified on real repositories up to 100k commits.
  Writing one on a real 1M-commit fixture is also measured (5.1–5.3 s
  wall) but that specific path was exercised as part of the memory-curve
  measurement (M1-27), not this ticket's own acceptance run.

### M1-25. History scales to a million commits without loading everyone into memory

```meta
id: M1-25
area: history
milestone: m1-the-graph
status: works
```

- **How:** automatic — the same repo-open and scroll behaviour as any
  smaller repository, but measured at real scale.
- **Behaviour:** every commit is addressed by a small handle, not a full
  20-byte object id. When a commit-graph file exists, a commit's handle is
  literally its position in that file; commits outside it (or every commit,
  if there's no commit-graph at all) go through an overflow interner. At 1M
  commits with no commit-graph, the interner + order structures cost about
  9 bytes/commit (after a fix that removed vector-doubling slack that had
  briefly pushed this to 10.37 B/commit). The walk itself reads the
  commit-graph file directly rather than the full object database when one
  exists — at 1M commits this cut object-database reads to zero and peak
  RSS from 933 MiB to 646–650 MiB, before the later handle-scale fixes
  below took it further (see M1-27).
- **Proven by:**
  `graft::history::tests::page_down_past_the_rows_on_hand_while_promotion_swaps_keeps_its_place`
  `graft_core::commit_handle::tests::parents_collect_into_the_inline_shape_for_their_count`
  `graft_core::commit_handle::tests::parents_read_back_in_order_whatever_the_shape`
  `graft_core::commit_handle::tests::an_empty_set_allocates_nothing`
  `graft_core::commit_handle::tests::a_set_costs_a_bit_per_handle_up_to_the_largest`
  `graft_core::commit_handle::tests::two_sets_with_the_same_members_are_equal_however_they_got_there`
  plus property test `a_handle_set_behaves_like_a_hash_set`.
- **Decided in:** ADR-0026 (everything that's O(history) uses a dense,
  handle-indexed representation, 8–16 bytes/commit; the walk reads the
  commit-graph, not the object database; rows hydrate near the viewport,
  not eagerly). This ADR was itself a mid-milestone design correction:
  the original M1 plan measured 881–949 MiB peak RSS at 1M commits against
  a 300 MB budget — a 3× miss — and the fix was scoped as a design pass,
  not a tuning pass.
- **Status:** works, gated in the heavy test tier (`heavy-1m` feature).

### M1-26. Rows on screen hydrate only as they're needed; scrolling a huge history doesn't pull everything in

```meta
id: M1-26
area: history
milestone: m1-the-graph
status: works
```

- **How:** automatic — scrolling or paging through any repository, huge
  ones included.
- **Behaviour:** the layout engine can lay out a slice of history as pure
  handle/lane bookkeeping — merge-lane identities included — without
  reading the actual commit objects for rows that aren't visible yet. A
  merge whose lane spans a chunk boundary is read (to recover its branch
  name for lane identity) exactly once, counted separately from ordinary
  row reads. Chunks the layout has already passed through, off-screen, add
  zero further reads when scrolled past again. On a cold jump straight to
  the top of a 100,000-commit fixture, the total rows actually hydrated
  stays bounded at roughly 2× the chunk size plus a small buffer — not the
  whole history.
- **Proven by:**
  `graft_graph::layout::stream::tests::a_merge_lane_open_across_a_chunk_boundary_is_named_with_one_read`
- **Decided in:** `docs/design/03-graph-layout-algorithm.md` §3.6; a
  design amendment mid-ticket chose deferred merge-lane identity
  (resolved only when a row is actually assembled) over reading every
  merge eagerly, because eager reads would have touched roughly a quarter
  of `git/git`'s whole history just to lay out lanes.
- **Status:** works. On the real `git/git` repository (85,649 commits),
  landing this dropped peak RSS with a commit-graph from 223 MiB to 46 MiB
  and `G` (jump to a commit) from 0.53 s to 0.14 s; without a commit-graph,
  220 MiB to 98 MiB and 0.85 s to 0.80 s.

### M1-27. Memory stays inside a gated budget at a million commits — and the budget is honestly split between what's proven and what's only measured

```meta
id: M1-27
area: history
milestone: m1-the-graph
status: partial
unverified_aspect: first paint at 1M commits right after clearing the OS page cache — only ever measured warm
```

- **How:** invisible — the same repo-open/scroll path as any size, this row
  states the actual numbers at the largest tested scale.
- **Behaviour, with a commit-graph file present:** peak RSS 174–184 MiB at
  1M commits, gated and passing against the 300 MB budget. **Without one:**
  peak RSS 445–509 MiB — this is **explicitly not gated**, by a user
  decision recorded in ADR-0026: instead, the live heap allocation slope is
  gated at ≤100 bytes/commit (measured 76, passing) and the memory
  footprint (~140 bytes/commit, 155–158 MiB at 1M) is reported with a named
  floor but not enforced as pass/fail. The gap between reported RSS and
  footprint at that scale (roughly 290–350 MiB) is the repository's own
  pack file on disk, not a Graft data structure. Getting from the original
  881–949 MiB measurement to here took a chain of specific, named fixes:
  reading the commit-graph instead of the object database (M1-25); rows
  hydrating on demand (M1-26); one read per distinct target instead of a
  full peel for tag/branch tips (below); replacing an LRU checkpoint cache
  with an even grid so far jumps stay fast without holding everything;
  parent lists stored as a compact array instead of per-commit allocations;
  and paging the string interner instead of one growing table.
- **Proven by:** `graft::heavy::tests::slopes_per_commit_with_a_commit_graph`
  `graft::heavy::tests::budgets_hold_on_the_100k_fixture_with_a_commit_graph`
  `graft::heavy::tests::budgets_hold_on_the_100k_fixture_without_a_commit_graph`
  `graft::heavy::tests::budgets_hold_on_the_100k_realistic_fixture_with_a_commit_graph`
  `graft::heavy::tests::budgets_hold_on_the_100k_realistic_fixture_without_a_commit_graph`
  `graft::heavy::tests::budgets_hold_on_the_100k_fixture_with_50k_refs`
  `graft::heavy::tests::child_gated_run` (guards measure inside a real
  child process, not the parent's lifetime-max RSS, so a run's peak can't
  leak forward and falsely pass a later, smaller fixture)
- **Looks like:** `docs/shots/m1-memory-curve.txt`,
  `docs/shots/m1-memory-curve-offer.txt`
- **Decided in:** ADR-0026 and its two amendments (the split heap/footprint
  bar; the tag-tip residual below).
- **Status:** partial by design, and honestly so. Green where gated (peak
  RSS with a commit-graph, live heap slope without one); explicitly
  reported-not-gated where the number is still real but the team chose not
  to fail the build on it (RSS/footprint without a commit-graph). One
  named, unresolved residual specific to tag/branch-tip badges: annotated
  tags aren't stored in the commit-graph table by design, so each one still
  costs one extra object read; after every fix applied, the commit-graph
  memory slope guard's own bar (originally a single ≤150 bytes/commit
  figure) is still red under that original number — the team's actual
  resolution was to split the guard into a ≤50 B/commit heap bar (passing,
  measured 29–31) and a ≤200 MiB RSS bar (passing, measured 167–169),
  rather than close the original number, and that split is the currently
  governing bar (ADR-0026 amendment). Also explicitly reported, not proven:
  first paint at 1M commits straight after clearing the OS page cache
  (`sudo purge`) — only ever measured warm, by user-accepted convention
  (ADR-0027); cold numbers (~0.7–1.0 s) are recorded but not re-verified
  cold on this machine.

### M1-28. First paint at a million commits stays fast, and far jumps replay a bounded slice, not the whole history

```meta
id: M1-28
area: history
milestone: m1-the-graph
status: works
```

- **How:** opening a huge repository; jumping far in it (search, or a
  direct commit lookup).
- **Behaviour:** warm first paint at 1M commits is 88–125 ms, well under
  the ~500 ms budget (cold runs, only ever measured with the OS page cache
  warm by convention, are 710–1,032 ms and reported, not gated — see
  M1-27). The single largest remaining cost in first paint was found and
  named exactly: reading every ref three times before the first window
  painted (once each for branch resolution, the history stream, and
  tip-badge lookups) — fixed to read the ref set once and share it, taking
  1M warm first paint from 291–347 ms down to 88–125 ms. Far jumps (e.g.
  to row 500,000 or 900,000 after a cold repo open) replay at most a few
  thousand rows from the nearest checkpoint rather than the whole distance
  jumped, because checkpoints now sit on an even grid (stride scaling with
  history size, e.g. 16,000 rows apart at 1M) instead of being kept or
  evicted by recency — before this fix, the same jump replayed the entire
  500,500-row distance in 1.4 seconds; after, 4,500 rows in 13–16 ms.
- **Decided in:** part of ADR-0026's overall scaling design.
- **Status:** works, gated by a dedicated 1M-scale scroll-timing test kept
  behind the `heavy-1m` feature (a manual/review-only run, per this
  project's laptop-safe test-tier rule) rather than the standing test
  commands, at p99 ≈6–8 ms per frame at 1M commits, under the same 16 ms
  budget as smaller scales.

### M1-29. Piping Graft's output, or opening a repository that fails, never hangs a script

```meta
id: M1-29
area: foundations
milestone: m1-the-graph
status: works
```

- **How:** running Graft with stdout redirected to a file or another
  program (`graft <repo> | cat`, CI, scripts).
- **Behaviour:** a piped run waits for history to actually finish loading
  before it's considered "settled" and prints its one frame — an earlier
  bug treated "no history loaded yet" as already settled, so a script
  could see an almost-empty frame and the process exit 0 with no real
  output and no error. A repository that fails to open still exits
  promptly with its error, rather than hanging waiting for history that
  will never arrive. Piped output never contains a raw escape byte.
- **Proven by:**
  `graft::event_loop::tests::headless_is_not_settled_before_history_exists`
  `graft::event_loop::tests::headless_is_not_settled_while_history_is_streaming`
  `graft::event_loop::tests::headless_is_settled_once_history_completes`
  `graft::event_loop::tests::headless_is_settled_when_the_walk_fails`
  `graft::tests::it::piped_stdout::piped_stdout_never_contains_an_escape_byte`
  `graft::tests::it::piped_stdout::a_repository_that_fails_to_open_exits_with_its_error_never_hangs`
- **Decided in:** bug found during M1's memory-scale work, not a
  pre-written ticket.
- **Status:** works.

### M1-30. The manual terminal checklist and the checkpoint script give the user a single, repeatable way to judge the milestone

```meta
id: M1-30
area: graph
milestone: m1-the-graph
status: works
unverified_aspect: the per-terminal pass/reads-well verdicts — blank by design, the user's own checkpoint step
```

- **How:** `scripts/m1-checkpoint.sh <scenario>` (`git-git`, `criss-cross`,
  `foxtrot`, `octopus`, or `list`); the terminal matrix in `TESTING.md`.
- **Behaviour:** each scenario builds or locates its fixture, prints and
  runs the exact command, and states in plain language what to look for
  (the default branch as a straight line, where a merge should visibly
  join, whether lanes stay distinguishable). The terminal matrix records
  two separate verdicts per terminal — *pass* (a correct graph or a clean
  fallback, judged against fixed conditions) and *reads well* (whether the
  shape of history is obvious at a glance — the milestone's actual promise,
  left blank for the user alone to fill in, never a builder or reviewer).
- **Decided in:** `TESTING.md` "The manual terminal checklist" (skeleton
  defined at M0, filled in progressively by M1's tickets 08, 24, 25, 29 and
  completed by 28).
- **Status:** the script and matrix exist and were exercised once by a
  builder under a synthetic pty (not a real terminal), confirming the
  `criss-cross` scenario end to end and the `git-git` no-network failure
  path. The actual per-terminal *pass* and *reads well* verdicts are, by
  design, still blank — filling them in is the user's own checkpoint step,
  not something this milestone's build work could complete on its own.

## Deliberately not built

- **Map view, the Overview strip, and the detail pane's "Contained in"
  field.** All three are designed in `docs/design/05-visualization.md` /
  `docs/design/01-view-mockups.md` but explicitly deferred to a later
  milestone at M1's ticket-splitting stage — M1 builds the per-commit graph
  and Focus mode (M1-10) only.
- **Tooltip with commit body, author, date and pinning.** Built (ticket 34,
  ADR-0020), then deliberately removed after a user review (ADR-0023) —
  see M1-12. Not a regression: a considered replacement.
- **Sixel encoder.** A terminal can win the Sixel tier in detection with no
  code able to actually draw to it; such terminals draw ASCII instead. Not
  planned until a Sixel encoder exists (carried over from M0, still open).
- **Sidebar-based ref filtering and ranking at very large ref counts.** M1
  proves that *reading* 50,000 refs stays fast (M1-20); there is no sidebar
  to filter or rank them in yet — that's the milestone that builds a
  sidebar.
- **Homoglyph/width-aware text safety, LFS, sparse-checkout, and every
  other M0-level "not planned for v1" item.** Unchanged by M1; see
  `docs/capabilities/m0-foundations.md`.
- **A guard that would catch a `git/git`-sized memory regression in
  automated tests.** The heavy test tier's own synthetic fixtures (even a
  "realistic" one meant to approximate `git/git`'s shape) only move memory
  by 23–27 MiB between variants, while `git/git` itself sits with roughly
  75 MiB less headroom than the synthetic numbers suggest — a regression
  that size could pass every automated test today. `KNOWN_GAPS.md`,
  expiry M7.
- **Background layout advance between keystrokes.** Layout only advances as
  far as the current view asks; there's no idle work that gets ahead of
  the viewport before it's needed. `KNOWN_GAPS.md`, expiry M7 (deferred
  four times before being given a firm milestone).
