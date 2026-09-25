# M6 final usability walk: plan (2026-09-25)

Input: `docs/reviews/m6-usability-final.md` (7 Major, 6 Minor, 3 Question). Read: that review, CLAUDE.md, the MAP.md
files of graft, graft-ui, graft-ops, graft-git, graft-core (sections cited in each spec), `docs/research/commit-stash.md`,
`checkout-cherry-pick.md`, keymap §2.1–2.3, `m6-conflicts.md` promise and landing sections, spec 50. No source.

## Fix specs (bugs; brownfield: each gets a scout; `Build with` blank)
All four groups touch `graft`, so they run one after another (ADR-0036). Suggested order A → D → B → C.

| Group | Spec (`.scratch/m6-conflicts/issues/`) | Review | Folds in | Crates |
|---|---|---|---|---|
| A radar | `radar-skips-bookkeeping-commits.md` | N1 | n5's radar half | graft-ops, graft |
| D preview gate | `preview-refuses-what-the-write-refuses.md` | N4 (bug half) | — | graft-ops, graft |
| B bottom row | `resume-words-say-what-happened.md` | N2 | — | graft |
| B bottom row | `a-box-owns-the-bottom-row.md` | N5 | KNOWN_GAPS M6/50; n3's stale-status part; Q4 (pin only) | graft (+graft-ui if the scout says) |
| C pane words | `tab-words-name-where-tab-goes.md` | N6 | n2; n1 (word; body after a find) | graft-ui, graft |

Causes, one line each:
- N1: the duplicate probe walks `--all` (backup refs, stash) where PROMPT §1.4 says visible refs → allowlist
  `--branches --remotes --tags HEAD`. Stash commits filtered out before the evil-merge probe.
- N4: `preview::plan` never runs `check`; `apply` does → `plan` runs it first; no preview on a refusal, the same words.
- N2: `resumed_done` words every closed resume as done → words from the entry's status and the `Resume` sent.
- N5: status rungs outrank an open modal's keys; the drift answer leaves `Busy` standing → a modal owns the row; the
  answering job's `Busy` ends when its box opens.
- N6: `⇥ graph` is a literal → one cycle-reading function feeds every `⇥` word and the refusal's count.

## N3 (a stash pop that conflicts) and N7 (checkout and cherry-pick keys): features

| Step | N3 | N7 |
|---|---|---|
| Need | Review N3; daily (after pull/checkout). **None in his words** in field notes | Study §Need; review M5/N7. None in his words |
| Prior art | **Partial.** commit-stash.md covers pop keys; Magit keeps the stash "on complete success" (§1.2); JetBrains opens its conflicts dialog on unstash (conflict-resolution.md:21). **Missing:** lazygit, VS Code, Sublime on a conflicted pop; complaints (lost/kept stash); anyone offering drop after resolve | **Done enough to decide.** Switch 3, pick 2, dirty switch 4 (autostash). Gaps: no praise quotes; GitUp/Fork/Tower not read |
| Decide | Not done. Take P5 (land in the resolver); invent the close words (`stash@{0} kept · S d drops it`) unless research finds better | Not done. Needs a keymap change: §2.1 names cherry-pick a "rare verb" behind `.` |
| Journeys | None. Today 11 (`S p` git error … `S d y`). Target ~7: `S p` → resolver, `o ⏎` → closes with the kept-stash words, `S d y` | Best counts only; Graft's not written |
| Prototype | Not needed: no new screen, P5's landing reused (the conductor confirms) | Needed for a typed branch picker (new interaction); not for a bare key |
| Size | **2 groups**: ops (a pop/apply leaving unmerged paths is its own outcome with its paths, not `ApplyError::Git`; the stale `Unmerged` words at `operation.rs:550-553`), app (landing, close words) | **1 group** keys + fixed menu slots; **+1–2** a branch picker; **+2** stash-around (shared with N4's other half) |

**Recommendation.**
- **N3 → M6 now.** The promise's own acceptance line names it: "a merge, a rebase that stops twice, a stash pop … each
  end with git's own state clean" (`m6-conflicts.md:9-11`). The resolver already works on a pop (m6-05 review :33);
  only the way in and the way out are missing. And `operation.rs:550-553` tells him to resolve "in the shell … in a
  later milestone", which the shipped resolver makes false. Cost first: a researcher addendum to `commit-stash.md` on
  conflicted pops (~1 pass), then one note section with its journey, then 2 groups.
- **N7 → M7, first group.** It is not the conflict promise. It needs a keymap change, a picker prototype, and one hard
  decision: set changes aside and bring them back automatically on a switch or pull (lazygit, JetBrains), or keep
  refusing (m5 hard choice 4). That decision wants an ADR. **Interim, M6 (small, bug-grade, not specced here):**
  put `Checkout` and `Cherry-pick` in the `.` menu's first fixed slots, above `Explain`. The review saw a scripted press
  aimed at Cherry-pick start a rebase (N7 :202-204).

## Minors and Questions
| Id | Where it goes |
|---|---|
| n1 untracked `bin` / `select a file` | **folded** → C (word; body only if the read exists) |
| n2 `c continue` vs `c finish` | **folded** → C |
| n3 keys in three places | stale-status part **folded** → B; "one place for keys" → KNOWN_GAPS |
| n4 `O` vs `o` in the one-file count | KNOWN_GAPS (note says the 4 assumes `O`, or the last take marks done: a design call) |
| n5 stash internals drawn | radar half **folded** → A; drawing half → KNOWN_GAPS :208 (update with the observation) |
| n6 force box names a SHA | KNOWN_GAPS (with P17) |
| Q2 palette | KNOWN_GAPS (stands) |
| Q3 opens on HEAD, not the changes row | KNOWN_GAPS as a question |
| Q4 drift undo, then `u` again | **folded** → B as a pinning test only |
| M6/50 lease box keys hidden | **folded** → B |

## For him to decide (plain words)
- Fix "a stash that clashes when you bring it back" now, before this milestone closes? (recommended: yes)
- When you pull or switch branch with unsaved edits: should Graft set them aside and put them back for you, or keep
  stopping and asking you to stash?
- Should check-out and cherry-pick get their own keys (next milestone), instead of living in the `.` menu?
