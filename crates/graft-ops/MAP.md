# graft-ops — map
One line per public item. Update when a public item changes.

`graft-ops`: operations, the undo journal, Mistake Radar detection and Ghost Preview planning. Depends on `graft-git` and `graft-core` only; `graft-fixtures` as a dev-dependency.

## mistake_radar
- `CommitFacts` (struct) — what the rules know about one commit: `id`, `parents`, `subject`, `on_default_chain`, `parents_on_default_chain`.
- `RepoFacts` (struct) — `default_branch`, `remotes`, `previous_default_tips` (reflog tips of the default branch or its remote copy).
- `UpstreamFacts` (enum) — `None`, `InSync`, `Apart { ahead, behind }`.
- `BranchFacts` (struct) — one local branch: `name`, `tip`, `is_default`, `upstream`.
- `HeadFacts` (enum) — `Branch`, `Detached(Oid)`, `Unborn`.
- `Policy` (struct) — `Option<Severity>` per kind; `severity` / `set` (fn); `Default` = PROMPT §1.4, every kind on.
- `Finding` (struct) — `kind`, `severity`, `at`, `evidence`.
- `Evidence` (enum) — `Foxtrot { merge, first_parent, real_tip, branch }` (`branch` = the default branch), `Backward { merge, default_tip, into }`, `AccidentalPull { merge, branch, remote }`, `Risk(Risk)`, `Evil { files }`, `Duplicate { copies }`, `ForcePushed { remote_ref, previous, current }`.
- `Risk` (enum) — `DetachedHead`, `UnpushedOnDefault { branch, ahead }`, `NoUpstream { branch }`.

## mistake_radar::rules
- `MergedName` (struct) — what a git merge subject merged: `branch`, `remote`, `into`, `pull_request`.
- `merged_branch_from_subject` (fn) — git's own merge subjects (`Merge branch 'x' [of <remote>] [into y]`, `Merge remote-tracking branch 'r/x' [into y]`, `Merge tag 't' [into y]`, `Merge pull request #N from o/x`) → `MergedName`; anything else `None`.
- `classify_commit` (fn) — one finding per merge, foxtrot > backward > accidental pull; `None` for a non-merge or a kind the policy has off. Pure, O(1).
- `classify_refs` (fn) — `RiskyState` findings: detached `HEAD`, default branch ahead of its upstream, non-default branch with no upstream.

## mistake_radar::explain
- `Explanation` (struct) — `title` (upper case), `paragraphs` (≤ 3), `evidence` (labelled commits top→bottom), `highlight` (commits the graph keeps lit).
- `explain` (fn) — a finding to plain English, commits named through the caller's `short`.

## mistake_radar::probes
- `EvilVerdict` (enum) — `Clean`, `Resolved { files }` (the remerge diff differs only in files both sides changed: a hand-resolved conflict, never a finding), `Evil { files }` (files only one side, or neither, touched since the merge base), `NotCheckable(String)`.
- `ForcePushVerdict` (enum) — `NotCheckable(String)`, `Clean`, `Rewritten { remote_ref, previous, current }`.
- `Duplicates` (struct) — patch-id `groups`, the `window` looked at, `complete`.
- `probe_evil_merge` (fn) — one `git show --remerge-diff --name-only` spawn; non-merge/octopus/unreadable/`git` too old → `NotCheckable(reason)`, empty diff → `Clean`, else `Evil { files }` (m2-details-and-diffs/14).
- `probe_force_push` (fn) — `(repo, branch, remote_ref, current, stop)`; the remote-tracking ref's last reflog value against `current` through one `git merge-base --is-ancestor` spawn: exit 0 → `Clean`, exit 1 → `Rewritten`, empty reflog or any other failure → `NotCheckable(reason)` (m2-details-and-diffs/15).
- `probe_duplicates` (fn) — `(repo, window, stop) -> Result<Duplicates, RunError>`; three spawns (`log --format=%H`, `log -p`, `patch-id --stable` over the patches' `stdin`) grouped by patch-id, singletons dropped, ordered by `log`'s own newest-first order (m2-details-and-diffs/16).
- `finding_from_evil` (fn) — `Evil` → an `EvilMerge` finding on the merge; else `None`.
- `findings_from_force_push` (fn) — `Rewritten` → a `ForcePushedRemote` finding at `current`; `NotCheckable` and `Clean` → `None`.
- `findings_from_duplicates` (fn) — one `DuplicateCommit` finding per copy, `copies` = the rest of its group.
