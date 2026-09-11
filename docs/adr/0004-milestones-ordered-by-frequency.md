# Milestones are ordered by how often the user does the thing

The original plan ordered milestones by engineering dependency, which put three
daily workflows — stash, remotes, and conflict resolution — in milestones 7, 6
and 5 of 8. Since this tool exists to replace the primary user's daily Git
client, we reordered by frequency instead: stash joins staging in M3, remotes
move to M4 ahead of history rewriting, branching becomes M5 and conflicts M6.
Every milestone should leave behind something usable on a real working day.

## Consequences

M5 can produce conflicts a milestone before M6 builds the resolver. M5 must ship
an honest interim — detect the conflicted state, list the files, hand off to the
user's `git mergetool` or editor, keep Abort available — recorded in
`KNOWN_GAPS.md` with M6 as its expiry. It must never claim conflict resolution
is done.
