# M6 stop — whole-app usability walk

First walk under CLAUDE.md "Every milestone ends with a whole-app usability walk". Reviewer, read-only against
`10d06533`. Scored against `docs/research/ux-principles.md` (cited as **UX §n**, heuristics as **H1–H10**).

**User context used:** Zed / VS Code integrated terminal, dark theme, 187 columns, Unicode tier (field notes :49,
:57; memory). Daily tasks: history, diffs, stage/commit, fetch/pull/push, branch/checkout, conflicts, stash,
cherry-pick; team on dependent branches (memory `graft-daily-workflows`, `works-in-a-team-on-dependent-branches`).

**How this was checked, and what was not.**
- Behaviour: read from source, with file:line on every behavioural finding (rule 7). **The app was not run.** No gate,
  walk or test was executed for this review (scope: usability walk; the M6 gates are the milestone review's).
- Text: from captures under `docs/shots/`. Many pre-date M6 (e.g. `m5-02-preview-merge-160x48.txt` has no `y confirm`
  footer, which M6 added). No capture is at 187 columns outside `docs/shots/m6/`. Where a capture and the source
  disagree, the source wins and the finding says so.
- **Colour: not judged (rule 5).** The one colour result the user has confirmed by eye is the cursor-row fill (field
  notes :50). Every other colour on every screen is unverified; nothing below asserts one.
- Step counts: M6's are walk-enforced (`crates/graft/tests/it/m5_walk/conflict_journeys.rs::journey_counts_match_the_note`,
  **not run here**). M3–M5 design notes have **no Journeys section**, so the counts in the table below are mine, by
  UX §1.3, and have no best-tool figure to sit beside (finding M6).

## Known — not refiled

- Conflict screen colour and connecting lines; finish/`c` discovery; the summary's `⏎` vs `y` — being redesigned,
  `docs/design-notes/m6-conflicts.md` "Amendment 2026-09-24" (P11–P13).
- `?` help not built (KNOWN_GAPS :163, M8). Nothing teaches the app / drag not discoverable (:563).
- `Ctrl+Enter` to commit unverified outside the kitty protocol (:206). Graft pushes `DISAMBIGUATE_ESCAPE_CODES`
  (`crates/graft/src/terminal.rs:303-306`); whether Zed or VS Code honour it is still unknown.
- `a amend` never on any bar (:544). Remote keys only on bars ≥156 columns (:456; not his width).
- `p`/`P` swallow their refusal while a write runs (:582 M6/06). No way-back hint from the changes pane's file list
  (:635 M6/12). Overlay + Detail tab draws dead keys (:559 M5/42).
- Journal revert preview does not take `y` (user ruling, keymap §2.3b) — a deliberate exception, not a finding.
- Chain rebase for dependent branches (ADR-0032) — M7, not built.

## Daily tasks, walked

Counts per UX §1.3 (keys; a typed string = 1; a repeated motion = 1). "Zed" = `⌃⏎` assumed not to arrive (known row
:206). Principles: ✓ holds, ✗ fails (finding id), – not applicable.

| Task | Start → done | Keys | H1 status | H4 consistent | H6 next key shown | H9 recovery | UX 3.1 landing |
|---|---|---|---|---|---|---|---|
| Read a commit's diff | graph → pane on it | `j` `⏎` **2** | ✓ | ✓ | ✓ | – | ✓ |
| Stage one file, commit (Zed) | graph on uncommitted row → committed | `⏎ j space c <msg> ⇥ ⇥ ⇥ ⏎` **9** | ✓ | ✗ M2 | ✓ | ✓ | ✗ m5 |
| Fetch | graph → fetched | `f` **1** | ✓ | ✓ | ✓ | ✓ | ✓ |
| Pull, behind only | graph → pulled | `p` **1** | ✓ | ✓ | ✓ | ✓ | ✓ |
| Pull, diverged (team, daily) | graph → rebased on upstream | `p` `r` `r` **3** | ✓ | ✓ | ✓ | ✓ | ✗ m5 |
| Push | graph → pushed | `P → ⏎` **3** | ✓ | ✗ M1 | ✓ | ✓ | ✓ |
| Push rejected → recover | rejection line → pushed | `p r r P → ⏎` **6** | ✓ | – | ✗ m6 | ✗ m6 | ✗ M4 (force box) |
| Stash, then pull | changes pane → stashed | `⇥ S s <msg> ⏎` **5** | ✓ | ✗ m2 | ✗ m2 | ✗ m7 | ✓ |
| Pop a stash | graph → popped | `S j p` **3** | ✓ | ✗ m4 | ✓ | ✓ | ✓ |
| New branch, switch to it | graph on a commit → on it | `b <name> ⇥ ⏎` **4** | ✓ | ✗ m3 | ✓ | ✓ | ✓ |
| Check out an existing branch | graph on its row → on it | `. j ⏎ y` **4** (+ finding the row) | ✓ | ✗ m3 | ✗ M5 | ✓ | ✓ |
| Cherry-pick a commit | graph on it → picked | `. j×6 ⏎ y` **4** (9 presses) | ✓ | ✓ | ✗ M5 | ✓ | ✗ m5 |
| Merge a branch in | graph on it → merged | `m m` **2** | ✓ | ✓ | ✓ | ✓ | ✗ m5 |
| Resolve + finish a merge | per M6 note | 4–6 (walk-enforced) | known | known | known | ✓ | known |
| Commit during a live merge | changes pane → merged | `c Esc ⇥ ⇥ c ⏎` **5** of which 4 carry no work | ✓ | ✗ M3 | ✗ M3 | ✗ M3 | ✗ M3 |
| Undo the last thing | anywhere → undone | `u` **1** | ✓ | ✓ | ✓ | ✓ | ✓ |

