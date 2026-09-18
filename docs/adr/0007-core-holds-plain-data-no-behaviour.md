# `core` holds plain data, no behaviour

`core` is the one crate every other crate is allowed to depend on, which
makes it tempting to also let it hold shared logic — a parsing helper here,
a small algorithm there — since "it's already a dependency of everything
anyway." We rule that out. `core` holds plain data types only: no
behaviour, no algorithms. Laying out a graph, parsing `git` output,
detecting a Mistake Radar pattern, replaying a journal entry, probing a
terminal — every one of those stays in the crate that owns it (`graph`,
`git`, `ops`, `term`, `ui`), never in `core`. A type belongs in `core` for
exactly one reason: two crates that may not depend on each other (siblings
under docs/design/08-architecture.md §8.1's layering, such as `graph` and `git`, or `graph` and
`ops`) both need the same shape, and `core` is the only place below both of
them.

This keeps `core` inert. Anyone reading it never has to ask "does this
constructor call out to the filesystem, the terminal, or another crate's
logic" — the answer is always no, by construction. It also keeps the
architecture's real decisions (how a graph is laid out, how a mistake is
detected) attached to the crate that owns them instead of smeared into the
one crate nothing can avoid depending on.

## Consequences

Checked against docs/design/08-architecture.md §8.2's existing placements, every current `core`
type already holds under this rule: `Oid`/`CommitId`, `Commit`, `RefName`/
`WalkStrategy`, `MistakeRadarKind`, `RenderingTier`/`Capabilities`,
`RasterFrame`, `JournalEntry`/`OperationKind`, and `JobId`/`Event` are all
inert records — the behaviour that produces or consumes each of them lives
in `graph`, `git`, `ops`, `term`, `ui` or `app`, never in `core` itself.

One placement needed a documented boundary rather than a change:
`core::Error` covers `core`'s own fallible constructors (for example,
rejecting a malformed `Oid` string). That's enforcing a type's own
invariant at construction time, not an algorithm acting on domain data —
the same boundary any newtype has in any language — so it stays in `core`.
Any future `core` type whose constructor needs more than that (parsing,
lookup, cross-referencing another type) is a signal that the type belongs
in the crate that owns the behaviour, with only its inert shape left behind
in `core`.
