# Mergewright 6: what dist builds is named mergewright, and the formula is ours

Status: draft
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
- **dist:** 0.33.0, built from its `v0.33.0` tag into `.tools/` (gitignored), as ticket 24 did. If the build or install
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
- [ ] `docs/release/plan.txt` lists `mergewright-{aarch64,x86_64}-apple-darwin.tar.xz`,
      `mergewright-{aarch64,x86_64}-unknown-linux-gnu.tar.xz`, each `[bin] mwr`, `[misc] … mwr.1`, and
      `mergewright-installer.sh`; no `graft` or `trib` artifact name (lines quoted). Run: `scripts/release-local.sh`.
- [ ] `dist plan` accepts the hand-edited `release.yml` (`allow-dirty`), quoted; closes M8/31.
- [ ] `docs/release/build.txt`: `tar tf` shows `mergewright-<host>/{mwr,mwr.1,README.md,CHANGELOG.md}`; the archive's
      binary prints `mwr 0.1.0`.
- [ ] `brew style` on the rendered formula: `1 file inspected, no offenses detected` (quoted in `build.txt`);
      the formula's install block has `man1.install "mwr.1"` (quoted). Closes M8/24-brew-style.
- [ ] `release_names::formula_template_names_the_table`: the template names `Generation::CURRENT.dir()` (class,
      archive stem), `.command()` (bin, man page); `release_names::readme_install_lines_match_the_plan`: README's
      Install lines name the artifacts `plan.txt` lists.
- [ ] `docs/release/cargo-install.txt`: `install ok in <n>s`, `mwr --version: mwr 0.1.0`.
- [ ] No push, no tag: `git tag` and `git status -sb` before and after, quoted.

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
