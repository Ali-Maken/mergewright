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
