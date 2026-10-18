# mergewright-capabilities — map
The feature database: parses `docs/capabilities/*.md`, checks every citation against the repository on disk, and generates `CAPABILITIES.md` and `docs/generated/capabilities.html`.
Outside the crate table: no dependency on another Mergewright crate, and none may depend on it (ARCHITECTURE.md). One line per public item; modules are private, so every item is a re-export at the root.

## lib
- `Capability` (struct) — one parsed capability entry: what Mergewright can do, its status, decisions and citations.
- `Decision` (struct) — a `Decided in:` citation and the path it resolves to.
- `DecisionPath` (enum) — how a decision is checked: `Exact` path or `GlobPrefix` (ADR number).
- `Status` (enum) — `Works`, `Partial`, `Unverified`, `Broken`; broken and unverified kept apart.
- `Status::parse` (fn) — `(&str) -> Option<Self>`: read the status word.
- `Status::as_str` (fn) — `-> &'static str`: the status word.
- `LoadError` (enum) — why the database files could not be read or parsed.
- `load` (fn) — `(repo_root: &Path) -> Result<Vec<Capability>, LoadError>`: parse every `docs/capabilities/*.md`.
- `Finding` (struct) — one citation that does not hold, with where it was cited.
- `FindingKind` (enum) — what is wrong with a citation: dead test, missing capture or decision, unverified aspect not in prose.
- `check` (fn) — `(repo_root: &Path, &[Capability]) -> Vec<Finding>`: every citation checked on disk.
- `render_markdown` (fn) — `(&[Capability]) -> String`: `CAPABILITIES.md`.
- `render_html` (fn) — `(&[Capability]) -> String`: `docs/generated/capabilities.html`.
