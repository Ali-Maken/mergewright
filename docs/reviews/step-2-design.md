# Design review: DESIGN.md and ARCHITECTURE.md

Reviewed against `PROMPT.md`, `CONTEXT.md` and `docs/adr/0001`–`0007`. No code
exists yet; this is a design-only review. Findings only — nothing in
`DESIGN.md` or `ARCHITECTURE.md` was changed.

---

## The five verdicts

1. **Streaming topo-order vs. the 150 ms budget, no commit-graph file: does not hold up.** DESIGN.md §3.2/§3.3 names the exact algorithm ("the standard in-degree-counting topological sort git itself uses for `--topo-order`"), and that algorithm requires a full pass over the visible commit range to compute in-degree before a single commit can be emitted — the same reason git itself needs a full `limit_list` walk before `--topo-order` output starts. Without a commit-graph file's generation numbers, there is no way to know a commit's in-degree is final without having seen every one of its children, which can be discovered anywhere in the walk. §3.6's "the first chunk is sized to hit the 150 ms budget" is asserted, not shown to be possible under the algorithm the design itself chose, and the `LayoutCursor.walker_state` described as an "opaque, resumable snapshot" (§3.1) is incompatible with an algorithm whose real state (the in-degree table) is sized to the whole visible range, not to a lane count.

2. **`Vec<Row>` vs. "memory bounded by viewport": they disagree, as written.** §3.6 says virtualization only limits what's *rendered*, and explicitly: "rows up to `viewport_bottom` must already exist." `rows` is the append-only `Vec<Row>` backing the view (§3.6, first bullet). Nothing describes evicting or windowing `rows` once the viewport moves past a range. So a `G`-to-bottom jump on a 1M-commit repo requires `rows` to hold on the order of a million `Row` structs, not a viewport's worth — the opposite of PROMPT.md §1.2's requirement, which DESIGN.md itself quotes two lines earlier. Checkpoints reduce the *walk* cost of a deep jump (resume from the nearest checkpoint instead of row 0) but say nothing about the *memory* cost of materializing everything up to the target row, and the cold-start case — jumping to the bottom before any checkpoint past row 0 exists — isn't addressed at all.

3. **Undo of a merge/rebase with uncommitted changes present: specified, but unsafe.** §6.3's pre-op working-tree snapshot only fires for "discard, forced checkout, `reset --hard`, stash drop, clean" — merge, rebase, cherry-pick and revert are excluded. Yet §6.4's inverse for exactly those four kinds is "restore index and working tree to match" `refs_before`. If the user edits a file (uncommitted) any time after the merge/rebase finishes and before pressing undo, that edit was never snapshotted, and §6.7's safety rail only compares ref OIDs (`refs_after` vs. current) — a working-tree edit that doesn't move a ref passes the rail silently. Undo would then overwrite the working tree, discarding that edit with no backup. This is a direct violation of PROMPT.md's non-negotiable principle 2, "never lose work."

4. **gix + git2: the design commits to two full adapters, and that cost isn't justified.** DESIGN.md §8.2 states the `RepoReader` seam has "a `gix` adapter and a `git2` fallback adapter... two real adapters, so this is a real seam, not a hypothetical one" — i.e., git2 reimplements the entire read surface, not just the specific gaps gix has. PROMPT.md's own Technology section frames git2 as "an acceptable fallback for anything gix lacks" — narrower, gap-by-gap language. Building and testing a second complete adapter for status, rev-walk, diff and blame parsing is a real scope increase the design doesn't argue for beyond "codebase-design says two adapters is a real seam," which is a design-pattern justification, not a product one. ARCHITECTURE.md's own crate table (line 52) only lists `gix` under "Owns," while its "Where to start reading" prose says "a `gix` adapter and a `git2` fallback" — the two sections of the same file don't agree either.

