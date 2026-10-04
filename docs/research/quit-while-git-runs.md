# Prior art: quitting while a git write runs (and git hangs)

Owed by `docs/reviews/m8-13b-quit.md:144` (Major 4). Researched 2026-10-04. Primary sources only (GitHub issues, source, vendor docs/forums). Where a fact could not be confirmed it is written as a **question**.

## The question

User presses `q` / closes the window / Ctrl+C while a git write (commit, merge, rebase, checkout) is running. Worst case: git never finishes because a hook waits on stdin (commitizen-style `prepare-commit-msg`) or something holds a lock. What do the best tools do: wait, warn, kill, leave the repo paused?

## git itself (the floor every tool stands on)

- On SIGINT, SIGHUP, SIGTERM, SIGQUIT, SIGPIPE git deletes its own lock/temp files, then re-raises the signal: `sigchain_push_common` registers exactly those five ([sigchain.c](https://github.com/git/git/blob/master/sigchain.c)); `remove_tempfiles_on_signal` removes tempfiles and re-raises ([tempfile.c](https://github.com/git/git/blob/master/tempfile.c)). So a **catchable** signal leaves no stale `index.lock`.
- SIGKILL (or a crash) cannot be caught; the stale `index.lock` then blocks every later command with "Another git process seems to be running" until removed by hand ([GitLab runner issue 3338](https://gitlab.com/gitlab-org/gitlab-runner/-/issues/3338) shows it in the wild).
- **Question (not confirmed):** after a signal mid-`merge`/`rebase`, exactly which state files (`MERGE_HEAD`, `.git/rebase-merge/`) survive, i.e. whether the repo is reliably left "paused, continue or abort". The tempfile code above only covers lock/temp files, not sequencer state. Needs a test against real git before Graft claims it.
- **Question:** whether git forwards the signal to a running hook child, or the hook keeps the terminal. Not read in source.

## Per tool

| Tool | On quit/close while git runs | When git hangs (hook on stdin) | Sources |
|---|---|---|---|
| **lazygit** | Not found: no documented wait/warn on quit with a command in flight. | Hangs in "Checking out" on an interactive `post-checkout` hook; issue open since 2024-05, no maintainer fix. Reporter proposed: give the hook stdin, or kill it. Users say most UIs hang here; lazygit's escape is running the commit in a real terminal (`C`). | [#3557](https://github.com/jesseduffield/lazygit/issues/3557), [discussion #4344](https://github.com/jesseduffield/lazygit/discussions/4344), [#4320](https://github.com/jesseduffield/lazygit/issues/4320) |
| **VS Code SCM** | No cancel for a running git command. | Progress bar spins forever; "the only way to stop it is to close all instances of VS Code and re-open". Request to cancel a long commit / pre-commit hook is open, labelled feature-request, Backlog. | [#171353](https://github.com/microsoft/vscode/issues/171353), [#66821](https://github.com/microsoft/vscode/issues/66821), [#207396](https://github.com/microsoft/vscode/issues/207396) |
| **JetBrains** | **Warns.** Exit shows a confirm along the lines of "You have background tasks running. Are you sure you want to cancel them and exit?" Complaint: the dialog does not say *which* task. **Unconfirmed wording**: support forum returned 403; text taken from search snippet only. | **Question:** whether cancelling a VCS task kills git, and what state it leaves. | [support thread (403 to fetcher)](https://intellij-support.jetbrains.com/hc/en-us/community/posts/206864705-Which-background-tasks) |
| **Sublime Merge** | **Question:** quit behaviour with a running command not found in docs. Forum reports a Cancel button that freezes the UI 4-5 s during merge (snippet only, not opened). | Hook complaints are about PATH/env, not hangs; GPG prompts hang the app. | [GPG hang](https://forum.sublimetext.com/t/gpg-causing-sublime-merge-to-hang/56615), [rebase locks up #1098](https://github.com/sublimehq/sublime_merge/issues/1098), [husky #1106](https://github.com/sublimehq/sublime_merge/issues/1106) |
| **GitUp** | Not found. | Runs `prepare-commit-msg` async; **blocks commit while the hook runs and asks the user to wait**; hook failure shown once then blocks. GUI merges bypass hooks entirely. | [PR #2795](https://github.com/git-up/GitUp/pull/2795), [#167 bypass hooks request](https://github.com/git-up/GitUp/issues/167) |
| **Magit** | Not handled specially on Emacs exit (no code found in `magit-process.el`). | **Explicit kill, two-stage:** `k` in the process buffer "First try to interrupt the process. If that fails a second invocation instead kill the process." `C-g` at a git prompt aborts the prompt and terminates the process. | [magit-process.el](https://github.com/magit/magit/blob/main/lisp/magit-process.el) |
| **tig** | **Question:** tig appears to hand the terminal to git for commit, so hooks see a tty and Ctrl+C reaches git. Not confirmed in source. | Complaint: with a `prepare-commit-msg` hook, quitting the editor still commits. | [#543](https://github.com/jonas/tig/issues/543) |
| **gitui** | Not found. | Hangs on commit/rebase/merge/cherry-pick when `prepare-commit-msg` wants input (commitizen). Open bug, 2025-03; PR #2547 unmerged at time of read. | [#2546](https://github.com/gitui-org/gitui/issues/2546) |

Related, same pattern elsewhere: GitHub Desktop freezes on commit with a pre-commit hook ([desktop#11997](https://github.com/desktop/desktop/issues/11997)); Neogit hangs on a hook needing input ([neogit#1672](https://github.com/NeogitOrg/neogit/issues/1672)).

## What users praise / complain about

- **Complain, loudly and everywhere:** a hung hook freezes the UI with no way out but killing the app (VS Code #171353, lazygit #3557, gitui #2546, desktop #11997). This is the dominant complaint; nobody praises the status quo.
- **Complain:** a quit warning that doesn't name the task (JetBrains thread, unconfirmed text).
- **Praise / wanted:** an explicit cancel for the running command (VS Code #66821 / #171353 ask for exactly this); Magit's interrupt-then-kill is the only shipped one found.
- **Wanted:** a stale lock after a hard kill is a recurring "how do I fix" topic; tools that SIGKILL git hand the user this.

## Summary

| Tool | Waits | Warns on quit | Can stop git | Hang handled |
|---|---|---|---|---|
| git CLI | n/a | no | Ctrl+C; locks cleaned on catchable signals | no |
| lazygit | ? | ? | no | no (open bug) |
| VS Code | ? | ? | no (open request) | no |
| JetBrains | yes, by choice | yes (unconfirmed text) | cancel on exit (?) | ? |
| Sublime Merge | ? | ? | Cancel button (snippet only) | ? |
| GitUp | blocks commit while hook runs | ? | ? | partial (async hook) |
| Magit | no | no | yes: interrupt, then kill | yes, by hand |
| tig | ? (terminal) | no | Ctrl+C (?) | ? |
| gitui | ? | ? | no | no (open bug) |

`?` = not found in sources read; a question, not a "no".

## Does "after 10 s, `q` opens a stop box; stopping git leaves the merge paused to continue or abort" match any tool?

No tool found does all of it. The closest pieces: JetBrains' warn-on-exit-with-tasks (confirm, but no timer and no state promise), Magit's interrupt-then-kill (stop exists, but no prompt on quit and no timer), GitUp's "wait for the hook" block. No tool found uses a time threshold before offering stop, and none promises the repo's resulting state; that part would be an **invent**, justified by the dominant complaint (hung hook, no exit but killing the app). Two caveats before building it: (1) git's lock cleanup only holds for SIGINT/SIGTERM-class signals, so the stop should send SIGINT/SIGTERM first and SIGKILL only as a second step (Magit's order); (2) "leaves the merge paused" is **unverified**: whether `MERGE_HEAD` / rebase state survive an interrupted git is an open question above and needs a test against real git before the box promises it.
