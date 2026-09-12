# Known gaps

Everything unfinished, stubbed or deliberately deferred. Nothing here may be
described as done, and nothing in the README, the first-run tour or a milestone
summary may imply any of it exists.

Every entry names an **expiry**: the milestone that closes it, or "not planned".

---

## Deferred stretch goals (agreed at step 1, 2026-09-11)

These were in the original scope list and were cut before any code was written.
Each is either something Sublime Merge does not do well either, or something the
primary user does not use. Don't build them without asking.

| Gap | Why deferred | Expiry |
|---|---|---|
| Visual bisect (mark good/bad in the graph) | Rare workflow for this user; the graph work it needs is better spent on daily views | Not planned for v1 |
| Worktree switcher | Rare. Opening a worktree directly still works: it looks like a normal repo | Not planned for v1 |
| LFS awareness (pointer files, locks) | Writes go through the user's own `git`, so their LFS filters already work. Graft adds no LFS UI | Not planned for v1 |
| Sparse-checkout awareness | Rare for this user; correctness risk is high and the payoff is low | Not planned for v1 |
| Several repos open at once in tabs | A window-management feature, not a Git feature. One repo per process | Not planned for v1 |
| Submodule operations (update, sync, init) | Submodules **display** in the sidebar and can be opened; operating on them is deferred | Not planned for v1 |

## Platform

| Gap | Why | Expiry |
|---|---|---|
| Windows | Explicit non-goal for v1 (ADR-0005). No Windows CI, no Windows terminals in the manual checklist | Not planned for v1 |

## Design gaps (step 2 review, 2026-09-11)

Named in `docs/reviews/step-2-design.md`; deliberately left out of DESIGN.md
until the milestone that needs them.

| Gap | What's needed | Expiry |
|---|---|---|
| Rebase with a dirty tree (review N3) | DESIGN.md never says whether Graft surfaces git's refusal or offers `--autostash`. Decide in M5's design; if autostash, the popped stash needs DESIGN.md §6.3's untracked-safe snapshot handling, not a bare `git stash` | M5 |
| 50k+ refs in the sidebar and ref badges (review N4) | PROMPT.md §6.3 requires handling repos with 50k+ refs; branch-tree filtering, badge lookup and ranking at that scale are undesigned. Validate in M1's performance pass, not assumed free | M1 |

## Design gaps (step 2 review round 2, 2026-09-12)

Named in `docs/reviews/step-2-design-round-2.md`. (R2-2, backup-ref naming,
is already fixed in DESIGN.md §6 and ADR-0010 — not a gap.)

**REQUIRED M1 TICKET — R2-1: post-promotion checkpoint state on the
no-commit-graph path.** Whoever drafts M1's tickets must create a ticket for
this; it is not optional polish. DESIGN.md §3.2/§3.6 never say what
`walker_state` checkpoints contain after promotion on the fallback path. The
naive reading — snapshot the fallback topo walker's in-degree state — is
O(remaining history) per checkpoint: 100+ MB of checkpoints on the 100k
no-commit-graph fixture, busting the heavy tier's 300 MB peak-memory gate
(the exact configuration ADR-0008 orders that gate to run). Required fix, one
or two sentences in §3.2/§3.6 plus the matching implementation: after the
background walk completes, retain the emitted order as a compact commit-id
list; post-promotion `walker_state` is an index into it, so checkpoints stay
small on both paths and §3.1's "bounded by parallelism" annotation holds
unconditionally. Expiry: **M1**.

| Gap | What's needed | Expiry |
|---|---|---|
| Unconditional snapshot cost on high-frequency index ops (R2-3) | Every stage/unstage fires the full §6.3 snapshot (status walk, untracked hashing, commit object, backup ref, two journal lines), and §6.8's manual-only retention lets backup refs pile up at keystroke rate. Off-thread, so no UI block — but measure on the heavy fixture in M2; if staging lags on untracked-heavy repos, propose scoping (e.g. skip untracked hashing for ops that provably can't touch untracked files) as an ADR, never silently | M2 |
| Commit-graph offer plumbing unspecified (R2-4) | §3.2's "declined, remembered" flag has no stated home — put it in a file under `git rev-parse --git-path graft/…`, per §6.1's rule. And `git commit-graph write --reachable` is a repo write, which §6 says only `ops` may run: route the offer through `ops`, journaled as an ordinary non-undoable entry or explicitly exempted | M2 |

## Prototype gaps (step 3 spike, 2026-09-12; revised at the step-3 fix)

The graph spike (`prototypes/graph-spike/`, ADR-0012 + its Correction). The
original claim "proved Kitty + iTerm2 alignment and scrolling" was wrong —
that was pty-capture evidence, which proves emission, not rendering. After
the fix, the Kitty path is verified to the strongest level available without
human eyes: a real Ghostty 1.3.1 answered the probe, accepted the transmit
and placement at `q=1`, and reported zero errors. The iTerm2 tier is no
longer a gap — the user ran it in a real iTerm2 on 2026-09-12 and the graph
drew correctly (ADR-0012). It is throwaway code; these are the open
questions that remain.

