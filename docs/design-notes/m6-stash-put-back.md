# M6: a stash that clashes, edits set aside around a rebase or switch, menu places (2026-09-25)

Picks 15–17 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24-26). Review N3, N4 (other half), N7
(`docs/reviews/m6-usability-final.md` :125-208). Studies: `docs/research/commit-stash.md` addendum **A1–A4**,
`docs/research/checkout-cherry-pick.md` (**CC**), `docs/research/push-pull.md` (**PP**), UX = `ux-principles.md`.
Decision record: **ADR-0045**. Read: MAP.md of graft-core, graft-git, graft-ops, graft, graft-ui; keymap §2.1–2.9. No source.
**Amended 2026-09-26** (review `docs/reviews/m6-final-fixes-74.md` M1–M3; ticket 84): see the last section. Lines it
changes are marked `[A]`.

**Promise:** *a real conflict is resolvable without leaving the app* (M6), now also for a stash put back by hand or by
Graft; and *you cannot lose work* (M3): edits set aside are in the tree or in the stash list at every instant.

## Need
- Who: him, daily, in a team on dependent branches; unsaved edits are his usual state (CC "Need"; review N4).
- Tasks: pop a stash after a pull or checkout (daily); pull a diverged branch with edits; switch to a teammate's branch
  with edits; cherry-pick a fix (daily, memory `graft-daily-workflows`).
- Hurts: a clashing pop reads as a git error and sends him to the shell (N3, 11 keys); a dirty rebase is refused after
  confirm (N4, 16 keys walked); menu items move row to row, a scripted `. j×7 ⏎` started a rebase (N7).
- His words: "a human needs not take 5 to 6 steps; whatever is enough is enough" (field notes :51); picks 15–17, "all
  three took the recommended option" (:67).

