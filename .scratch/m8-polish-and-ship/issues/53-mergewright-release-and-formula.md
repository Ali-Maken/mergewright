# Mergewright 6: what dist builds is named mergewright, and the formula is ours

Status: done
Blocked by: 49, 51
Build with: **Opus**. Packaging judgement (dist config, a formula template, a hand-edited workflow) where a wrong call
would ship another product's name; small code.
Brownfield: ticket 24's NOTES and `scouts/24-release.md` serve as the scout. Conductor confirms.
Needs: `mergewright-package-and-command` merged (package `mergewright`, bin `mwr`, `docs/man/mwr.1`).
Beside: `mergewright-git-messages-and-journal-column` (disjoint files; it runs cargo, `dist build` is a release build).

## Read first
- ADR-0057 §1 (nothing published) and Amendment (2026-10-11) §9, §11
- `issues/24-release-dist-and-matrix.md`: Decisions, NOTES items 2 and 7, "24-b additions" B+2
- `issues/27a-rename-command-and-app-name.md` NOTES "Open — plan.txt"
- KNOWN_GAPS rows `M8/24-brew-style`, `M8/27a-plan-txt`, `M8/27c-dist-artifact-names`, `M8/28-release-docs-stale`,
  `M8/31-dist-plan-hand-edited-workflow`
- `docs/research/install-docs.md` §2–§3 (dist, tap); README's Install section only

## Decisions
- **dist** (user allowed the .tools/ build 2026-10-11): 0.33.0, built from its `v0.33.0` tag into `.tools/` (gitignored), as ticket 24 did. If the build or install
  is refused (24 B+2 hit the permission classifier), **stop: `blocked: dist not installable`**, with the refusal's
  first line. Installing it is the user's call; do not work around it.
- **Dist metadata:** homebrew installer off (`installers = ["shell"]`); `allow-dirty = ["ci"]` (§11);
  `repository`/`homepage` → `…/<owner>/mergewright`. `publish-jobs = []` stays.
- **Formula (§11):** `packaging/homebrew/mergewright.rb.in` + `scripts/formula.py <dist-manifest.json> <out.rb>`.
  Class `Mergewright`, `desc` without an article, `homepage`, per-target `url`/`sha256` from the manifest
  (macOS arm64/x86_64, Linux arm64/x86_64), `bin.install "mwr"`, `man1.install "mwr.1"`, `test do` running
  `#{bin}/mwr --version`. Python with no third-party imports; run with `python3 -I`.
- **`scripts/release-local.sh`**: `dist plan` → `docs/release/plan.txt`; `dist build` (host) → `docs/release/build.txt`
  with `tar tf` of the host archive; `formula.py` on the build's manifest → `.scratch/…/mergewright.rb` (not committed;
  the template is); `brew style` on it (read-only; **never `brew install`**); shellcheck on the installer. Its words and
  extract paths say `mergewright`/`mwr`.
- **Stale `docs/release/`**: `plan.txt`, `build.txt`, `cargo-install.txt` regenerated (`scripts/cargo-install-check.sh`
  re-run: `cargo install --locked --path crates/graft` into a temp `CARGO_HOME`, `mwr --version`). `linux.txt`,
  `git-matrix.txt`, `linux-git-2.39.txt` are dated run records: kept, and NOTES says so.