5. **Undo journal location vs. worktrees: not specified, and breaks as written for linked worktrees.** §6.1 fixes the journal at `.git/graft/undo-journal.jsonl`. For a linked worktree (`git worktree add`), `.git` at that worktree's root is a *file* containing `gitdir: <main>/.git/worktrees/<name>`, not a directory — a literal `.git/graft/...` join fails outright. Even resolved correctly, the design never chooses between the per-worktree git-dir (`--git-dir`, private to that worktree) and the shared git-common-dir (`--git-dir`/`--git-common-dir` of the main repo) — a choice that changes real behaviour: branch delete, merge and rebase move refs shared across every worktree of the repo, so a per-worktree journal would hide entries a different worktree needs to see, while a shared journal needs to record which worktree's index/HEAD each entry belongs to (neither is discussed). §6 never mentions worktrees at all.

---

## Critical

**C1. Topo-order requires a full walk before first paint; the 150 ms budget is asserted, not shown achievable.**
DESIGN.md §3.2 (lines 595–602), §3.3 (`walker.next_topological()`, line 655), §3.1 (`LayoutCursor`, lines 581–583), §3.6 (lines 734–737).
The chosen algorithm is exactly the one that needs full in-degree computation over the visible range before Kahn's algorithm can emit its first ready commit — there's no commit-graph file assumed anywhere in the document, and PROMPT.md never requires the repo to have one. "The first chunk is sized to hit the 150 ms first-paint budget" is stated as a property of chunk size, but chunk size is irrelevant if the underlying walker can't start emitting until it has walked the whole range. This is the single riskiest unresolved claim in the document, given the graph is "the most important section" and the primary success metric (150 ms / 500 ms) hangs on it.
*Fix:* Either (a) show, with reasoning about gix's actual walk API, that an incremental topo-order is possible without a full pass (e.g., because gix always builds/uses a commit-graph, or falls back to a bounded approximate order for the first paint), or (b) design an explicit two-phase approach: paint the first screen in date order (cheap, streaming) and silently promote to strict topo order once the full walk completes in the background, documented as a deliberate fallback rather than left implicit. Either way, put this decision in an ADR — it changes a claim PROMPT.md treats as fixed (§1.2's incremental/streaming requirement).

