# Scout 26: code lines behind two findings (graft-ops, graft-ui, graft)

## 1. Put-back (Recover MoveRef, `main` checked out) under autocrlf

**(a) Forward: the git commands, in order.** Paths are under `crates/graft-ops/src/`.
- Set-aside is on: `Operation::sets_aside` gives `Self::Recover(p) => recover::moves_checked_out(p, facts)` (operation.rs:1337). It counts staged or unstaged entries (1342-1347). `moves_checked_out` is true when HEAD is the branch (operation/recover.rs:134-141).
- The before-snapshot is taken first (apply/record.rs:92-107, `take_backup` → `snapshot::take` at record.rs:170). Then `run` runs these (apply.rs:249-291):
  1. `git stash create <msg>` (apply/set_aside.rs:80-83). This stores the **cleaned** blob.
  2. `git stash store -m … S` (set_aside.rs:121-126).
  3. `git reset --hard` (set_aside.rs:128).
  4. The plan: `git reset --hard <to>` (operation/recover.rs:224-229). This is not `--keep`, `read-tree -m -u` or checkout.
  5. `git stash apply --index S` (apply/set_aside/put_back.rs:66 → `apply` 99-109). If that fails, a second apply without `--index` runs (put_back.rs:83). The env only sets `apply.whitespace=nowarn` (put_back.rs:35-39).
  6. On a clean put-back, S is dropped by id (`set_aside::close`, set_aside.rs:228-248). That fits the observation that the stash list is unchanged.
- Which step writes src/a.rs through the smudge filter: step 5 (`stash apply`) writes his edit from S's cleaned LF blob, and with autocrlf that comes out CRLF. Steps 3 and 4 write HEAD's content. **Could not confirm by running git**; this is read from code only.

**(b) Undo of that entry.**
- The before-snapshot is taken for the unstaged file. `raw_record` (snapshot/raw.rs:80-107) keeps raw bytes for paths that differ from HEAD, are converted, and that smudge would not give back. The entry stores this as `worktree_raw_before` (record.rs:126).
- `undo::inverse` → `generic_steps` pushes `RestoreWorktree { tree: worktree_before, paths: scope(worktree_after, tree), raw }` (undo.rs:339-347). `worktree_after` is re-read with `snapshot::worktree_tree`, i.e. `add -A`, the clean filter (record.rs:271-273).
- `restore_worktree` (restore/steps.rs:272-283) and `scoped_paths` (steps.rs:592-606) give no paths when `from == to` (`return Ok(Vec::new())`, 604-606). On an empty list it returns before any write, `write_raw` included (steps.rs:280-283).
- `write_raw` writes only paths inside the scope (steps.rs:355-357).
- So if the cleaned after-tree equals the before-tree (CRLF normalises to the same LF blob, and C and D have the same src/a.rs), nothing is restored and the raw bytes are never used. **Could not confirm** that the two trees were equal in his run: that is inferred, not observed.
- When a path is in scope, it is written by `git restore --source` (smudge, steps.rs:302-311), then overwritten by `write_raw`.
- The stash step: `stash_step` with `PutBackWhy::SetAside`, clean put-back → no step (undo.rs:409-424).

## 2. Sticky status at 80 columns

- **The words:** `in_the_way_of` (graft-ops/src/operation.rs:1097-1112). Line 1104 is `"{first} (ignored) is in the way of {doing} · move it aside first"`. The whole string, "move it aside first" included, goes in as the `reason`. It is not split into `keys`.
- **`esc dismiss` appended:** `DISMISS` (graft/src/view/status.rs:76). `graph_problem` gives `Sticky { keys: None }` → `keys: Some(DISMISS)` (status.rs:96-99).
- **Fitting:** `fit_status_line` (graft-ui/src/graph_screen/hints/fitting.rs:356-371). It keeps `" · keys"` whole and cuts the reason with `fit(reason, room)` (line 370). `fit` (graft-ui/src/fit.rs:29) is a **left-to-right cut ending in `…`**, so the tail is always the part lost.
- **Callers:**
  - graft-ui/src/graph_screen/footer.rs:71
  - graft/src/view/frame/bars.rs:40, which passes `width-1` and a leading space
  - graft/src/view/frame/overlays.rs:60
- **Middle elision or path shortening:** none found. I grepped `middle|elide|shorten_path|ellipsize` in graft-ui, graft and graft-term. The hits are unrelated: journal_screen.rs:244 drops optional keys, and text_input.rs:205 is about memory. The only other width tool is the rung ladder (`fit_list`, fitting.rs:96; `StopWording` ladders, fitting.rs:210ff), used for stop rows, not for `Sticky`.
- **Tickets 28 and 28c:** grepping for width, 80, cut, fit and long path found no width requirement in either file. **Could not confirm** any width spec exists for these words.
