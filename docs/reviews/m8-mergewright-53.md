# Review: m8-polish-and-ship 53, Mergewright release and formula

Reviewed: `fb5f61fe` + `6d7477ed` (merged `99136fd4`) against `4dfc2abd`. Ticket
`.scratch/m8-polish-and-ship/issues/53-mergewright-release-and-formula.md`; ADR-0057 Amendment (2026-10-11) §9, §11, §12.
Reviewer: read-only. Per the dispatch, **no cargo, dist or brew was run**. Gates are the conductor's (`659c5424`
"gates green"), not observed here. Skill: `code-review` (both axes applied in one context; the diff is 15 files).

**Counts: Critical 0 · Major 1 · Minor 9.**

## Acceptance spot-check: 7 checked, 7 held (on the text; no run reproduced)

| Box | What I checked | Held? |
|---|---|---|
| plan.txt names | `git show 6d7477ed:docs/release/plan.txt`: `:3` `  mergewright 0.1.0`, `:6` installer, `:8-10` archive / `[bin] mwr` / `[misc] … mwr.1`; `grep -cE 'graft\|trib'` → 0; `m8-accept.sh:86`'s grep → 4 | yes |
| allow-dirty | `Cargo.toml` has `allow-dirty = ["ci"]`; plan.txt dated 2026-10-12 shows that `dist plan` ran. I did not re-run the removed-line experiment | yes (config); the run itself is not reproduced |
| build.txt archive | `:12-15` the four entries, `:16` `mwr 0.1.0` | yes |
| brew style + man1 | `:19` `no offenses detected`, `:24` `man1.install "mwr.1"`; the stand-in caveat is stated in the tick | yes |
| tests + mutations | Both tests exist. The cited panic lines `:75` and `:145` are the right asserts **at `fb5f61fe`**. At HEAD they are `:81` and `:153-154` (see Minor 8). I did not reproduce the mutations | yes |
| cargo-install.txt | `install: ok in 144s`, `installed: mwr`, `mwr --version: mwr 0.1.0` | yes |
| no push / tag | `git tag \| wc -l` → 0 now; scripts grep: no `git push`, `git tag`, `gh`, `brew install`, `brew tap`, `curl` | yes |

No false tick.

## Publish / old-name sweep (as asked)

- **Nothing in this diff can publish.** `publish-jobs = []`, no `tap`, no `homebrew` installer (`Cargo.toml` dist block).
  `release-local.sh` runs `dist plan`, `dist build`, `brew style`, `shellcheck`. `cargo-install-check.sh` clones `.`
  locally.
- **The formula installs `mwr` and the man page:** `packaging/homebrew/mergewright.rb.in:37-38` has
  `bin.install "mwr"` and `man1.install "mwr.1"`. Its test (`:42`) runs `#{bin}/mwr --version`. Class `Mergewright`;
  the `desc` has no article.
- **Old names:** none in `packaging/`, `scripts/formula.py`, plan.txt or build.txt. The scripts still say
  `crates/graft`, a codename path that §9 keeps. Two things outside this diff would still ship or arm old wording:
  Major 1 (release.yml) and Minor 9 (archive README and CHANGELOG).

## Builder's raised gaps: judged

- **Placeholder checksums (NOTES 1).** Sound. A host build can't make a four-checksum formula. The stand-in copy is
  labelled (`build.txt:18`), gitignored, and fails safe: brew would reject a zero checksum. It is still
  **unverified**: we have never seen a formula with four real checksums, and `formula.py` has never read a CI-merged
  manifest. So M8/24-brew-style closes only for style and the man1 location. A new **unverified** row is needed
  (Minor 4). The stand-in file also needs a marker (Minor 3).
- **Installer env names `MERGEWRIGHT_*` (NOTES 3).** Accept, not a defect. They are install-time variables that dist
  derives from the package name. They name the current product, not an old one. They also stay out of the app's
  runtime `MWR_*` namespace, which is arguably cleaner than sharing it. Whether dist 0.33 can override them is
  unchecked. That doesn't matter unless someone wants them to be `MWR_*`, and nobody has asked. Record it as
  decided, e.g. one line in `docs/release/` or the ticket. Don't open a gap.
- **`%3Cowner%3E` in URLs (NOTES 2).** Fine. The owner is a placeholder until publishing.

## Critical

None.

## Major

### 1. release.yml's guard comment now names a condition M8 itself meets

- **Where:** `.github/workflows/release.yml:41-44`: "Disarmed: armed by the rename ticket (27/G-rename) … after any
  `dist generate` put this block back **until the rename has landed**."
- **What's wrong:** the rename lands in M8 (tickets 48–57). ADR-0057 §1 and the new `Cargo.toml` comment gate
  publishing on the **trademark search**, not on the rename. Once ticket 55 merges, someone following this comment
  after a `dist generate` would leave the tag-push trigger armed.
- **Why it matters:** this comment is the only guard on the CI publish path. The workflow still has
  `gh release create` at `:280`, and `publishing: ${{ !github.event.pull_request }}` at `:56`, which is true on
  `workflow_dispatch` and on a tag push.
- **Ownership:** the builder flagged it (NOTES 5) and left it out of scope. Ticket 55's scope doesn't list
  release.yml, so nobody owns it.
- **Fix:** one comment edit: "until the trademark search clears (ADR-0057 §1)", dropping "27/G-rename". Route it to
  55 or 57.

## Minor

