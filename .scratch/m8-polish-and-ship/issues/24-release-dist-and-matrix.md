# G-release — dist builds, a tap formula, an installer; Linux and git-version runs — nothing published

Status: ready
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: release config is small; the matrix and the
Linux run need judgement about what was actually exercised (rule 1).
Blocked by: 23
Needs: `docs-readme-and-man` (`docs/man/graft.1` goes in the archive).

## Read first
- `docs/adr/0057-tributary-and-what-ships.md` (whole; §1 binds: nothing published, registered, tagged or pushed)
- `docs/adr/0005-macos-and-linux-only.md` — Decision only
- `docs/research/install-docs.md` §2 (dist), §3 (tap), §4 (Gatekeeper), §6 (journeys)
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows "Binaries, tap, installer", "Notarisation"; Journey J16
- `ARCHITECTURE.md` "The crates" (one section)

## Decisions (planner)
- dist (v0.33 or the latest 0.x) in the workspace; targets `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`; installers `shell` and `homebrew`; `tap =
  "<owner>/homebrew-tap"` with `<owner>` a placeholder the user fills at publishing; **`publish-jobs = []`** until the
  trademark search clears (ADR-0057). Archive includes the man page; the formula installs it to `man1`.
- The CI workflow dist generates is committed but never pushed by an agent. `dist plan` and `dist build` (host target)
  run locally; output to `docs/release/plan.txt`, `docs/release/build.txt` (summary lines).
- `cargo install --locked --path crates/graft` works from a clean clone (README says so; tested in a temp
  `CARGO_HOME`).
- **Linux run**: the fast tier on Linux in a local container if one is available (scout); results to
  `docs/release/linux.txt`. No container → stated unverified, and the user decides at the stop (push to a private
  repo for CI is his call, not an agent's).
- **git-version matrix**: `scripts/git-matrix.sh` builds git 2.38.0 (the floor, `graft-git/src/version.rs:5`) and the
  latest release from source into `~/.cache/graft-git-matrix/` and runs the version-sensitive tests with the git
  binary pointed at each: the rebase-preview replay tests (the preview must equal what git does; checked on 2.54.0
  only) and `git_alone_stops_on_a_pick_of_an_ancestor`. Results to `docs/release/git-matrix.txt`. A failure on 2.38 →
  the floor rises to the oldest passing version, with a TESTING.md line; never a skipped test.
- Thread-teardown fix and the HungRemote failed-wake path (unverified on Linux since M4) run in the Linux pass.

## Scope
In: dist config, generated workflow (committed, not pushed), local plan/build, formula artefact, installer script
artefact (shellcheck-clean), cargo-install check, Linux pass, git matrix, TESTING.md install rows (J16 as a person's
step on a clean account after publishing). Out: publishing, tagging, notarisation (ADR-0057), homebrew-core.

## Signatures
None (config and scripts). How tests find git (an env override or `PATH`) is the scout's to name.

## Acceptance
- [ ] `dist plan` lists 4 targets, 2 installers, man page in each archive (`docs/release/plan.txt` lines quoted).
- [ ] `dist build` on this Mac produces the host archive; `tar tf` lists the binary (`graft` until G-rename) and
      `graft.1` (quoted).
- [ ] Generated formula passes `brew style` on the file (read-only; never `brew install` on his machine).
- [ ] `cargo install --locked --path crates/graft` into a temp `CARGO_HOME` succeeds; binary prints `--version`.
- [ ] Linux: fast tier result line quoted from `docs/release/linux.txt`, or "no container — unverified" stated.
- [ ] git matrix: per version, tests run / passed quoted from `docs/release/git-matrix.txt`.
- [ ] No push, no tag by this ticket's agent: `git tag` and `git status -sb` before/after quoted.

## Touches
- Workspace `Cargo.toml` metadata (dist), `.github/workflows/` (new), `crates/graft` `[[bin]]` (name stays `graft`
  until G-rename); scripts; TESTING.md install rows; README install section (G-docs wrote it — check it matches);
  G-rename list (dist app name, formula name, archive names, `~/.cache/graft-git-matrix`).

## Scout questions
- Is docker, podman, colima or a Linux VM available on this machine?
- How do the version-sensitive tests choose the git binary today (file:line)? Is there a toolchain to build git from
  source (make, a C compiler, `curl`)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.
Matrix and Linux runs are scripts writing to `docs/release/`; the builder reads summary lines only.

## Size
One context, ~55k tokens.
