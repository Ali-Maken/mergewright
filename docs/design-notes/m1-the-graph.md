# M1 design pass: the graph after the gallery walk (2026-09-13)

Ordered by the user after walking the gallery (git/git, Ghostty, pixel tier,
`docs/shots/user-37-git-git-pixel-after-scroll.png`; Unicode
`docs/shots/35-git-git-unicode.txt`). No source read. ADRs 0022–0024,
Status: Proposed. Tickets 39–42 filled in; two new specs; 36 proposed wontfix.

**Promise:** the shape of history is obvious at a glance — on git/git as much
as on a five-commit repo. Ticket 38 (the gallery, already Fable) proves it
after these land: its git-git expectation line and TESTING.md entries change.

**Split into one file per section** (housekeeping/01, 2026-09-18): read
[`m1-the-graph/README.md`](m1-the-graph/README.md) for the section index.
Nothing deleted — cite the file, not a heading in this document.

## Sections

- [What the screenshot says (facts the decisions rest on)](m1-the-graph/01-what-the-screenshot-says-facts-the-decisions-rest-on.md)
- [Interfaces and data shapes](m1-the-graph/02-interfaces-and-data-shapes.md)
- [Hard-to-reverse choices (the three decisions)](m1-the-graph/03-hard-to-reverse-choices-the-three-decisions.md)
- [Where I'd differ from DESIGN.md / PROMPT.md (Proposed changes)](m1-the-graph/04-where-id-differ-from-designmd-promptmd-proposed-changes.md)
- [New UI](m1-the-graph/05-new-ui.md)
- [Genuinely risky tickets](m1-the-graph/06-genuinely-risky-tickets.md)
