# ADR-0046 — The journal and the reflog are one list; `u` never enters a reflog-only row

Date: 2026-09-26
Status: **accepted** (conductor, 2026-09-26, on the user's M7 decisions; drafted by planner, M7 pass 2a). Direction from the user: prototype rev 1 scene 3 and field notes :72 ("the lost
commit was ✗ but couldnt go on it"); the screen itself is being tried as rev 2 now, and its findings may amend §3–§4.
Design note: `docs/design-notes/m7-power-tools.md` "Journal (`J`)", hard-to-reverse choice 1. Study:
`docs/research/reflog.md` §3, §4.4, §4.5, §4.8, §4.10.

## Context
- Graft's journal (`docs/design/06-undo-journal-design.md` §6.1–6.4) records what Graft did: every ref, the index and
  the worktree before and after, so `u` can put all of it back. git's reflog records what anything did to a ref,
  shell included, but not the index or the files.
- The user's lost-work case is rare and stressful. It is usually a shell `reset --hard` or a branch deleted in the shell,
  which the journal never saw. GitUp shows two panes and jj one log (`reflog.md` §3). Rev 1 showed one list with the
  lost commit as a non-row. He hunted it for about 2 minutes and about 50 keys (picks.txt 12:13:06–12:15:35).
- lazygit's `z` walks the reflog as undo (§4.10). An undo that cannot restore the index or the files breaks M3's promise
  "you cannot lose work". jj #4625 is the complaint.

## Decision
1. **One list in `J`**, newest first. It has three row kinds, and each looks different in glyph, words and position
   (UX §3.6):
   - `●` Graft: a journal entry.
   - `$` outside Graft: a reflog record that no journal entry explains. It is worded in plain words, and git's raw
     subject sits beside it.
   - `✗` lost commit: a commit no branch, tag, remote or stash reaches now. It sits indented under the event after
     which nothing held it. Several lost in a line fold into one row, `+N under it`.
2. **Dedupe by (ref, old, new).** A reflog record whose ref, old value and new value match a journal entry's
   `refs_before`/`refs_after` is that entry. It is drawn once, as `●`.
3. **`u`/Ctrl-Z and `U` walk journal entries only.** `u` never undoes a `$` row, a `✗` row or a reflog-only change.
   Those rows get **`⏎` put back** instead: a Ghost Preview confirmed with `y` (ADR-0044 §1).
   - The put-back is itself an Operation. Graft journals it, so `u` undoes the put-back.
   - It restores only refs. The preview says so, and names the files that change when the ref is the checked-out
     branch (rev 2 finding).
4. **Reachability is measured, not assumed.** The reflog's old values that nothing reaches are found off-thread, with
   `Stop`, and capped. The monorepo cost is measured in G-list before the design is fixed: if the read is slow, the rows
   fill in as it runs. It never blocks `J`.

## Consequences
- Undo stays exact: it only replays what Graft recorded in full.
- One place to look under stress, and the thing he looks for is a row he can select.
- A shell change is recoverable only as far as git recorded it (refs, not files). The row's words must not suggest
  more.
- Cost: a reflog read and a reachability pass on `J`. G-list measures it on the monorepo, and the pass is cancellable.

## Rejected
- **Two panes** (GitUp): he would have to know which pane holds his loss. Two places to look under stress.
- **`u` walks the reflog** (lazygit `z`): it restores refs without the index or files, so it can lose work.
- **Reflog only in a separate screen**: an extra key and a second vocabulary for the same question.