**C2. `Vec<Row>` memory does not stay bounded by the viewport; the cost of a first deep jump is unspecified.**
DESIGN.md §3.6 (lines 734–751), §7 (lines 1158–1163, doesn't mention this risk at all).
As written, `rows` grows with "history walked so far up to the current scroll position," not with the viewport. A cold `G`-to-bottom on a freshly opened 1M-commit repo has no earlier checkpoint to resume from beyond row 0 (or wherever the first chunk stopped), so it needs a walk — and a `rows` allocation — proportional to the full depth jumped, run before the target row can render. The design doesn't say whether this blocks the UI thread, what happens to memory afterward (is `rows` ever trimmed back down once the jump lands?), or how this interacts with the heavy tier's 300 MB peak-memory budget.
*Fix:* Specify eviction: `rows` should be windowed (e.g., keep only `[nearest checkpoint below viewport, viewport_bottom + buffer]` and drop older entries, relying on checkpoints to resume if the user scrolls back), not append-only for the process lifetime. Specify explicitly that a deep jump runs as a cancellable background job (per PROMPT.md's "never block the UI"), with a progress/partial-result affordance rather than a synchronous stall.

**C3. Undoing a merge, rebase, cherry-pick or revert can silently destroy uncommitted work made afterward.**
DESIGN.md §6.2 step 1 (lines 1046–1048), §6.3 (lines 1061–1066), §6.4 (line 1091), §6.7 (lines 1130–1138).
No pre-op working-tree snapshot is taken for these four operation kinds (§6.3's list is discard/forced-checkout/`reset --hard`/stash-drop/clean only), yet undoing them explicitly "restore[s]... working tree to match" the pre-op ref state (§6.4). The only guard before undo runs (§6.7) diffs ref OIDs, which a plain uncommitted edit never changes — so the guard doesn't fire, and the edit is overwritten with nothing to recover it from. This is exactly the class of loss PROMPT.md's untracked-file fix (§3, quoted almost verbatim in DESIGN.md §6.3) was written to close for *discard*; the same hole is open for undoing merge/rebase/cherry-pick/revert.
*Fix:* Before applying the working-tree-restoring branch of an undo, snapshot the *current* working tree/index the same way §6.3 already does for discard (reuse the combined-tree mechanism), unconditionally — not only when the ref-comparison in §6.7 detects drift. Then the restore is always backed by a fresh recovery point, and "never lose work" holds even when the loss isn't ref-visible.

**C4. Undo journal location is unspecified for `git worktree` and breaks under the literal path given.**
DESIGN.md §6.1 (line 1019); no other mention of worktrees anywhere in §6.
`.git` is a file, not a directory, at the root of any linked worktree, so `<worktree>/.git/graft/undo-journal.jsonl` cannot be created as written. This is not a hypothetical: PROMPT.md explicitly requires handling worktrees (§6.3, §7.2) and the sidebar mockups list a WORKTREES section, so this path will be exercised. Beyond the literal bug, the design never decides whether the journal is per-worktree or shared across a repo's worktrees, which changes correctness (see verdict 5 above).
*Fix:* Resolve the journal path via the shared git-common-dir (so ref-level operations from any worktree land in one journal other worktrees can see), and record which worktree each entry's working-tree/index state belongs to, so an entry recorded in worktree A is never blindly replayed against worktree B's working tree. State this explicitly in §6.1, and add a fixture/property test for it once M3 builds the journal.

---

## Major

**M1. Keymap contradicts its own mockup: `e` is bound to two different actions in the same pane.**
DESIGN.md §1.1 (line 60: hint bar shows `e Explain`), §2.3 (line 424: `e` = "Open in `$EDITOR`"), §1.8 (no key shown for `[ Explain ]`).
The wide graph-view mockup's footer explicitly advertises `e` for Explain when a Mistake Radar marker is selected, but the Graph-pane keymap table (the actual spec a builder implements from) assigns `e` to opening `$EDITOR`, and no binding for Explain exists anywhere in §2. This is a real instance of the clash §2.1's own principle rules out ("that verb is never reassigned to something else anywhere the letter is live") — a builder following the mockup and a builder following the table would ship different bindings.
*Fix:* Pick one. Given `.` (context actions) already exists for exactly this kind of secondary action, the cleanest fix is to drop "e Explain" from the §1.1 footer and route Explain through `.` or a button-only path in the marker panel, and correct the mockup to match.

**M2. Escape-sequence sanitising is named as a responsibility with no design behind it.**
ARCHITECTURE.md line 54, DESIGN.md line 1199 — both just say "`term`... escape-sequence sanitising" with no further elaboration anywhere in either document.
PROMPT.md §6.3 (Security) requires cleaning five specific kinds of untrusted string (commit messages, author names, file names, file contents, branch names) before drawing them, precisely because a malicious repo can embed terminal escapes in any of them. The design gives no function signature, no integration point (is it applied once when `app` builds a view model, or per-widget in `ui`?), and no list confirming all five string categories are actually routed through it. Given this is a named non-negotiable principle and the one item in the review checklist about data-safety-adjacent security, leaving it a one-line label is under-specified enough that a builder could sanitise some paths and miss others.
*Fix:* Add a short subsection (even a paragraph) naming the function (e.g. `term::sanitize(&str) -> SanitizedString`), where in the data flow it's mandatorily applied (ideally: once, at the `core`/view-model boundary, so `ui` never receives an unsanitised `String` at all — perhaps via a newtype that can only be constructed through the sanitiser), and which of PROMPT's five categories it covers.

**M3. Pixel-tier strip caching and the 60 fps / 16 ms frame budgets are unaddressed.**
PROMPT.md §1.1 ("Cache them by row range, layout hash, theme and cell size...") and §1.2/§6.3 (60 fps on 1M+ commits; input-to-frame under 16 ms) vs. DESIGN.md, which never mentions a layout hash, a strip cache, a frame budget, or how virtualization (§3.6) is supposed to deliver 16 ms frames for the pixel renderer specifically. `Row` (§3.1) has no field that could serve as a cache key (no layout hash), and `RasterFrame` (§8.2) is described only as the pixel renderer's output shape, never as something cached.
*Fix:* Add the strip-cache design PROMPT.md already specifies the keys for: what a "layout hash" is computed from, where the cache lives (per-renderer? in `graph`? in `ui`?), and its invalidation triggers (scroll, resize, theme change, cell-size change). Tie it back to the 16 ms/60 fps budgets explicitly, since that's the mechanism meant to hit them.

**M4. gix/git2 adapter scope is an unreviewed, unjustified expansion of PROMPT.md's stated fallback.**
See verdict 4 above; DESIGN.md §8.2 (lines 1298–1304), ARCHITECTURE.md line 36 vs. line 52.
*Fix:* Either add this as a §9 "Proposed change" with the actual cost/benefit argument (test surface, maintenance, what capability specifically motivates a full second adapter rather than call-outs) so the user can approve it at the checkpoint, or narrow it back to PROMPT's framing: a `gix`-primary reader with a small number of named, git2-backed functions for whatever gix specifically can't do yet, not a parallel `RepoReader` implementation.

**M5. Push-undo doesn't say it reuses the mandatory force-push safety rail.**
DESIGN.md §6.4 (line 1096): "'undo' here opens a guided, freshly-confirmed re-push of the pre-push ref state."
Undoing a push necessarily moves the remote branch backward — a non-fast-forward update, which PROMPT.md §4 requires *always* go through `--force-with-lease --force-if-includes` with explicit confirmation, never a plain `--force`. The design's wording is compatible with that but doesn't say it, leaving a builder to either reinvent the confirmation flow or, worse, special-case "this is an undo, skip the usual rail."
*Fix:* One sentence: "This re-push goes through the same push confirmation and `--force-with-lease --force-if-includes` path as any other force push (PROMPT.md §4) — undo is not a separate, less-guarded code path."

---

## Minor

**N1. §2.1 undercounts its own exceptions.**
DESIGN.md lines 356–370 say "Three places do this deliberately" and list the conflict resolver, the interactive rebase editor and the operation banner. The first-run/empty-state screen (§2.13, lines 520–530) does the same thing (`c` = Clone instead of Commit, `i` = Init instead of gitignore, `o` = Open) but isn't counted among the three. Harmless today since first-run has no repo open yet, but the text reads as an exhaustive list and could mislead a later edit into thinking a fourth context-scoped rebind is a novel violation of the rule rather than an already-existing pattern.
*Fix:* Reword to "a small number of places" or explicitly add first-run as a fourth.

**N2. §7's "open design questions" doesn't list the topo-order/commit-graph risk (C1) as a question to validate.**
DESIGN.md §7 (lines 1151–1176) lists five tunable parameters but not the single biggest performance risk in the document. Given §7's own framing ("the M0/M1 prototype... should validate against a real terminal and real fixtures"), this belongs there.
*Fix:* Add an entry: whether topological order can stream without a full pre-pass, and what the fallback is if it can't, validated against the `git/git` heavy-tier fixture before M1 ships.

**N3. Rebase's clean-working-tree precondition (and whether Graft offers autostash) isn't mentioned.**
`git rebase` refuses to start with a dirty tree unless `--autostash` is passed; the design never says whether Graft passes it, offers it as a prompt, or just surfaces git's refusal. This interacts with C3 above — if Graft does adopt autostash, the popped stash needs the same untracked-file-safe handling as §6.3, not a bare `git stash create`.
*Fix:* One line in §6 or the merge/rebase feature notes (M5 scope) stating the chosen behaviour.

**N4. 50k+ refs (PROMPT.md §6.3) isn't addressed for sidebar/ref-badge rendering.**
Neither the sidebar mockups nor the layout algorithm section discuss what happens to branch-tree filtering, ref badge lookup, or ranking when a repo has tens of thousands of refs. Likely fine to leave for a later milestone's detailed design, but worth a one-line acknowledgment so it isn't forgotten.
*Fix:* A pointer in §7 or `KNOWN_GAPS.md`-adjacent note that ref-badge/sidebar performance at 50k+ refs is validated in M1's performance pass, not assumed free.