## Prior art (T take · R reject · I invent)
| Aspect | Best answer found | Ours | Rec · why |
|---|---|---|---|
| Where a clash lands | JetBrains: its merge dialog, straight away (A3, manage-branches) | the resolver on the first clashed file (P5's landing) | **T**. Zero steps to the first conflict |
| Stash after a clash | git, every tool: kept (A1.1, A2) | kept until every file is done | **T**. Losing it is the one forbidden outcome |
| Drop after resolving | nobody; drop is by hand, ~3 keys, and forgotten (A2) | pop and set-aside: dropped by **commit id** when the last file is done, no box; `u` brings it back. Apply: kept | **I** (A4). Guard by undo, not confirm: GitUp `Cmd-Z` (CS §1.6), lazygit #4491 asks for it |
| Knowing it is a stash clash | no git state (A1.3); VS Code keys on the `Stashed changes` label (A4) | recorded on the journal's Pending line: which stash, why | **I** (A4). Survives relaunch; a shell pop stays generic |
| "Abort" a clash | no git verb (A4 question) | `esc u`: the pop's (or the rebase's) own undo; stash kept or restored | **I**. No new verb: the journal already inverts it exactly |
| Exit code | git exits 0 on a clashing autostash (A1.5; VS Code #325762, SM #1199) | read the index after every put-back | **T** |
| Set aside around rebase/switch | lazygit: stash, switch, pop behind one prompt (+1); JetBrains Smart Checkout (A3; CC §1) | always, no prompt; the preview says so (the preview *is* the prompt) | **T** lazygit/JetBrains, **R** their extra prompt: `y` in the preview already confirms |
| Mechanism | `git rebase --autostash` (A1.5) | Graft's own `stash create` → store → op → `stash apply <id>` | **R** `--autostash`: hidden `autostash` entry, no switch equivalent (A1.6, A4) |
| Staged split on put-back | git's autostash applies without `--index` (sequencer; `(?)`, the builder verifies) | without `--index`; preview says `N staged come back unstaged` | **T** git. **R** `--index`: refuses on a clash (A1.2, Magit #4993) (superseded 2026-09-26: ADR-0045 amendment b, tickets 30/30b) |
| Menu order | JetBrains, Sublime Merge: cherry-pick a top-level item, 2 (CC §2) | Checkout 1st, Cherry-pick 2nd, fixed; Checkout dims on the current branch, Cherry-pick never dims (71's amendment) | **T** (interim). Own keys + picker: M7, prototyped |

## Journeys (keys; UX 1.3; start: graph, cursor on the named row)
| id | Task | Keys | Count | Today | Best tool |
|---|---|---|---|---|---|
| `stash-pop-clash-to-done` | pop clashes, one block, done, stash gone | `S p` → resolver · `o ⏎` → dropped | **4** | 11 | lazygit ~10, VS Code ~10 (A2) | **Superseded 2026-09-26: 5 keys (`S p y o ⏎` / `S ⏎ y o ⏎`), stash preview, `m6-stash-preview.md`.**
| `stash-apply-clash-to-done` | apply clashes, done, stash kept | `S ⏎ o ⏎` | **4** | 8 (review's 11 minus `S d y`) | ~7 (A2 minus the drop) | **Superseded 2026-09-26: 5 keys (`S p y o ⏎` / `S ⏎ y o ⏎`), stash preview, `m6-stash-preview.md`.**
| `stash-clash-put-back` | give up: tree as before the pop | `esc u` (+`y` if a take was made) | **2** | 2, unchanged (not walked) | none found |
| `pull-rebase-dirty` | diverged, 3 edited files | `p r r` | **3** | refused | JetBrains 3–4 (PP §4) |
| `rejected-push-dirty-to-pushed` | from a rejection | `p r r P y` | **5** | 16 walked; 15 after 67 | JetBrains 3–4 |
| `switch-dirty` | branch row under the cursor | `. ⏎ y` | **3** | refused | lazygit 2, JetBrains 3 (CC §1) |
| `switch-dirty-clash` | put-back clashes, one block | `. ⏎ y o ⏎` | **5** | refused | JetBrains ~5 `(?)`, lazygit ~10 |
| `pull-rebase-stop-then-clash` | rebase stops, then put-back clashes | `p r r o ⏎ c o ⏎` | **8** | refused | none counted |
| `menu-checkout` | branch row | `. ⏎ y` | **3** | `. j×4 ⏎ y` 5 (8 presses) | 3 (filter, type, confirm) |
| `menu-cherry-pick` | commit row | `. j ⏎ y` | **4** (4 presses) | 4 (10 presses, moving) | JetBrains/SM 2 |
Decisions: one per journey (the picture), plus one per block taken. `u` after any of them: **1**, back to before.

**Where each event lands him.**
- Pop/apply clashes → overlay closes, changes pane opens, resolver on the first clashed file (`OpenedFrom::Pane`),
  notice `stash@{0} clashed in 2 files · esc, u puts it all back`. `Esc` → changes pane on `CONFLICTS`.
- Last file done, pop or set-aside → resolver closes to the pane; the drop runs; status
  `every file done · stash@{0} dropped ✓ · u brings it back`. Apply: `every file done · stash@{0} kept ✓`.
  `[A]` set-aside: `every file done · set-aside copy dropped ✓ · u brings it back`.
- Rebase/switch, put-back clean → graph on `HEAD`, the op's own done line `+ · your edits are back`.
- Put-back clashes → as a pop clash; notice `your edits clashed in 2 files · esc, u undoes the rebase too`.
- Rebase stops (edits aside) → P5 as today; the stash row `graft: your edits, set aside…` shows in `S`. `c` that
  finishes it → put back → clean line, or the clash landing. `a` → the rebase aborts, edits put back.
- Put-back fails for another reason → sticky `your edits are in stash@{0} · S p puts them back`, stash kept.
  `[A]` the words follow where `apply` says the edits are (ADR-0045 §5 amended).
- Graft killed mid-way → relaunch; the interrupted line names the stash: `… interrupted · edits in stash@{0}`.
- Relaunch with a clash open → stop line `stash@{0} clash from last run · 1 file conflicted`; `⏎` on `CONFLICTS`.
- `⏎` on a disabled menu slot → the menu stays; the status shows its reason (the drop menu's rule).

## Interfaces and data shapes
```rust
// graft-core journal (E) — additive; old lines parse and re-render byte-identically
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)] pub struct PutBack { pub stash: Oid, pub why: PutBackWhy }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum PutBackWhy { Pop, Apply, SetAside }
pub struct JournalEntry { /* … */ #[serde(default, skip_serializing_if = "Option::is_none")] pub put_back: Option<PutBack> }
// graft-ops (E)
pub struct PendingLine { /* … */ pub put_back: Option<PutBack> }            // written by `begin`, same serde rule
pub struct Clash { pub stash: Oid, pub why: PutBackWhy, pub paths: Vec<PathBuf> }
pub enum ApplyError { /* … */ Clashed { entry: Box<JournalEntry>, clash: Clash } } // like Conflicted: not retryable
pub struct StashClash { pub entry: EntryId, pub stash: Oid, pub index: Option<usize>, pub why: PutBackWhy }
pub fn live_clash(entries: &[JournalEntry], in_progress: Option<InProgress>, any_unmerged: bool,
                  stashes: &[(Oid, usize)]) -> Option<StashClash>;              // pure; rule in ADR-0045 §2
pub enum Operation { /* … */ DropPutBack { stash: Oid } }                   // kind StashDrop, pins [stash]
// graft-ops (F)
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct SetAside { pub files: usize, pub staged: usize }
impl Operation { pub fn sets_aside(&self, facts: &RepoFacts) -> Option<SetAside>; } // Rebase | Checkout, tracked edits
pub struct PreviewPlan { /* … */ pub set_aside: Option<SetAside> }
// graft-ui (F)
pub enum SideRoles { /* … */ SetAside }                                     // [A] ours "branch", theirs "yours"
// graft (G)
pub struct HistoryVerb { pub label: SanitizedText, pub op: Option<Operation>, pub disabled: Option<SanitizedText> }
// graft-ui (G)
impl ContextMenu { pub fn for_row(marked: bool, history: &[(SanitizedText, Option<SanitizedText>)], leading: usize) -> Self; }
```
App-side shapes (the clash outcome in `Operated`, `ResolverState::clash`, the stop kind) are named by the scouts;
the behaviour and words above are fixed.

## Hard-to-reverse choices
1. **Where the put-back is recorded.** Journal Pending line / a sidecar file / derived from marker labels.
   **Journal field.** One append-only source, relaunch-proof, additive serde; a sidecar goes stale when he finishes in
   the shell.
2. **Set-aside + op + put-back = one journal entry** / three entries / git `--autostash`. **One.** One `u` returns the
   tree exactly, staged split included, from the entry's backup (ADR-0009); three entries make `u` undo only the
   put-back and hide the edits again.
3. **Put-back without `--index`.** / `--index` then a fallback. **Without.** A fallback on failure is a retry (rule 2);
   git's own autostash does the same; the preview says what changes.

## Where I'd differ from DESIGN.md
None. Supersedes m5 hard choice 4 ("refused, never `--autostash`"), by his pick 16; PROMPT :316 asks for "stash and
switch". Doc rows owed (conductor): keymap §2.6 `Enter` ("a stash pop closes to the graph" → dropped, closes to the
pane), §2.9 `p`/`Enter` (a clash opens the resolver; pop drops at the end, apply keeps), §2.1 rare-verbs line (Checkout
and Cherry-pick hold the first two places); `Refusal::DirtyTree` doc; KNOWN_GAPS row 50. Journey rows above go into
`m6-conflicts.md`'s `journey-counts` block (the walk parses only that fence).

## New UI (existing surfaces only; no new screen, so no prototype)
- **Resolver on a clash:** as today, no banner row. Names `yours · main` | `stashed · stash@{0}` (ADR-0043 table), or
  `yours · main` | `set aside · your edits`; bar `o yours  t set aside`. Notice row words above. First look: the notice.
  `[A]` set-aside: `branch · main` | `yours · edits set aside`; bar `o branch  t yours` (ADR-0043 §4 amended).
- **Preview, rebase or checkout on a dirty tree:** one row under `working tree:`,
  `your edits: 3 files set aside, put back after` + ` · 1 staged comes back unstaged` when any. Cut from the right. (superseded 2026-09-26: ADR-0045 amendment b, tickets 30/30b)
  `[A]` and the conflict line forecasts the put-back too (below).
- **Stop line** while a clash is open and the resolver is shut: `stash@{0} clashed · 1 file conflicted`; shorter
  wordings drop the count, then the name (ladder as `stop_wordings`).
- **`.` menu on a commit row:** 1 `Checkout <b>` (or `Checkout — already on main`, dim), 2 `Cherry-pick <sha7>` (always
  enabled, per 71's amendment), then `Explain` if marked, then today's order; further `Checkout <b2>` after slot 2.
  80 and 120 columns alike (ADR-0039: contents never depend on width).

## Genuinely risky tickets
- **F `set-aside-around-rebase-and-switch`**: a git write whose inverse must be proven; resume, recover and undo all
  change; a wrong order loses edits and no diff shows it. The Fable candidate (tests in the conduct skill, not read).
- E is a journal-format change, additive, one pure rule with a table test: builder-opus after its scout.
- G and the acceptance ticket are routing and words. **Order:** A(66) → D(67) → G → B(68, 69) → C(70) → E → F → H.
  E and F last: their walks cross the bottom row (B) and `⇥` words (C).

## Amendment 2026-09-26 (review m6-final-fixes-74 M1–M3; ticket 84)
Decisions in ADR-0045 (§3, §5 amended, §8 new) and ADR-0043 §4 (amended). Here: the words and the journeys.

**Prior art, added rows.**
| Aspect | Best answer found | Ours | Rec · why |
|---|---|---|---|
| Forecast the put-back | not recorded in A1–A4 or CC (studies not re-read for this amendment) | `merge-tree` of the edits onto the landed tree, the m5 forecast's machinery | **I**. M5's promise; the box said `no conflicts expected` over a clash (review M2) |
| Naming the sides of a put-back clash | JetBrains Smart Checkout's dialog `(?)` — not in the studies | `branch · <b>` \| `yours · edits set aside` | **I**. UX heuristic 2 (`ux-principles.md:52`, ours/theirs flip): `yours` is his work, as the rebase row already says |

**Journeys.** Counts unchanged. `switch-dirty-clash` and `pull-rebase-stop-then-clash` walk `o` (take the branch);
keeping his edits is `t` instead, same count. Their landing now reads `branch · <b>` | `yours · edits set aside`,
bar `o branch  t yours`; their end reads `every file done · set-aside copy dropped ✓ · u brings it back`. Taking the
branch side everywhere then `⏎` drops the copy: his edits are one `u` away (ADR-0045 §3 amended).

**Preview: the conflict line folds the put-back.** Words, in the line that says `no conflicts expected` today:
| op forecast | put-back (`PreviewPlan::put_back`) | line |
|---|---|---|
| `Clean` | `None` or `Clean` | `no conflicts expected` (with edits aside, now true of the put-back too) |
| `Clean` | `Likely { paths }` | `your edits may clash when put back: <paths>` (paths listed and cut as the op's `may conflict:` list) |
| `Clean` | `NotChecked(r)` | `your edits' put-back: not checked (<r>)` |
| `Likely` / `NotChecked` | `Some(_)` | the op's line as today + ` · put-back not checked` (cut first) |
The row `your edits: N files set aside, put back after …` is unchanged.

**Failure words** (ADR-0045 §5 amended): `<the operation's failure> · <clause>`, clause by `EditsAt`; the clause is
kept whole at narrow widths, the failure cut first.

**Done line shape** (review m3, M4): every done line is `<what> ✓`, then ` · <also>`, then ` · <keys>`. The clean
put-back's `your edits are back` is `also`; `u brings it back` is `keys`.