## Critical

None. No path found that loses work or writes without a confirm the design requires.

## Major

**M1 — Confirm boxes do not take `y`, and `y` silently cancels them.**
- Task: push (daily); also discard, stash drop, abort, skip, start over, undo-after-push.
- Screen: every `ConfirmView` box.
- Principle: H4 (same key, same meaning), H1 (a cancel with no word), UX §3.2 (exit differs per mode).
- Evidence: `crates/graft/src/update/confirm.rs:103-111` acts only on `Enter`; `:173` `_ => next.confirm = None`
  closes on every other key, `y` included, with no status. Previews confirm on `y`:
  `crates/graft/src/update/preview.rs:423-425`. Field notes :58 record the rule he was given: *"`y` confirms every
  operation preview"*. The push box is the one he meets daily; after `P y` the box is gone, nothing was pushed, and
  only the badge's `↑n` says so.
- Direction: `y` acts in every box with one acting choice (the undo-after-push box keeps its cursor); at minimum an
  unknown key is swallowed, not a silent cancel.

**M2 — `⏎` in a commit subject means two things.**
- Task: commit (the most frequent write).
- Screen: commit editor vs finish summary.
- Principle: H4; UX §1.1 (KLM: `⏎` after typing is anticipated, Rule 3).
- Evidence: commit editor, `⏎` in the subject moves to the body (`crates/graft/src/update/commit_editor.rs:601`);
  finish summary, `⏎` in the subject commits (`crates/graft/src/update/finish/keys.rs:105-107`). With `⌃⏎` unproven
  in Zed (known :206), the editor's real path is `⇥ ⇥ ⇥ ⏎` (`crates/graft-ui/src/commit_editor.rs:90-99`: subject →
  body → amend → button): 4 keys to commit a typed message, where the summary takes 1.
- Direction: one rule for both subjects. `⏎` in the subject commits (the summary's rule; the body is `⇥` or `↓`
  away), or neither does.

**M3 — `c` during a live operation opens an editor that refuses, and names three more keys.**
- Task: finish a merge after resolving outside the resolver (`space` on a `CONFLICTS` row), or any `c` while a
  merge/rebase is open.
- Screen: graph / changes pane → commit editor.
- Principle: UX §3.1 ("never land on a neutral step"), UX §5 item 2, pipeline step 4 ("never 5–6 where 2–3 do").
- Evidence: `commit_editor::open` does not look at `in_progress` (`crates/graft/src/update/commit_editor.rs:289-318`);
  the refusal then reads `… esc, ⇥ banner, then c finishes it` / `then c continue  a abort` (`:212-228`). The note
  counts this route at 5 (`m6-conflicts.md:505`, `editor-to-finish`); the banner's own `c` is 1.
- Direction: `c` with an operation live goes where the banner's `c` goes (summary / continue, or its refusal in
  words); the editor never opens to refuse.

**M4 — After he fixes a rejected push, the next `P` offers a force-push.**
- Task: push rejected → pull → rebase → push (team, daily).
- Screen: push box.
- Principle: H5 (error prevention), UX §3.5 (NN/g: "if you warn people too much, they stop paying attention").
- Evidence: a non-fast-forward sets `last_rejected_push` (`crates/graft/src/update/remote/in_flight.rs:168-170`); it
  is cleared only by a successful push (`:129`) or a branch change (`crates/graft/src/update/remote.rs:260-267`),
  never by the pull, merge or rebase that resolved the divergence. While set, `P` builds the lease variant
  (`crates/graft/src/update/remote/push_box.rs:88-106`): title `Force-push main → origin/main?`, line `rewrites history
  for everyone who has <sha>`, button `Force push` (`:167-182`). After `r r` onto the fetched upstream the push is a
  fast-forward; the box says the opposite.
