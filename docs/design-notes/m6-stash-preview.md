# M6: the stash list previews before it writes (2026-09-26) — PROPOSED, waits for the user

Source: usability walk 2 Major **C** and Q-b (`docs/reviews/m6-usability-2.md` :98-111, :198-200); earlier walk m4
(`KNOWN_GAPS.md` :773). Study: `docs/research/commit-stash.md` §1.1, §1.4, §1.6, A1.4. Read: keymap §2.3b, §2.9;
ADR-0045 §8; `docs/design-notes/m6-stash-put-back.md` Journeys. Ticket: `85`'s sibling, `86-stash-preview.md`.

**This changes approved direction.** Keymap §2.9 (M3, kept by pick 15) has `⏎` apply and `p` pop at the keypress, and
the put-back note's approved counts are 4. Nothing here is built until he says yes (question at the end).

**Promise:** M5's *you see the result before you confirm*, now also for putting a stash back.

## Need
- Him, daily: look at a stash, then pop or apply it (memory `graft-daily-workflows`).
- Hurts: walk 2 C — `space files` listed `M notes.md`; the next `⏎`, read as "open that file", applied the stash and
  landed him in a clash. Nothing was lost (`esc u y`), but a look became a write.
- His words: "the finish screen didn't read as a preview (so `⏎` committing was a surprise)" (field notes :60);
  "pick another key for confirming" → `y` confirms every preview (:58).

## Prior art (T take · R reject · I invent)
| Aspect | Best answer found | Ours | Rec · why |
|---|---|---|---|
| Confirm before apply/pop | lazygit asks before apply (switchable), pop and drop (§1.1 :33-36) | a preview, `y` confirms | **T** the confirm; **I** its content: lazygit's is a bare "are you sure", which his :54 says teaches `⏎` without reading |
| Apply on `Return` | GitUp, no confirm (§1.6 :122) | not taken | **R**. It is the key he read as "open" |
| See the stash before acting | lazygit shows its content beside the list (review Q-b, §1) | the preview lists its files, untracked ones included | **T** partly. The full diff beside the list stays `Space`'s job (gap row Q-b) |
| Forecast a clash | none found (A2: every tool learns it by applying) | the m5/ADR-0045 §8 forecast, base `stash^1` | **I**. Nobody forecasts a stash put-back; the machinery exists (84) |
| Dirty file in the way | git refuses before touching anything (A1.4) | the preview refuses, in words, no `[ Apply ]` | **T** git's rule, shown before `y` (ticket 67's rule: no Confirm for a write git refuses) |

## Journeys (keys; UX 1.3; start: graph)
| id | Task | Keys | Count | Today | Best tool |
|---|---|---|---|---|---|
| `stash-look-and-leave` | look at what a stash would do, write nothing | `S ⏎ esc` | **3** | writes on `⏎` | lazygit 1 (content beside the list) |
| `stash-pop-clean` | pop, no clash | `S p y` | **3** | 2 | lazygit `g ⏎` 2 (+ panel switch) |
| `stash-pop-clash-to-done` | pop clashes, one block, done | `S p y o ⏎` | **5** | 4 | lazygit ~10, VS Code ~10 (A2) |
| `stash-apply-clash-to-done` | apply clashes, done, kept | `S ⏎ y o ⏎` | **5** | 4 | ~7 (A2) |
+1 on pop and apply, bought back by one look that writes nothing. `⏎ ⏎` (his slip) lands on Cancel (ADR-0038 §1).

**Landings.** `⏎`/`p` → the preview over the stash list. `esc` → the list, cursor on the same stash. `y` → exactly
where `⏎`/`p` land today for each outcome (clean; clash → resolver, ADR-0045). Refused (in the way) → the box says why,
`esc` back.

## Interfaces and data shapes
```rust
// graft-ops, preview — `plan` covers StashApply / StashPop (scout: what it does today)
//   worktree:  WorktreeEffect::Updated { paths, more }  // stash^1→stash paths, then stash^3's (untracked) paths
//   conflicts: ConflictForecast                          // merge-tree --merge-base=<stash^1> <index tree now> <stash>
pub enum Refusal { /* … */ StashInTheWay { paths: Vec<PathBuf> } } // unsaved or untracked files it would write over
// graft — update/stash.rs: `⏎` / `p` open the preview on Operation::StashApply / StashPop; `y` confirms.
```

## Hard-to-reverse choices
1. **Preview on `⏎` and `p`** / on `⏎` only. **Both.** One rule for a write from the list (earlier walk m3, row :772).
2. **In the way = refusal** / a warning with `[ Apply ]`. **Refusal.** git refuses those (A1.4); a Confirm git then
   refuses is review N4's shape.
3. **`y` only** / also `p p`. **`y` only.** ADR-0044 §3, one confirm key; `m m`/`r r` stay the named exception.

## Where I'd differ from DESIGN.md — proposed keymap §2.9 text (conductor applies on his yes)
| `Enter` | Open the **apply preview** for the selected stash (Ghost Preview frame, §2.3b): the files it writes (untracked ones included), whether it may clash with the tree now, `stash@{n} stays in the list`. `y` applies; `Enter` is the choice under the cursor (Cancel first, ADR-0038 §1); `Esc` back to the list. Refused in the box, nothing written, when unsaved or untracked files are in the way (git refuses those). A clash opens the resolver on the first clashed file; the stash is kept (ADR-0045) |
| `p` | Open the **pop preview**: as `Enter`, with `stash@{n} is dropped · u brings it back`. `y` pops. A clash opens the resolver; the stash is kept until the last file is done, then dropped, `u` brings it back (ADR-0045) |

Replaces (2026-09-26, usability walk 2 C): `Enter` applied and `p` popped at the keypress, no preview
(m3-working-tree-undo-and-stash/06; kept by `m6-stash-put-back.md` Journeys). §2.3b's `y` row gains "or the stash
list's `Enter`/`p`". Journey fence (`m6-conflicts.md` :515-516): 4 → 5 each; add `stash-look-and-leave 3`.

## New UI (existing surface: the Ghost Preview box; no new screen, so no prototype)
- Title `Preview: apply stash@{0}` / `Preview: pop stash@{0}`; under `working tree:` the paths as every preview lists
  them; the conflict line in the preview's own words (`no conflicts expected` / `may conflict: notes.md` / not
  checked); one row for the stash's fate. First look: the conflict line.
- Refused: the box's refusal state, `your unsaved edits to notes.md are in the way`, keys `esc back`.
- 80 columns: paths cut as every preview cuts them; the fate row is kept whole.

## Genuinely risky tickets
- The forecast must agree with `git stash apply` (clean / clash / refused). Judged by an oracle test that runs git on
  a copy, so Opus, not Fable.

**Question for him:** "In the stash list, `Enter` and `p` act at once today. Should they first show what would happen
(files, and whether it clashes), with `y` to go ahead? One more key for apply and pop. yes / no"