| Gap | What's needed | Expiry |
|---|---|---|
| Resize frame busts the frame budget | A terminal resize clears and retransmits the whole strip: ~32–41 ms in the Ghostty bench, against a 16 ms budget. One frame per resize, so the spike leaves it. M1's fixed-height strips with ids (DESIGN.md §3.8) must not inherit a whole-viewport retransmit on every SIGWINCH | M1 |
| Is ED-frees-image-data Ghostty-specific? | Only Ghostty 1.3.1 was tested. Unknown whether kitty-the-terminal (or other kitty-protocol terminals) also free image data on `ESC[2J`. M1 must not assume uploads survive a clear anywhere | M1 |
| Sixel and plain-text rendering tiers unproven | Spike exercises only Kitty and iTerm2. Sixel alignment/scroll strategy and the text-cell fallback graph need their own validation | M1 |
| tmux passthrough and SSH untested | No `tmux` escape wrapping attempted; detection over SSH (env vars lie) not exercised | M1 |
| Probe/input conflicts and resize | Spike loses typeahead during capability probes (two readers on stdin) and never re-probes cell size on SIGWINCH. M1 needs the input mux and resize handling named in ADR-0012 | M1 |

## Rendering tiers: order and the Unicode tier (recorded 2026-09-12, settle at the start of M1)

Raised by the user at the step-3 fix. **Recorded, not decided** — DESIGN.md is
deliberately untouched and no ADR is written yet. M1 opens by settling this.

| Gap | What's needed | Expiry |
|---|---|---|
| **Unicode tier is the daily view, not a fallback** | The user works in the Zed and VS Code terminals, which have no image protocol, so the Unicode graph is what he sees most days. M1 must build it as a first-class tier — designed, not degraded — and judge it by eye in those terminals to the same standard as the pixel tier in Ghostty. Nothing may describe it as a downgrade | M1 |
| **Protocol preference order conflicts with DESIGN.md §4.2** — **needs an ADR when settled** | The user's order is Kitty → **Unicode** → iTerm2 → Sixel, with Sixel used only if it demonstrably beats Unicode. DESIGN.md §4.2 says Kitty > iTerm2 > Sixel and treats Unicode as the downgrade path. These disagree. M1 must resolve it and then do both: **write an ADR recording the order and why** (this is not optional — it is a design decision that overrides an approved DESIGN.md section), and amend §4.2 to match. Neither DESIGN.md nor an ADR may be edited before the decision is made | M1 |
| Sixel in VS Code unverified | VS Code has `terminal.integrated.enableImages`; whether it gives a usable Sixel tier there is untested. Check before assuming VS Code is image-less — it changes what the order above should be | M1 |

## M0 gaps

| Gap | What's needed | Expiry |
|---|---|---|
| **A signal kills Graft with the terminal still in raw mode.** `TerminalSession` restores on quit and on panic, both verified under a pty (m0/03). SIGINT, SIGTERM and SIGHUP still take their default action: the process dies without unwinding, so neither the panic hook nor `Drop` runs, and the shell is left with echo, canonical mode and `ISIG` off. Measured, not assumed: a `kill -TERM` under a pty leaves `ECHO=false ICANON=false ISIG=false`. Note this is *not* Ctrl+C at the keyboard — raw mode means that arrives as a key event, not a signal (ticket 09 maps it to quit) — it is `kill`, a closed terminal window, and a logout | A handler that restores the terminal and re-raises the signal with the default action, so the exit status stays honest. **Decided 2026-09-12: ticket 09.** The handler only sets a flag and wakes the event loop; the loop returns normally so `TerminalSession`'s `Drop` does the restoring. No restore and no `exit()` inside the handler. Ticket 09's acceptance now requires a measured pty check for all three signals | **M0 (ticket 09)** |

## Scheduled interim states

These are *temporary*. Each one is a real gap between the milestone that creates
it and the milestone that closes it, and each must be visible in the UI while it
exists.

| Gap | Created by | Expiry |
|---|---|---|
| **No in-app conflict resolver.** M5 can produce conflicts through merge, rebase and cherry-pick before M6 builds the three-pane resolver. M5 must detect the conflicted state, list the conflicting files, offer the user's configured `git mergetool` or `$EDITOR`, and keep the operation banner with Continue / Skip / Abort available. M5 must not claim conflict resolution works | M5 | **M6** |

---

*Builders and reviewers: add rows here the moment something is stubbed, naming
the milestone and ticket it came from. An entry here is not a failure; a missing
entry is (CLAUDE.md).*
