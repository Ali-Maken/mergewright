# blame-commit-path — each blamed commit carries the file's path in that commit

Status: done
Blocked by: 18-trace-readers (landed; scout `.scratch/m7-power-tools/scouts/blame-place.md` "Exists already")
Crates: graft-git. (Holds graft-git: the conductor must not run it beside `journal-list`.)
Build with: **Sonnet**. One field read from a porcelain header already parsed; an oracle test against real git judges it.
Needs: none.

## Why this exists
`⏎` in blame lands on the line's commit with **its** file selected (B1). `git blame` follows whole-file renames, so the
path in that commit can differ from the blamed path. `BlameCommit` holds `previous` (the path in the parent) but not
the path in the commit itself (`crates/graft-git/MAP.md` `## blame`). Without it, `⏎` on a line older than a rename
selects nothing.

## Read first
- `crates/graft-git/MAP.md` `## blame`.
- This file. Nothing else.

## Scope
- In: `BlameCommit::path`, filled from porcelain's `filename` header; the fuzz target still builds.
- Out: every app change (20b, 20c, 20).

## Signatures (exact)
```rust
// graft-git::blame
pub struct BlameCommit { pub id: Oid, pub author: String, pub author_time: i64, pub summary: String,
                         pub path: PathBuf,                        // new: the file's path in this commit
                         pub previous: Option<(Oid, PathBuf)>, pub boundary: bool }
```

## Acceptance (name the verifying test at tick time)
- [x] `blame_commit_path_follows_a_rename` (tests/it/blame.rs; seen failing first). Uses the `session` fixture's existing rename `src/session.rs` → `src/auth/session.rs` rather than a new a.rs→b.rs fixture; oracle is `git blame --line-porcelain`'s per-line `filename`. Spec was: fixture renames `a.rs` → `b.rs`; blaming `b.rs` at HEAD, a line
      from before the rename has `path == "a.rs"`, a line after has `"b.rs"`. Oracle: `git blame --porcelain` on the same
      repo.
- [x] `a_repeated_filename_keeps_the_commits_path` (src/blame/tests.rs; seen failing first) — `parse_porcelain` unit test: an entry whose `filename` repeats for a later line of the same commit keeps it.
- [x] graft-git MAP.md `BlameCommit` line names `path` (checked by reading the line; no test).

## NOTES
- `filename` is now required on a commit's first entry (ParseError otherwise, same as `author`); git always prints it.
- Touches: only `BlameCommit {` literal is `record_commit` in blame.rs; grep finds no `BlameCommit` in crates/graft or fuzz/. Fuzz crate is standalone (not a workspace member), calls only `parse_porcelain`; not built.
- No compaction.

## Touches (check each, report how)
- Every `BlameCommit { .. }` literal (tests, fixtures): grep `BlameCommit {` workspace-wide; each gains the field.
- The fuzz target `blame_porcelain`: `cargo fuzz build` not required; `cargo check` of the fuzz crate if it is in the
  workspace, else say so.
- No app caller reads `BlameCommit` yet (grep `BlameCommit` in `crates/graft`); report the grep.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~15k.
