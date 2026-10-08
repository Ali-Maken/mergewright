# Scout: merge preview over a dirty index (m8-polish-and-ship)

## 1. What the merge preview checks
- Forecast: `forecast_merge` crates/graft-git/src/history_reads/forecast.rs:60 runs `git merge-tree --write-tree` on two commit oids (:80-99). Commits only — **reads neither index nor worktree**.
- Line: `conflict_line` crates/graft/src/update/preview/plan_lines.rs:271; `Clean` → `no conflicts expected` (:277). Only extra input is a put-back forecast, which exists only when the op sets aside.
- Pre-check: `check_merge` crates/graft-ops/src/operation/history.rs:582-585 = `head_is` + `source_holds`. **No `clean_tree`**.
- Set-aside: `Operation::sets_aside` crates/graft-ops/src/operation.rs:1475-1500 wraps Rebase/Rewrite/CherryPick, Pull (overlap only), Checkout/BranchCreate (SetAside); Merge falls to `_ => false` (:1494). So a merge neither refuses nor sets aside a staged change; git refuses.

## 2. Precedent
- `clean_tree` history.rs:559-563 → `Refusal::DirtyTree{count}` (staged or unstaged tracked, untracked never: :550-557). Used by checkout `OnEdits::Refuse` (:644), branch create (:658), branch-from-stash (:712).
- Rebase: no clean-tree rule, sets aside (history.rs:587-588, ADR-0045 §4). Cherry-pick: sets aside (operation.rs:1477; `check_pick` :596 no clean check). Pull: sets aside only on overlap, else "git's own words" (operation.rs:1468-1472).
- Refusal wording: `<verb> needs a clean tree` + keys `S stash N changed file(s)` — crates/graft/src/update/operate/failure.rs:207-216.

## 3. Design
- m5 hard choice 4 (docs/design-notes/m5-history-and-ghost-preview.md:164-166): **rebase** on dirty tree refuses `DirtyTree`. Merge not named.
- Superseded by set-aside: docs/design-notes/m6-stash-put-back.md:110; ADR-0045 §4 docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md:65-66 "For `Rebase` and `Checkout`". Amendment :258-265 (switch rule). **No line found saying what a merge preview shows over a dirty index/tree.** Searched docs/design/, DESIGN.md, docs/adr/, docs/design-notes/ for dirty/clean tree/uncommitted/autostash.

## 4. After the failed merge
- Plan failure closes the entry `EntryStatus::Aborted{reason}` — crates/graft-ops/src/apply/close.rs:188-197 (generic path; merge-specific path *could not confirm* without running).
- App: `operation_failed` failure.rs:14; no `refused`, not remote → `Status::sticky(error)` (git's text) at :121; refresh per job scope (:43-46). Where cursor/screen lands after: **could not confirm**.
