# Ease pass (graph by role, commit, merge) — session log

Built to the three mocks accepted 2026-10-17 (ADR-0063, `prototypes/ease-mocks/`). Every mock behaviour is an
acceptance line; any departure goes to the user as a question before it is built.

Format: one line per helper or gate run, ending with the harness tool count.

## Session log

- 2026-10-17 · ease pass go (user ran `/conduct` after accepting the mocks). HEAD 739f3553. Steps 1–5 on file: need (field-notes:101–111), prior art (graph-readability, commit-stash, conflict-resolution, conflict-visuals-and-finish), decide + journeys + prototype (ADR-0063). planner-fable dispatched: note + specs.
- planner-fable pass 1 (note + 5 interface-first specs + acceptance): 14 groups, pass 2 needed (8 screen groups); Fable 2 (roles-and-route signature, resolver-picks); departure 1 (⏎ commits) already built P16; 13 questions → user before build; blocked ease-vocabulary (Q1–3,6–8,11), resolver-picks (Q7,12) · 60 tools
- user answered 13 questions: all (a) (note + field-notes). planner-fable pass 2 dispatched.
- planner-fable pass 2: 8 new specs + 4 updated; waves R‖K · V‖W · P‖F · G‖S · B‖Mp · L‖Fr · X alone · A; Fable 2; Q14 new (top line names branch twice) · 41 tools
- conductor: filed 01–14 (01 R … 14 A), Blocked by set, Status ready.
- scout 02: CommitSummary is String/Identity not Vec<u8>; when exists (author.when); NoSuchBlock exists (conflict.rs:329); merge_forecast needs &GitVersion (forecast.rs:71-81); files changed + clash places need new reads; reopen feasible. Unconfirmed: unrelated histories. Rec builder-opus. → spec fix with 01's scout.
- scout 01: made_on — Row.lanes is Option<BranchIdentity> (row.rs:45), LaneIdentity only on RowShape; ahead_behind no stop, WalkError (divergence.rs:52); merge_base no stop (compare_span does); counts usize not u32; of_name &[u8]. Unconfirmed: Candidate.name full vs short. · 11 tools
- scout 02 · 10 tools. Both → planner-fable spec-only fix (01, 02 + dependents).
- planner-fable spec fix: 01 full ref names, usize, count_between/commits_between, made_on from Row.lanes; 02 merge_forecast &GitVersion, unrelated = not checkable, Build with Opus; 05/07/09/10 + note Interfaces updated; merged rule replaced (stacked branches not merged).
- ease-pass start 9cb03ce8. W1: 01 builder-fable ‖ 02 builder-opus.
- builder-opus 02 (17cbbcc7, merged 4fb8f4bb): merge_forecast/MergeCheck, reopen, line_origins; 6/6 boxes (conductor wrote ticks from report — worktree based on c5235291, stale, read specs by absolute path). Gap unverified: rename/rename clash places. scout used. skills: implement · 82 tools
- gates after 02 merge (4fb8f4bb): fmt ok, clippy 0, nextest 6455/6455, doc ok; worktree removed. reviewer 02 dispatched.
- scout 04: message String not Vec<u8> (types.rs:57); Amend uses CommitParams (operation.rs:82); scratch index needs env on Invocation (types.rs:102), executor unconfirmed; NothingChosen new (explain.rs); u already restores HEAD+index (undo.rs:325-335). Rec builder-opus. → spec fix.
- scout 03: resolver.next_block not next (table.rs:323); c = resolver.finish (325); esc = Global app.back (77); u = Global app.undo (78, resolver.rs:379); finish.toggle_diff on ⏎/space (362-364); preview one context; editor/preview bars from screen code not hint lists. Rec builder-opus. · scouts 03 04 → planner-fable spec fix.
- builder-fable 01 (c3fa30a0): blocked needs graft-git (commits_from with stop, no exclude) — 8/9 boxes; trunk branch_status + Who no-base broken until then; route skips walked tips (deviation, tested). 6472/6472. scout used. skills: tdd, implement · 131 tools → resumed same builder, graft-git now free.
- planner-fable spec fix 03/04: 03 five new ids (graph.colours_off, resolver.review, commit.toggle_* ×3), no new ContextId, Opus; 04 Invocation.index: IndexFile (scratch copy of index) in 04, Opus; 05,07,08,10–12 + note use real ids, carry scout questions.
- builder-fable 01 resumed (e7ba3ddc, merged 862d40d2): commits_from in graft-git; 9/9 boxes (MAP box checked by reading, no test); route skips walked tips (deviation, tested); caps 10k/2k; unverified rev-list spawn cost → B. scout used. skills: tdd, implement · 176 tools total
- gates after 01 merge (862d40d2): fmt ok, clippy 0, nextest 6488/6488, doc ok; worktree removed. reviewer 01 + builder-opus 03 dispatched (04 waits for a cargo slot).
- reviewer 01 (09d6421d): 0C/4M/6m; ticks 9/9 held; role rule = ADR-0063; deviation accepted. M1 sibling as built-on (branch_status.rs:79-81,131-134); M2 merged reads Into(later) (status.rs:246-257); M3 synced stacked base reads merged (status.rs:264-279); M4 no-base Who lists main's authors (status.rs:143-166). All spec rules → planner-fable 01r spec. skills: code-review
- reviewer 02 (ebbc1b67): 0C/1M/6m; ticks 6/6 held. M merge_forecast 72 lines + file 395/400 + tree-id parse ×3, reason string ×2 (forecast.rs:191-262,226,380,200,79). m1 line_origins by text anywhere (origins.rs:44) wrong side for blank/}; m2 changed_paths ignores stop (history_reads.rs:314); m3 cat-file per clash uncapped; Q criss-cross changed. Heavy 2 red = known held_key Kitty; release perf 21/21. → 02r (M + m1 + m2) after 04; rest → gaps. skills: none
- builder-opus 04 dispatched (cargo slot freed).
