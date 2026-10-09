# G-release — dist builds, a tap formula, an installer; Linux and git-version runs — nothing published

Status: partial (2026-10-09; see NOTES)
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
- [x] `dist plan` lists 4 targets, 2 installers, man page in each archive (`docs/release/plan.txt` lines quoted).
      **Verified by** `scripts/release-local.sh` → `docs/release/plan.txt`: `graft-installer.sh`, `graft.rb`, and
      `graft-{aarch64,x86_64}-apple-darwin.tar.xz`, `graft-{aarch64,x86_64}-unknown-linux-gnu.tar.xz`, each with
      `[bin] graft` and `[misc] CHANGELOG.md, README.md, graft.1`.
- [x] `dist build` on this Mac produces the host archive; `tar tf` lists the binary (`graft` until G-rename) and
      `graft.1` (quoted). **Verified by** `scripts/release-local.sh` → `docs/release/build.txt`:
      `graft-aarch64-apple-darwin/{README.md,CHANGELOG.md,graft.1,graft}`; the archive's binary runs
      (`usage: graft [<path>]`).
- [ ] Generated formula passes `brew style` on the file (read-only; never `brew install` on his machine).
      **Not met** (`docs/release/build.txt`): 3 offences, all in dist's own template (`Style/SoleNestedConditional`,
      `Style/MutableConstant`, `Style/TrailingCommaInHashLiteral`); ours (`desc` starting with an article) fixed.
      And the formula does **not** put `graft.1` in `man1`: dist's template has no man-page support and puts
      leftovers in `pkgshare` (install block quoted in `build.txt`). See NOTES.
