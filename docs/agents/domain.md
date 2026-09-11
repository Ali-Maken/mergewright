# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the
codebase. This repo is **single-context**: one `CONTEXT.md` and one ADR folder at the root.

## Before exploring, read these

- **`CONTEXT.md`** at the repo root: the glossary produced by `/grill-with-docs`. Use its terms
  everywhere.
- **`docs/adr/`**: read the ADRs that touch the area you're about to work in.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest
creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and
`/improve-codebase-architecture`) creates them lazily when terms or decisions actually get
resolved.

## File structure

```
/
├── CONTEXT.md
├── docs/adr/
│   ├── 0001-graph-layout.md
│   └── 0002-terminal-graphics-protocol.md
└── src/
```

**ADRs live in `docs/adr/`.** `PROMPT.md` sometimes says to record a decision in `DECISIONS.md`:
wherever it does, write an ADR in `docs/adr/` instead (`CLAUDE.md`).

## Use the glossary's vocabulary

When your output names a domain concept (a ticket title, a refactor proposal, a hypothesis, a
test name), use the term as defined in `CONTEXT.md`. Don't drift to synonyms the glossary
explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal: either you're inventing
language the project doesn't use (reconsider) or there's a real gap (note it for
`/domain-modeling`).

Note that **"Graft" is a codename only** and is not a domain term. The real product name is
chosen in M8.

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently
overriding:

> _Contradicts ADR-0002 (Kitty graphics protocol), but worth reopening because…_

Changes to the spec or cuts to scope need the user's OK (through the conductor), and get
recorded as an ADR.
