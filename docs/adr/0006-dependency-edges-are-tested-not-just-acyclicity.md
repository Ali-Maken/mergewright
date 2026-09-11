# Dependency edges are tested, not just acyclicity

PROMPT.md §6.1 says the compiler rejects cycles, so the crate structure
can't quietly erode. That's true but incomplete: cycle-rejection only stops
`ui` from depending on something that depends back on `ui`. It does nothing
about a direct, acyclic edge that still breaks the intended layering — `ui`
adding `git` as a path dependency compiles cleanly, no cycle, and yet
violates "`ui` never imports `git` or `ops`" (DESIGN.md §8.1). Cargo has no
opinion on layering, only on cycles, so the two checks aren't the same
thing and only one of them is automatic today.

Graft closes that gap with a fast-tier test, not a review habit: parse
`cargo metadata`'s dependency graph and assert it matches DESIGN.md §8.1's
edge set exactly — no more edges, no fewer. It lives in `app`'s test suite,
since `app` is the only crate that already depends on everything it needs
to inspect, and ships as an M0 ticket alongside the workspace scaffold.

## Consequences

Adding a crate, or adding a legitimate new edge between existing crates,
means updating this test in the same commit — a one-line tax each time,
paid by whoever is already changing the crate map. In exchange, a stray
dependency (someone reaching for the obvious-but-wrong `ui → git` import
under deadline pressure) fails the fast tier immediately instead of waiting
for a reviewer to notice it in a diff.
