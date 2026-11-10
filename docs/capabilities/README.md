# The capability database

Written for an agent that has never seen this repository.

`docs/capabilities/*.md` — one file per milestone — is Mergewright's feature
database: a hand-written, hand-verified entry for every distinct,
user-visible thing the application can do. It has two readers with equal
standing: the user, scanning it to see what he's built, and an agent handed
it cold, expected to be able to rebuild the system from it. This file is the
schema, the workflow for keeping it current, and how the generated artefacts
are produced.

**Never rewrite the prose.** Every capability's `How`/`Behaviour`/`Decided
in`/`Status` text is the asset here — it took real verification work to
write and it is load-bearing. Add and edit the machine fields; add new
capabilities in the same voice; never summarise or regenerate existing prose
from scratch.

## The schema

Each capability is a `###` heading, immediately followed by a fenced `meta`
block, immediately followed by the existing prose bullets:

```markdown
### M2-07. Focus and keys inside the details pane

```meta
id: M2-07
area: diffs
milestone: m2-details-and-diffs
status: works
```

- **How:** how the user reaches it.
- **Behaviour:** exactly what it does.
- **Proven by:**
  `crate::module::tests::a_specific_test_name`
  `crate::other_module::tests::{a,b,c}_shared_suffix`
- **Looks like:** `docs/shots/some-capture.txt` (omit this bullet if the
  capability has no capture — not every capability is visual)
- **Decided in:** `docs/design-notes/<milestone>.md` §Some section;
  ADR-0016.
- **Status:** free-text prose — what's verified, what isn't, what's known
  broken. This is the detail a bare status word can't carry.
```

### `meta` block fields (required, all four)

| Field | Meaning |
|---|---|
| `id` | Stable id, `M<milestone-number>-<NN>`. Must match the heading's id exactly — the checker enforces this. |
| `area` | What part of the product this belongs to — **never a milestone**. Existing areas: `foundations`, `terminal-rendering`, `graph`, `history`, `diffs`, `mistake-radar`, `working-tree`, `undo`, `stash`, `remotes`, `credentials`, `conflicts`, `hints-and-ui`. Add a new one if nothing fits; don't force a bad match. |
| `milestone` | The milestone slug it arrived in, e.g. `m2-details-and-diffs` — matches the source filename stem. |
| `status` | One of exactly `works` / `partial` / `unverified` / `broken` (`CLAUDE.md`'s four — **broken and unverified are never the same thing**: broken means known not to work as specified; unverified means believed correct but nothing has checked). This is the machine status; the prose `**Status:**` bullet carries the detail. |

### `unverified_aspect` (optional fifth field)

A `works` or `partial` capability can still have one specific, named thing
nobody has checked — usually how it looks on a real screen, which a
captured character grid can never settle (`CLAUDE.md` rule 5: colour), or
some other thing a test in this repo structurally can't reach (a real
terminal, a real network peer, an actual colour-vision-deficient viewer).
When that's true, set `unverified_aspect` to a short phrase naming exactly
what's unchecked — not "some things are unverified", but the specific
thing:

```
unverified_aspect: lane-colour distinguishability by eye at truecolor/256, and by an actual colour-vision-deficient viewer
```

**Use `unverified_aspect`, not `status: unverified`, when the capability
itself works and is proven to — by a real test, or by eye where a test
can't reach — and only one named aspect of it is unchecked.** Reserve
`status: unverified` for a capability that is *itself* unverified end to
end (nothing about it has been checked yet). Downgrading a working,
tested capability to `status: unverified` just because one aspect (often
colour) can't be captured is the opposite lie: it understates what's
actually proven. The two fields are independent — a capability can be
`status: works` and still carry `unverified_aspect`.

The checker cross-checks this field against the capability's own
`**Status:**` prose: if `unverified_aspect` is set, the prose must
actually say so somewhere (in its own words — "unverified", "not
verified", "left open", "never been judged", and similar phrasings all
count; see `UNVERIFIED_ASPECT_TRIGGERS` in `src/check.rs`). This catches
drift — someone editing the prose to no longer mention the caveat while
forgetting to clear the meta field, or vice versa. It does not enforce
exact wording; it only requires that the prose corroborate the field in
some recognisable way.

`CAPABILITIES.md`'s "At a glance" section counts how many capabilities
carry this field (separately from, not folded into, the status
distribution) and lists them all under "Unverified aspects". The HTML
page has a "has unverified aspect" filter checkbox for the same query:
"show me everything nobody has actually looked at."

