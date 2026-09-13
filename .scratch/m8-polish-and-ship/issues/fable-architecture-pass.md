# m8-polish-and-ship — Fable architecture pass over the crate interfaces

Status: draft (standing ticket; the conductor numbers it with M8's specs)
Build with: **Fable** — `planner-fable`. A review of interfaces, not a build; doesn't count against the Fable build expectation (CLAUDE.md, "Standing rules → Models")
Needs: none. Runs after M8's other tickets that change public items, before the M8 review.

Why: before the name is chosen and the thing ships, one look at the whole system's shape from its public surface, with the source deliberately out of view, so the judgement is about interfaces rather than implementation detail.

## Read first
- CLAUDE.md, "Standing rules → Models" (M8 architecture pass)
- ARCHITECTURE.md
- every `crates/*/MAP.md`
- PROMPT.md §6.1 and §6.2 only
- `docs/reviews/` files from M2, M5 and M7's `/improve-codebase-architecture` runs, if any

**Never read source** (`crates/**/*.rs`, `prototypes/`). A MAP.md too thin to judge is itself a finding.

## Scope
In:
- Public interfaces as MAP.md lists them: leaks (a crate's internals visible in its signatures), shallow modules (interface as big as the implementation), crates doing two jobs, dependency-rule strain against ARCHITECTURE.md.
- MAP.md accuracy and thinness, as seen from the files alone.
- Suggested interface changes, each small enough to become one ticket.
Out:
- Code changes (builders do those from follow-up tickets).
- Implementation quality, tests, performance (the M8 review).
- Renaming the product (the M8 naming ticket).

## Signatures
None exposed or called. Output is a document.

## Acceptance
- [ ] `docs/reviews/m8-architecture.md` written, findings grouped Critical / Major / Minor, each naming the crate, the MAP.md line and a suggested change.
- [ ] Each Critical or Major finding stated as a candidate follow-up ticket (scope, signatures touched).
- [ ] No source file opened; the report says so.

## Gates
None (no code). The conductor commits the review file.

## Size
One context, under ~80k tokens.