- **README Install section** (this ticket owns it; the rest of README is `mergewright-docs-sweep`'s):
  `brew install <owner>/tap/mergewright`, the curl line naming `mergewright-installer.sh`, `cargo install --locked
  --path crates/graft` installing `mwr`. Lines match `plan.txt` exactly (test below).
- **No push, no tag, no release, no tap repository** (§1).

## Scope
In: workspace and `crates/graft` dist metadata; `packaging/homebrew/`; `scripts/formula.py`,
`scripts/release-local.sh`, `scripts/cargo-install-check.sh` words; `docs/release/{plan,build,cargo-install}.txt`;
README Install section; `scripts/m8-accept.sh` step 4's grep if its quoted names change. Out: CI publishing, notarising,
homebrew-core, the Linux and git-matrix re-runs.

## Signatures
None in Rust. Script interface:
`scripts/formula.py <dist-manifest.json> <out.rb>` → exit 0 and the formula; exit 2 naming the missing target.
Test only: `crates/graft/tests/it/release_names.rs`.

## Acceptance (each box: the evidence named at tick time)
- [x] `docs/release/plan.txt` lists `mergewright-{aarch64,x86_64}-apple-darwin.tar.xz`,
      `mergewright-{aarch64,x86_64}-unknown-linux-gnu.tar.xz`, each `[bin] mwr`, `[misc] … mwr.1`, and
      `mergewright-installer.sh`; no `graft` or `trib` artifact name (lines quoted). Run: `scripts/release-local.sh`.
      **Verified by** `DIST=.tools/bin/dist scripts/release-local.sh` (2026-10-12): `plan.txt:3` `  mergewright 0.1.0`,
      `:6` `    mergewright-installer.sh`, `:8-10` `    mergewright-aarch64-apple-darwin.tar.xz` / `      [bin] mwr` /
      `      [misc] CHANGELOG.md, README.md, mwr.1`, the same three lines for the other three targets (4 archive lines by
      `m8-accept.sh:86`'s grep). `grep -cE 'graft|trib' docs/release/plan.txt` → `0`. No formula in the plan (homebrew
      installer off).
- [x] `dist plan` accepts the hand-edited `release.yml` (`allow-dirty`), quoted; closes M8/31.
      **Verified by** `dist plan` with `allow-dirty = ["ci"]` → the plan above. With that line removed (temporarily,
      restored): `× …/.github/workflows/release.yml has out of date contents and needs to be regenerated:`.
- [x] `docs/release/build.txt`: `tar tf` shows `mergewright-<host>/{mwr,mwr.1,README.md,CHANGELOG.md}`; the archive's
      binary prints `mwr 0.1.0`. **Verified by** `build.txt:12-15` (`mergewright-aarch64-apple-darwin/mwr.1`,
      `/README.md`, `/mwr`, `/CHANGELOG.md`) and `:16` `the archive's binary, --version: mwr 0.1.0`.
- [x] `brew style` on the rendered formula: `1 file inspected, no offenses detected` (quoted in `build.txt`);
      the formula's install block has `man1.install "mwr.1"` (quoted). Closes M8/24-brew-style.
      **Verified by** `build.txt:19` `brew style mergewright.rb: ok (1 file inspected, no offenses detected)`, `:24`
      `        man1.install "mwr.1"`. **Caveat, NOTES 1: three of four sha256 in the styled copy are stand-ins.**
- [x] `release_names::formula_template_names_the_table`: the template names `Generation::CURRENT.dir()` (class,
      archive stem), `.command()` (bin, man page); `release_names::readme_install_lines_match_the_plan`: README's
      Install lines name the artifacts `plan.txt` lists. **Verified by** both tests passing (nextest), and both seen
      failing on a mutation (README curl → `trib-installer.sh`: panics at `release_names.rs:145`; template
      `man1.install` → `pkgshare.install`: panics at `:75`), then restored.
- [x] `docs/release/cargo-install.txt`: `install ok in <n>s`, `mwr --version: mwr 0.1.0`. **Verified by**
      `scripts/cargo-install-check.sh` at commit fb5f61fe: `install: ok in 144s`, `installed: mwr`,
      `mwr --version: mwr 0.1.0` (colon kept: `m8-accept.sh:89` greps `^install:`).
- [x] No push, no tag: `git tag` and `git status -sb` before and after, quoted. **Verified by** before:
      `git tag | wc -l` → `0`, `## worktree-agent-ac10eab5695fabbdf` (clean, no upstream); after: `0`, same line. No
      `git push`, no `gh`, no `brew install`, no `brew tap` run.

## Touches
- **Callers:** `scripts/m8-accept.sh` step 4 (quotes `build.txt`); TESTING.md install rows (words: docs sweep).
- **Screens / keymap / journal / undo:** none.
- **Captures:** none.
- **Docs:** README Install; `docs/release/`; `.gitignore` (`.tools/`, already); TESTING.md J16/J17 rows read `brew
  install <owner>/tap/mergewright`, `man mwr` (handed to `mergewright-docs-sweep`).
- **Crates:** `crates/graft/Cargo.toml` metadata only; test file in graft's `it`.
- **His machine:** `brew style` may touch Homebrew's vendor bundle (24 NOTES 7); nothing installed.
- Each is checked by the named grep, run or test in the report.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`,
workspace-wide. dist and brew runs write to `docs/release/`; read summary lines only.

## Size
One context, ~45k tokens (dist build time excluded).

## NOTES (builder, 2026-10-12)

No compaction. Scout: none — ticket 24's NOTES and `scouts/24-release.md` serve as the scout (ticket line 7); the
three scripts touched were read whole. dist 0.33.0 built from its `v0.33.0` git tag into `.tools/` (gitignored), as
the user allowed 2026-10-11; not refused.

1. **A host build cannot render the full formula (spec gap).** `dist build --target <host>` builds one archive of
   four, so `formula.py` on its manifest exits 2 (`build.txt:17`: `no archive for target x86_64-apple-darwin …`), as
   the interface says. To lint the template's shape, `release-local.sh` renders a second copy from `dist plan`'s
   manifest with the host archive's real sha256 and **64 zeros for the three not built here**, labelled at
   `build.txt:18`. That copy is for `brew style` only, never shipped. Style: verified. A formula with four real
   checksums: **unverified** until a four-target build exists (CI, at publishing).
2. **URLs carry `%3Cowner%3E`.** dist URL-encodes the `<owner>` placeholder in its manifest; `formula.py` decodes it
   for `homepage` only. A real owner at publishing is plain.
3. **Question, not a defect: the shell installer's env names are dist's**, derived from the package:
   `MERGEWRIGHT_INSTALL_DIR`, `MERGEWRIGHT_NO_MODIFY_PATH`, … (build manifest `releases[0].env`). The table's `MWR_*`
   is the app's environment; whether the installer's should match is a naming call. Not checked whether dist 0.33
   can override them.
4. **Kept as dated run records:** `docs/release/linux.txt`, `git-matrix.txt`, `linux-git-2.39.txt` — not re-run
   (out of scope); they name the binary of their date.
5. `.github/workflows/release.yml`'s "Disarmed" comment still cites "the rename ticket (27/G-rename)"; not edited
   (not in scope). With `publish-jobs = []` it has no homebrew job either way.
6. Gaps this closes (conductor to move; builders don't edit KNOWN_GAPS): M8/31-dist-plan-hand-edited-workflow,
   M8/24-brew-style (style; see 1), M8/27a-plan-txt, M8/27c-dist-artifact-names, M8/28-release-docs-stale.
7. Review 49 Minor 3: both scripts fixed. `release-local.sh` takes the package from dist's manifest and the command
   from the archive listing, not literals; `cargo-install-check.sh:43` runs `mwr`.
8. Side effects: `brew style` ran on this Mac (read-only; gems already present per 24 NOTES 7). `target/distrib/`
   holds the host build.

Touches: `scripts/m8-accept.sh:86` grep `graft-` → `mergewright-` (step 4 quotes `plan.txt`; the new grep counts 4);
its `:87-89` greps (`\.1$`, `^brew style`, `^install:`) still match `build.txt:12`, `:19`, `cargo-install.txt:3`;
`.gitignore` adds the rendered formula path; README Install only (lines 33-53); TESTING.md J16/J17 and README's other
man-page lines left to `mergewright-docs-sweep`; no Rust source and no public item, so no MAP.md change
(`grep -rn -e installer -e plan.txt -e release-local crates` → only the new test and a Cargo.toml comment).