- [ ] `cargo install --locked --path crates/graft` into a temp `CARGO_HOME` succeeds; binary prints `--version`.
      **Half met**: install from a fresh clone into an empty `CARGO_HOME` ok in 163 s, binary runs
      (`scripts/cargo-install-check.sh` → `docs/release/cargo-install.txt`). **Graft has no `--version` flag**
      (`graft: unknown option '--version'`); adding one is a `crates/graft/src/cli.rs` change → blocked (another
      builder holds graft's sources).
- [x] Linux: fast tier result line quoted from `docs/release/linux.txt`, or "no container — unverified" stated.
      **Verified by** `scripts/linux-fast-tier.sh` → `docs/release/linux.txt` (Docker, `rust:1-trixie`, git 2.47.3,
      aarch64), commit ecdfd742: `Summary [ 514.953s] 6183 tests run: 6127 passed (4 slow), 56 failed, 14 skipped`;
      doctests 3/3.
      The line is quoted; **the result is a failure** — see NOTES for what fails.
- [x] git matrix: per version, tests run / passed quoted from `docs/release/git-matrix.txt`. **Verified by**
      `scripts/git-matrix.sh`: 2.38.0 `26 tests run: 16 passed, 10 failed`; 2.40.0 `26 tests run: 26 passed`;
      2.56.0 `26 tests run: 26 passed`. Floor not raised — see NOTES.
- [x] No push, no tag by this ticket's agent: `git tag` and `git status -sb` before/after quoted. **Verified by**
      the commands, before (start of session) and after (before the final commit): `git tag` → no output both
      times; `git status -sb` → `## worktree-agent-ac82c9f1762b8cec6` (no upstream, so nothing ahead/behind)
      both times. No `git push`, `git tag`, `gh release` or registry login was run.

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

## NOTES (builder, 2026-10-09)

**Status: partial.** Config, workflow, scripts and runs are in. Two acceptance items are not met (formula style
and man1; `--version`), and two ticked items quote results that fail (Linux, the 2.38 floor).
No compaction. Scout used: `.scratch/m8-polish-and-ship/scouts/24-release.md`.

What landed:
- `Cargo.toml`: `[profile.dist]`, `[workspace.metadata.dist]` (dist 0.33.0, 4 targets, `shell` + `homebrew`,
  `tap = "<owner>/homebrew-tap"`, `publish-jobs = []`, `install-path = "CARGO_HOME"`, no updater).
  `crates/graft/Cargo.toml`: `description`, `repository`/`homepage` with `<owner>`, `[package.metadata.dist]`
  `dist = true` (the workspace is `publish = false`, which dist reads as "don't ship") and `include` the man page.
  No `[[bin]]` was needed: the binary is already `graft`. `Cargo.lock` unchanged.
- `.github/workflows/release.yml`: generated by `dist generate`, committed, never pushed. It triggers on a version
  tag push (`release.yml:41-45`, and on every pull request — not read what it does there); on a tag it runs `gh release create`
  (:279); with `publish-jobs = []` no tap is pushed.
- Scripts: `release-local.sh` (plan/build/tar/brew style/shellcheck), `linux-fast-tier.sh`, `git-matrix.sh`,
  `cargo-install-check.sh`; all shellcheck-clean. Output in `docs/release/`. `.tools/` gitignored.
- dist 0.33.0 is **not on crates.io** (newest there 0.32.0); installed from its `v0.33.0` git tag into `.tools`.

Not met / for the user at the stop:
1. **Floor (git 2.38).** 10 of 26 fail on 2.38.0, every one the rebase forecast that
   `crates/graft-ops/src/preview/chain.rs:31` (`MERGE_BASE_MIN` = 2.40, checked at :78) turns off below 2.40 — 7 print
   "git 2.40+ needed for a rebase forecast"; the 3 `left_behind_*` fail on a fork that cannot follow without a clean
   forecast. 2.40.0 passes 26/26. The ticket's rule says raise the floor to 2.40; I did not, because on 2.38–2.39 the
   app behaves as designed (says "not checked") — it is the tests that assume 2.40, and raising the floor changes
   `graft-git/src/version.rs`, the README line and the man page (graft's sources are held by another builder).
   Decide: raise to 2.40, or keep 2.38 and document that the fast tier needs 2.40.
2. **Formula and man page.** dist's Homebrew template (`templates/installer/homebrew.rb.j2`, v0.33.0) installs only
   binaries; `graft.1` goes to `pkgshare`, so `man graft` would not find it after `brew install`. Options: patch the
   formula at publishing (one line, `man1.install "graft.1"`, plus `brew style --fix` for the 3 template offences),
   or a custom publish job. Not done: editing generated output is a workaround, and nothing is published yet.
3. **`--version`.** Graft has no such flag (`cli.rs` takes a path, `-h`, `--generate-man`). Needs a graft-crate change.
4. **Linux: 56 failures (55 the run before), neither kind fixed (source changes, out of scope).**
   - ~36 are pty walks (`tour_tasks`, `palette_find`, `outside_repo`, `quit_waits::*`, `clock_pin`, …) that wait
     20 s for a Unicode marker while the app draws the **ASCII** tier (`>` cursor, `*`/`|` graph; e.g.
     `crates/graft/tests/it/quit_waits/hangup.rs:81` waits for `▸`). Setting `LANG=C.UTF-8 TERM=xterm-256color`
     did not change it (80 → 77). Cause **not found — a question**, not a defect: tier detection reads
     `TERM_PROGRAM` and friends (`graft-term/src/detect/environment.rs:53-64`) and probes the terminal; I did not
     trace which input decides it.
   - ~15 are capture mismatches (`docs/shots/... does not match what the code draws`), 2 insta snapshots for the
     `non-utf8-ref-name` shape (`graft-ui` graph column, `graft-graph` corpus), and a few singles
     (`m6_stash_put_back::kill` "a git the killed operation started still runs after 30 s", a journal put-back
     undo, `cherry_pick.rs:281`). Unverified whether they share the ASCII-tier cause.
   - **Intermittent**: `put_back_forecast::a_rebase_with_drops_measures_the_put_back_from_where_it_lands` passed
     in one run and failed in the next (`put_back_forecast.rs:328`, `Some(Clean)` vs `Likely ["x.txt"]`). Cause
     unknown; one sighting.
5. **Teardown on Linux (the M4 gap).** With `--init` every `run::tests::stop` and `stop_hooks` test passes on
   Linux (they had failed only because `bash` as PID 1 left orphans as zombies). Modules that drive a HungRemote:
   188 passed, 7 failed — 3 `m8_walk::clone`, 3 `remote_by_key::capture_*`, 1 `partial_fetch::capture_*`; their
   messages are capture mismatches and one "no cursor row" pty wait, none a hang or teardown assert.
6. **Hermeticity findings** (tests read the host): a global git identity is needed (first run: "Author identity
   unknown"); some tests open the checkout itself as a repo (an export fails `NotARepo`); the fast tier needs
   git 2.40+.
7. Side effect on this Mac: the first `brew style` installed Homebrew's own style gems into
   `/opt/homebrew/Library/Homebrew/vendor/bundle` (Homebrew's gitignored dev bundle; no formula installed, no tap).
   Left in place: I could not tell whether that folder held anything before.
8. Docker: images `rust:1-bookworm` and `rust:1-trixie` (official) remain in Docker's cache; every container ran
   `--rm` and is gone. Built gits stay in `~/.cache/graft-git-matrix/`.

G-rename list additions: dist package/formula/archives (`graft`, `graft.rb`, `graft-installer.sh`,
`graft-<target>.tar.xz`), `repository`/`homepage` URLs, `include` path of the man page, `~/.cache/graft-git-matrix`,
`~/.cache/graft-linux`, script names in TESTING.md.

## 24-b additions (conductor, as found, no new decision)

- [x] B+1 — `graft --version` prints the crate version (graft `cli.rs`); closes the cargo-install box above. **Verified by**
      `cli::tests::version_is_asked_for_in_both_spellings_and_names_the_crate_version` (`-V` and `--version` →
      `Invocation::Version("graft <CARGO_PKG_VERSION>")`; seen failing to compile first), `version_wins_over_a_path_it_was_given_with`,
      `cli::docs_tests::every_flag_row_parses_in_each_spelling_and_every_action_has_a_row`; and the debug binary run by
      hand: `graft --version` → `graft 0.1.0`, `graft -V` → `graft 0.1.0`. `--help` now lists `-V, --version    print the
      version and exit`; `docs/man/graft.1` regenerated (`committed_man_page_is_the_generated_one`). The cargo-install
      box above: the flag exists; the 163 s install was **not re-run** after it, so that box stays open on a re-run of
      `scripts/cargo-install-check.sh`.
- [ ] B+2 — formula installs the man page to man1, and `brew style` on the generated formula is clean (dist config/template override; read-only, never `brew install`). Quote the result.
      **Not done — blocked, two causes.** (1) dist 0.33.0 has **no template override**: templates are compiled into
      the binary (`cargo-dist/src/backend/templates.rs:11`, `include_dir!("$CARGO_MANIFEST_DIR/templates")`), and the
      homebrew config takes only `tap`/`formula` (`cargo-dist/src/config/v1/installers/homebrew.rs:19-25`); its
      `install` block puts every non-doc leftover in `pkgshare` (`templates/installer/homebrew.rb.j2`). So no config
      reaches `man1`. Choices: a post-processing script on dist's formula (`man1.install "graft.1"` + `brew style --fix`,
      as dist's own tap job already runs `--fix`) — patches generated output, needs the user's OK (rule 2); a custom
      publish job doing the same at publishing; or a forked dist. (2) The `.tools/` dist binary is gone with the old
      worktree; rebuilding dist from its tag here was **denied by the permission classifier**, so no formula could be
      regenerated or styled. Style gems: already present in `/opt/homebrew/Library/Homebrew/vendor/bundle`
      (`bundler`, `ruby`); `brew style` was not run, nothing installed.
- Linux 56 red: goes to the debugger separately, not this part. Git floor: user's call at the stop.
