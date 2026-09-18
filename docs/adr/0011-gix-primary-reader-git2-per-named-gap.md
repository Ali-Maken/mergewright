# gix is the only full reader; git2 fills named gaps

docs/design/08-architecture.md §8.2 originally specified the `git` crate's `RepoReader` seam
with two complete adapters — `gix` and a `git2` fallback covering the
whole read surface (status, rev-walk, diff, blame). The justification was
design-shaped ("two adapters make a real seam"), not product-shaped:
nothing Graft does needs every read implemented twice, and a second full
adapter doubles the parsing and test surface (review finding M4).

Decision: reads are **`gix`-primary and there is no second adapter.** If a
specific capability turns out to be missing or broken in `gix`, that one
function gets a `git2`-backed body behind the same `pub(crate)` reader
signature, and the gap is named in `KNOWN_GAPS.md` — which `gix` API is
missing, which function works around it — so it can be retired when `gix`
catches up. This matches PROMPT.md Technology's own framing: `git2` is "an
acceptable fallback for anything `gix` lacks" — gap-by-gap, not a parallel
implementation.

## Consequences

- `git2` enters `Cargo.toml` with the first named gap, not before.
- The reader seam stays private to `git` (docs/design/08-architecture.md §8.4): callers never
  see which library answered a read.
- Each `git2`-backed function needs the same fixture tests as its `gix`
  neighbours — the cost is per gap, not per surface.