- Direction: offer the lease only while the local tip still does not contain the rejected remote tip (or clear the
  flag on any write that moves the branch); the plain box otherwise.

**M5 — Checkout and cherry-pick are daily for him, but have no key and never appear on a bar.**
- Task: switch branch; cherry-pick a commit.
- Screen: graph; `.` menu.
- Principle: H6 (recognition), UX §3.3 ("only hide what's used rarely"), Hick UX §1.4.
- Evidence: keymap §2.1 files cherry-pick and checkout among "rare verbs" behind `.` (`docs/design/02-keymap.md:145-149`);
  §2.3's `Enter` "on a sidebar branch" checks out, and there is no sidebar. The only builders of either operation are
  the menu and drag (`crates/graft/src/update/context_menu/verbs.rs:74-88`, `crates/graft/src/update/drag.rs`). In
  the menu, Checkout is item 4 and Cherry-pick item 7 (`docs/shots/m5-ui-context-menu-verbs-160x48.txt:58-64`). The
  memory file lists both as daily.
- Direction: take both through the pipeline (prior art: lazygit, Sublime Merge, JetBrains) for a letter or a first
  menu slot; at least move them to the top of `.` on a branch/commit row.

**M6 — No daily task outside conflicts has a counted journey or a prior-art study.**
- Task: all of the table above except conflicts.
- Screen: all.
- Principle: CLAUDE.md "The feature pipeline" steps 2 and 4; UX §1.3, §5 item 2.
- Evidence: `docs/design-notes/m3-working-tree-undo-and-stash.md`, `m4-remotes.md`, `m5-history-and-ghost-preview.md`
  have no Need / Prior art / Journeys section (their `##` headings: Interfaces, Hard-to-reverse, Proposed changes, New
  UI, Risky groups). `docs/research/` holds conflict studies and `ux-principles.md` only. So the counts above (commit
  9, push 3, checkout 4 + find, cherry-pick 9 presses) have nothing to be judged against, and no walk fails on them.
- Direction: a research pass for commit, push/pull, checkout, stash, cherry-pick; a `journey-counts` block per task,
  asserted like M6's.

## Minor

**m1 — Three overlays, three rules for a key they do not know.** Task: any. Screens: confirm box / preview / menu.
H4, UX §3.2. Confirm box closes (`crates/graft/src/update/confirm.rs:173`); preview swallows
(`crates/graft/src/update/preview.rs:414-415`); menu closes and passes the key on (`crates/graft/src/update/keys.rs:131-140`).
Direction: one rule (swallow) for every modal box; the menu's pass-through stays, it is documented (keymap §2.1).

**m2 — `S` and `J` do nothing, silently, with the changes pane focused.** Task: stash before a pull. Screen: changes
pane. H4, rule 3 (no silent no-op). `S` opens only with graph focus (`crates/graft/src/update/stash.rs:171`), `J`
likewise (`crates/graft/src/update/journal.rs:153`); `c`, `p`, `P`, `f`, `u` fall through to the graph and work from
the same pane (`crates/graft/src/update/keys.rs:219-228` → `graph_key` `:257-292`, which has no `S`). The pull
refusal he gets there says `commit or stash first`. Direction: `S`/`J` answer from the pane as `c`/`p` do.

