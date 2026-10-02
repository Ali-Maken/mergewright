# G-first-run — clone from the empty state; one first-launch line

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: clone is a remote write outside any repo
(askpass, progress, partial-folder cleanup); the line is small.
Needs: `look-outside-repo-screen` (`EmptyStateView`, `EmptyPrompt`, `Exit::Open`), `palette-help-and-actions`
(the line names `?` and `⌃P`; "used once" means the palette opened).

## Read first
- `docs/design/01-view-mockups.md` §1.15 (the `c` bullet); `docs/design/02-keymap.md` §2.13, §2.14
- `docs/adr/0056-help-is-the-palette-on-this-place-and-the-tour-is-a-practice-repo.md` §4
- `docs/design-notes/m8-polish-and-ship.md`: Journeys J11, J15; "Where he lands" clone rows; New UI first bullet
- `crates/graft-ops/MAP.md` `## operation` (remote invocations, `remote: bool`), `## apply` (`RemoteHooks`);
  `crates/graft/MAP.md` `jobs` (askpass, progress), `update::credential`; `crates/graft-git/MAP.md` run/remote

## Decisions (planner)
- Clone is **not journaled**: there is no repository to journal into before it exists, and its undo is deleting a
  folder he asked for. It runs as a remote job (askpass, progress, `Esc` cancels) through graft-ops.
- Target `<this folder>/<name>`, name from the URL (last path segment, `.git` stripped; `git@host:org/x.git` → `x`).
  Target exists and is non-empty → refused before git runs: `x already exists here`. On failure or cancel, git's own
  cleanup removes a folder it created; a test proves it.
- First-launch line: `<config_dir>/graft/launches` holds a count or `done`. Shown (muted, status row) while count < 5
  and not `done`; `?` or `⌃P`/`:` opening the palette writes `done`. A problem line wins the row. Read/write failure:
  line not shown, one status warning (rule 3), never fatal.

## Scope
In: `c` on the empty state (prompt `clone › <url>`, `→ <cwd>/<name>` under it, progress in place, errors on the prompt
with the URL kept, `⏎ retry · esc cancel`); success → `Exit::Open(target)` → graph, cursor on HEAD; recents
remember it. Empty-state header actions and bar gain `c clone…` (§1.15). First-launch line.
Out: clone options (depth, branch, recursive) — not in v1. Tour `t` (G-tour).

## Signatures
```rust
// graft-ops::clone
pub struct CloneParams { pub url: String, pub into: PathBuf }
pub enum CloneError { TargetNotEmpty(PathBuf), BadUrl, Git { words: String }, Stopped }
pub fn name_from_url(url: &str) -> Option<String>;
pub fn clone(params: &CloneParams, hooks: RemoteHooks, stop: &StopFlag) -> Result<PathBuf, CloneError>;
// graft (pub(crate))
pub(crate) fn first_launch_line(config_dir: &Path) -> Result<bool, FirstRunError>;   // show it?
pub(crate) fn first_launch_done(config_dir: &Path) -> Result<(), FirstRunError>;
// EmptyPrompt gains Clone(FieldView, Option<CloneProgress>)
```
`StopFlag`/`RemoteHooks` are whatever graft-ops uses today for a cancellable remote job — the scout names them.

## Acceptance
- [ ] J15 `clone` = 3 (`c` URL `⏎`): `file://` bare fixture → that repo's graph, cursor on HEAD (walk test counts keys).
- [ ] HTTP Basic-auth fixture: the credential prompt appears, clone completes (test, as M4's walk).
- [ ] Bad URL → prompt stays, URL kept, reason + `⏎ retry · esc cancel` (test). Cancel mid-clone → no target folder
      left (test with `HungRemote`).
- [ ] Target exists non-empty → refused before git runs (test asserts no spawn).
- [ ] `name_from_url` table test (https, ssh, scp-form, trailing `/`, `.git`).
- [ ] J11 `first-launch` = 0: launch 1 shows the line; after `?` once, gone; launch 6 gone without use (tests).
- [ ] Captures: `docs/shots/m8-clone-prompt-120x36.txt`, `m8-clone-failed-120x36.txt`, `m8-first-launch-120x36.txt`.
      **Colour unverified**.

## Touches
- Empty state widget and its bar (5c), `every_key_the_bar_draws_is_answered` over it; askpass and credential prompt
  outside a repo (the prompt draws over the empty state — check); job pool without a repo; recents; config dir (G-rename
  list: `launches`); status row priority (problem > notice > first-launch); MAP.md graft-ops, graft, graft-ui.

## Scout questions
- Can graft-ops' remote invocation path run with no repository (cwd = target's parent)? What does it require of
  `Context` (file:line)?
- Does the askpass socket/credential prompt depend on an open repo or `AppState` parts absent on the empty state?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens.