1. **Rule 3: dist's error message is lost on failure.**
   - **Where:** `scripts/release-local.sh:38` sends `dist plan --output-format=json` stderr to `/dev/null`. `:37`
     and `:48` send dist's output to `$logs/…`, and the `EXIT` trap at `:34` deletes `$logs`.
   - **What happens:** if dist fails, `set -e` exits non-zero but the message is gone. The status survives; the
     cause does not.
   - **Fix:** on a failing dist step, `cat` its log to stderr before exiting. Drop the `2>/dev/null`.
2. **A mid-run failure truncates the committed records.**
   - **Where:** `release-local.sh:39-45` and `:58-117` redirect brace groups straight onto `docs/release/plan.txt` and
     `build.txt`. The script uses `set -eu` without `pipefail` (`:28`).
   - **What happens:** a failure at `:65` or `:94` leaves a half-written build.txt that `m8-accept.sh:87-88` then
     quotes. `tar tf … | sed` at `:63` can also fail unseen.
   - **Fix:** write to `$logs/…`, `mv` into place at the end, and add `set -o pipefail`.
3. **The stand-in formula has no marker in the file.**
   - **Where:** the copy with three zero checksums is written to `.scratch/m8-polish-and-ship/mergewright.rb`
     (`release-local.sh:55`, `:94`). The header (`:21`) calls that path "the formula".
   - **Why it matters:** nothing inside the file says three of its checksums are fake.
   - **Fix:** name it `mergewright.style-only.rb`, or prepend a comment line saying so.
4. **The unverified checksum path needs a gap row (rule 4).**
   - NOTES 6 asks the conductor to close M8/24-brew-style. Close it for style and the man1 location only.
   - Open a row marked **unverified**: "formula with four real sha256 from a four-target manifest; `formula.py`'s
     checksum source (`checksums.sha256` in the manifest vs the `.sha256` file beside it, `formula.py:66-75`) is
     untried on a CI manifest."
5. **The URL fill has no committed evidence and no test.**
   - **Where:** `release-local.sh:116` quotes only the install block, so `formula.py:95`'s URL composition
     (`artifact_base_url + artifact_download_path + "/" + archive`) shows up nowhere in a committed file.
     `formula.py` has no automated test; its two paths were exercised once by the script.
   - **Second gap:** `release_names.rs:54-59` hardcodes the four targets instead of reading
     `[workspace.metadata.dist] targets`. A fifth dist target added to `Cargo.toml` would be left out of the formula
     silently, and the test would still pass.
   - **Fix:** also quote the rendered `url` lines in build.txt, and have the test read its targets from `Cargo.toml`.
6. **The plan.txt parse in the test is fragile.**
   - **Where:** `release_names.rs:132-141` takes the first plan.txt line with exactly two leading spaces as the
     package. `release-local.sh:43` keeps one WARN line plus one more (`grep -A1`), and that extra line comes before
     `announcing`. The old plan.txt had one: `  consider adding "homebrew" …`.
   - **What happens:** any future dist warning makes the test fail with "dist announces the table's package:
     `consider` ≠ `mergewright`". It fails loudly, not falsely, but the message misleads.
   - **Fix:** parse only after the `announcing` line.
7. **Readability (6.2): 17 lines of inline Python in a shell heredoc.**
   - **Where:** `release-local.sh:77-93`. It is the manifest-merging logic, and it is untestable where it sits.
   - **Fix:** move it into `formula.py` as a flag (e.g. `--stand-in-missing <build-manifest>`) or into a sibling
     script with a docstring.
8. **`fb5f61fe` was not fmt-clean, and the ticket's line cites drifted.**
   - **What happened:** `6d7477ed` reformats `release_names.rs` (`:49-52`, `:65-68`, `:117-119`, `:180-181`), which is
     rustfmt's 60-column `fn_call_width` split. Its commit message ("cargo-install re-run; ticket done") doesn't say
     so. `fb5f61fe` alone would fail `cargo fmt --check`.
   - **Cite drift:** the acceptance cites `:75` and `:145` from that commit. They are `:81` and `:153` at HEAD.
   - **Fix:** run the gates before every commit, and give line cites at the final hash.
9. **The archive ships README and CHANGELOG with the old names.**
   - **Where:** `plan.txt:10` lists `[misc] CHANGELOG.md, README.md, mwr.1`. Today `README.md:1-3` says
     `# Tributary` / `trib`, and `CHANGELOG.md:6` says "Graft".
   - **Ownership:** ticket 55 owns the fix. Then `docs/release/{plan,build}.txt` record an archive built before the
     sweep.
   - **Fix:** route to 57: re-run `release-local.sh` after 55 merges, and check `tar -xOf … README.md | head -1`.

## Other checks

- **Rust outside tests:** none changed, so there is no `unwrap` or `expect` exposure. The test file carries
  `#![allow(clippy::expect_used)]` with a reason.
- **Not applicable here:** terminal-escape sanitising and git work on the UI thread.
- **Architecture:** no crate dependencies changed. The test uses `graft_core::on_disk::Generation`, which is allowed
  in graft's `it`. No MAP.md change is needed.
- **Feature pipeline:** the design note has Prior art (`m8-polish-and-ship.md:39`, row `:73` citing
  `docs/research/install-docs.md`) and Journeys (`:116`). J16 Install is 2 steps (`brew install …/mergewright`, then
  `mwr`), and README Install keeps it at 2.
  - The note's row `:73` still says "tap formula" is taken from dist, and `:135` says `tributary`/`trib`. Design notes
    are records under §12; ADR-0057 §11 holds the newer decision.
- **Laptop-safety:** the test reads three small files, with no process or network.