### Prose bullets the checker reads

- **`Proven by`** — one citation per line (or per clause), each a
  backtick-quoted, `::`-joined path of two or more segments
  (`crate::module::tests::name`, or a module-level citation like
  `mergewright::view::journal`). A brace group —
  `` `crate::tests::{a,b,c}_shared_suffix` `` — expands to one citation per
  alternative. Anything backtick-quoted that *isn't* `::`-joined (a
  keystroke like `` `Enter` ``, a file path with a `/`) is prose, not a
  citation, and the checker ignores it — that's deliberate: it lets you
  explain a citation in surrounding words without every mentioned symbol
  becoming a hard requirement.
- **`Looks like`** (optional — omit entirely for a non-visual capability) —
  backtick-quoted `docs/shots/...` paths, comma-separated. A path containing
  `<...>` (e.g. `` `docs/shots/35-<stop>-unicode.txt` ``) is read as a
  *template* describing a family of captures, not a literal file, and is
  skipped — give at least one concrete example path in the same bullet if
  you want that family checked at all.
- **`Decided in`** — free text. The checker pulls out two kinds of
  citation from it: any backtick-quoted path ending `.md`, and any bare
  `ADR-NNNN` reference (backtick-quoted or not) — resolved against
  `docs/adr/NNNN-*.md` by numeric prefix, since the prose never spells out
  the slug.

## Adding a capability when a new feature lands

1. Find (or create) `docs/capabilities/<milestone>.md`.
2. Add a new `###` heading, in the same voice as its neighbours: a specific,
   user-visible claim, not an implementation detail.
3. Write the `meta` block: pick the next free `id` in that milestone, the
   right `area`, the milestone slug, and the honest `status`.
4. Write `How` / `Behaviour` exactly as you'd explain it to someone who has
   never used Mergewright.
5. Cite real, current test names under `Proven by` — copy them from the
   test file, don't reconstruct them from memory.
6. If it's visual, capture it under `docs/shots/` (200 KB cap, `CLAUDE.md`)
   and cite the path under `Looks like`.
7. Cite what decided the shape under `Decided in` — an ADR, a design note,
   or the design-notes file's own section.
8. Write the honest `Status` prose, and set the machine `status` field to
   match — if any part is unverified or known broken, say so; don't round
   `partial` up to `works`.
9. Regenerate (below) and run the checker before committing.

If a capability stops being true (removed, replaced, or its promise
changed), don't delete the entry silently: update its `Status` to say what
happened, or move it to "Deliberately not built" if it was never shipped.

## Regenerating

```
cargo run -p mergewright-capabilities --bin gen-capabilities
```

Regenerates two files from `docs/capabilities/*.md`, both marked generated
at the top — never hand-edit either:

- `CAPABILITIES.md` (repo root) — one entry point, organised **by area**
  (not milestone: "what can this thing do" is an area question), with the
  status distribution up top.
- `docs/generated/capabilities.html` — a single self-contained page: open it
  directly from disk (no server, no network — everything, including the
  data, is inlined into the one file). Filter by area, status or milestone;
  free-text search covers id, title, status prose, decisions and test
  names.

## The checker

`crates/mergewright-capabilities/tests/checker.rs` runs under the normal fast
tier (`cargo nextest run`, workspace-wide — never `-p`). It fails when:

- a cited test (`Proven by`) no longer exists anywhere in `crates/` — dead,
  renamed, or the citation typo'd;
- a cited capture (`Looks like`) isn't a file on disk;
- a cited ADR or design note (`Decided in`) isn't a file on disk;
- a capability's `meta` block is missing a field, has an `id` that doesn't
  match its heading, or an unrecognised `status` value.

A failure names the file, the capability's heading line, and the exact dead
citation — see `crates/mergewright-capabilities/src/check.rs`'s `Finding`/
`FindingKind` and `tests/checker.rs`'s assertion message. This is the reason
the crate exists: the first pass at these files cited four tests that were
real when their ticket was written and had been renamed or deleted by later
work inside the same milestone. A checker that only runs when someone
remembers to run it would have missed that again.

## Why this lives outside the product crate graph

`mergewright-capabilities` (`crates/mergewright-capabilities/`) is repo metadata
tooling, not product code — see `ARCHITECTURE.md`, "`capabilities`: repo
metadata tooling, outside the table". It has no dependency on any other
Mergewright crate, and nothing may depend on it, `crates/mergewright` and
`crates/mergewright-ops` included.