**m3 — When a write is confirmed is decided three ways.** Task: branch/checkout, stash. Screens: name prompt, `.`
menu, stash overlay. H4, UX §3.5, M4 note choice 4 ("a box iff the write is not undoable or destroys uncommitted
work"). `b` + `⇥` (check out) moves `HEAD` with no preview (`crates/graft/src/update/name_prompt.rs:256-263`); `.` →
Checkout previews (`verbs.rs:120-128`, "`.` ends in a picture", `:5-7`). Stash drop boxes
(`crates/graft/src/update/stash.rs:458-487`) though undoable; pop, which also removes the entry, does not
(`:214`). Direction: one written rule, applied; checkout (undoable) probably needs neither.

**m4 — The stash overlay reuses `⏎` and `p` for writes.** Task: stash. Screen: stash overlay. H4. `⏎` applies
(`crates/graft/src/update/stash.rs:213`) where `⏎` opens everywhere else (keymap §2.2 "Open / drill in"); `p` pops
(`:214`) where `p` pulls. Shown on its bar (`docs/shots/m3-06-stash-list-160x48.txt`: `⏎ apply  p pop`), both
undoable, hence Minor. Direction: keep the bar; consider `⏎` = preview (as `space`) if he trips on it.

**m5 — Only `--continue` lands the cursor on the new commit.** Task: commit, merge, rebase, cherry-pick. Screen:
graph. UX §3.1 ("Finishes → cursor on the new commit"). A finished write sets a `Done` line and refreshes
(`crates/graft/src/update/operate.rs:769-780`); the head landing exists but is set only by a finished continue
(`crates/graft/src/update/banner/stop.rs:90-104`). After `m m` on `feature` the cursor stays on `feature`'s row.
Direction: land every write that moves `HEAD` the way `--continue` does.

**m6 — The rejected-push line sends him to a key that refuses next.** Task: push rejected. Screen: status line.
H9. `push rejected: … · p pull first · P force with lease` (`crates/graft-ops/src/apply/remote.rs:239-241`). Pull is
`--ff-only` (`crates/graft-ops/src/operation.rs:88`); a push is rejected because the remote moved while he had
commits, i.e. diverged, so `p` answers `diverged · m merge it  r rebase onto it`
(`crates/graft/src/update/remote/in_flight.rs:176-190`). Same key count, one more surprise. Direction: word it `p
fetch and choose`, or name `r` after the fetch.

**m7 — "commit or stash first" names words, not keys.** Task: pull on a dirty tree. Screen: status line. H9 ("the
next key"). `crates/graft-ops/src/apply/remote.rs:251-253`. Compare the rebase refusal, which names `S`
(`crates/graft/src/update/operate.rs:917-926`). Direction: `c commit  S stash`.

**m8 — The commit editor opens with `subject empty` already showing.** Task: commit. Screen: commit editor. CLAUDE.md
status-field rule; H8. The refusal is computed on every frame (`crates/graft/src/view/commit_editor.rs:21-24`), is
`subject empty` for an empty subject (`crates/graft/src/update/commit_editor.rs:242-243`), and draws on its own row with
the options row muted (`crates/graft-ui/src/commit_editor.rs:235-243`, `:320-324`). The name prompt deliberately does
not do this (`crates/graft/src/update/name_prompt.rs:133-137`). No capture shows a freshly opened editor. Direction:
say it only when `Commit` is pressed with an empty subject.

**m9 — Every new local branch is a "risky state".** Task: branch. Screen: graph. H8, status-field rule. Any non-default
branch without an upstream gets a `RiskyState` finding (`crates/graft-ops/src/mistake_radar/rules.rs:323-330`),
titled `no upstream (<branch>)` (`crates/graft/src/view/radar.rs:226`), and a marked row's bar leads with `. explain`
(`docs/shots/m4-01-push-box-plain-160x48.txt`, last row). Unpushed work branches are his normal state. Direction: no
finding for a branch with no upstream until something depends on it (or only on the default branch, as
`UnpushedOnDefault` does).

**m10 — The commit-graph offer appears on any repository without the file.** Task: open. Screen: graph banner. H8.
Offered whenever `Absent { declined: false }`, whatever the history's size
(`crates/graft/src/update/commit_graph.rs:32-34`); it reads `no commit-graph file: history sorts on every open`
(`docs/shots/m5-hints-uncommitted-160x48.txt`). Once per repo, hence Minor. Direction: offer only past a history size
where the sort is felt.

**m11 — Plan shorthand reaches the screen.** Task: pull, diverged, when the tracking tip is unknown. Screen: status
line. H2; memory "ask in his terms". `pull stopped: … diverged — merge or rebase (M5); shell: git pull --rebase`
(`crates/graft-ops/src/apply/remote.rs:247-250`), shown via `crates/graft/src/update/remote/in_flight.rs:177-181`.
Direction: drop `(M5)`; name `m`/`r` or `f` first.

## Questions (no line proves them)

- **Q1.** After a conflicted `stash pop` is resolved, git keeps the entry. Does the app tell him it is still in the list
  and how to drop it? The resolver closes to the graph with `every file done ✓` and no key
  (`crates/graft/src/update/resolver/resolved.rs:202-213`); what happens to the entry was not traced.
- **Q2.** Keymap §2.2 lists `Ctrl+P` / `:` (palette) as live globals; nothing in `crates/graft/src/update/` binds them
  (grep for the palette, `Char(':')`). Is the palette scheduled (H7's novice path), or should §2.2 mark it planned?
