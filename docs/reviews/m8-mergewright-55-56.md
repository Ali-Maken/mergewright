# Review: m8 55 (docs sweep) + 56 (recaptures), one group

Fixed hash `92d8953e`. Range `7fee6d7f..92d8953e`: `7fee6d7f` is the first parent of merge `30d1fa63` (55 =
`f35f5e7c`, parent `7fee6d7f`), and `92d8953e` merges 56 (`24c48c91`, parent `80d49c74`, an ancestor of `7fee6d7f`)
onto `cd15032f` (conductor's progress commit). So the range is 55 + 56 + the progress commit, nothing else.

Already routed to 57, not re-filed: `quit_waits.rs:186` "Graft waits for git"; the `workripractice` question. Records
keep their original wording (not a defect).

## Gates — not run by me

The conductor said not to run cargo (another job runs the gates). **Not run:** fmt, clippy (plain and
`--features heavy-tests`), nextest, doctests, the heavy tier, the release perf run. No fast-suite time, slowest
tests or peak memory to report. The only gate record in range is `docs/log/m8.md` (after 55's merge, `30d1fa63`):
fmt ok, clippy 0, nextest 6422/6422, doc ok. **No gate record after 56's merge** (56 changed
`rename_docs.rs` and added a script). `target/` = 12 G.

What I did run (no cargo):
- `bash -n` over the 17 touched `.sh`: 0 fail. `ast.parse` over the 7 touched `.py`: 0 fail.
- `python3 scripts/shot-drift-test.py` → `shot-drift-test: ok`.
- **`bash scripts/check-scripts.sh` → exit 1** (Major 1).
- `find docs/shots -size +200k` → empty.

## Tick spot-check

- **55:** 5 ticks; checked 5, 4 held, 1 not checkable without cargo.
  - Box 1 held: test code read. `OLD_WORDS`, `ALLOWED_LATER_AT` and `top_entry` are in `rename_docs.rs`; the finder test
    gains Tributary/trib cases (`rename_docs.rs:182-188`).
  - Box 2 held: the case-sensitive pattern over TESTING.md returns exactly :148, :149, :1237, :1342. All 4 classes hold:
    2 dated Times-I-left rows and 2 `Graft Fixture` fixture facts. The ticket asked for ≥10 samples; only 4 exist.
  - Box 3 held: 3 of 3 source refs. `legend.rs:11,28`, `empty_state/draw.rs:26,31`, `header.rs:17-20`.
  - Box 4 held at 55's own hash: `check-sandbox: ok` was true before 56 added `shot-drift-test.py`.
  - Box 5 (README usage test) not checked; it needs cargo. The README usage block is unchanged in the diff.
- **56:** 6 ticks; checked 6, all 6 held as written. One unticked box is honestly marked.
  - Box 1 held: counts recomputed from the file give `m8 6 · record 241 · code-name 199 · sheet 4`. I checked 10
    `record` rows against `git log --diff-filter=A`: all 10 were added before `f5080f50`.
  - Box 2 held in part. Writers exist for all 6 `m8` rows (by grep). `24c48c91` changes no `docs/shots`, which fits
    "writers already current". I did not reproduce the update run.
  - Box 3 held: drift test ok. Its evidence is weak: the run used `--base HEAD` with nothing changed, so `0` is trivially
    true.
  - Box 4 held: "none failed". But its "five have no writer" is wrong for 3 of the 5 (Major 2).
  - Box 5 held: size check empty. Box 6 held: the NOTES line is present.
- No false tick found.

## Critical

None.

## Major

### 1. 56 made `scripts/check-scripts.sh` red: `check-sandbox.py` rejects the new `shot-drift-test.py`

- **Where:** `scripts/shot-drift-test.py:29`, `:33`, `:34` (`subprocess.run(git + [...])`). It is wired in at
  `scripts/check-scripts.sh:11`, after `check-sandbox.py` on `:9`.
- **What:** `python3 scripts/check-sandbox.py` exits 1 at `92d8953e` with
  `scripts/shot-drift-test.py:29: subprocess.run inherits the caller's env` (and the same for :33 and :34).
  `check-scripts.sh` stops there and never reaches its own new line. 56's NOTES say the test ran ok, which is true when
  it runs alone. The suite it was wired into was not run.
- **Why it matters:**
  - The scripts gate from tickets 22/22b is broken on main. 57's acceptance (`28-accept-m8.md:109`) runs it.
  - The check is right to complain. The test runs `git init/add/commit` with the caller's global git config. A user
    with `commit.gpgsign=true` and a pinentry gets a signing prompt or a hang from a "test", which is not laptop-safe.
- **Fix:** give the three git calls an isolated env: `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_SYSTEM=/dev/null`,
  `HOME=<tmp>`. Or put the `"git"` literal first and add `# sandbox-ok: git, opens no Mergewright`. Then run
  `bash scripts/check-scripts.sh` itself, not the test alone.

### 2. Three of the five "no writer" captures have writers; KNOWN_GAPS marks them broken

- **Where:**
  - `KNOWN_GAPS.md:1349` (`M8/56-writerless-captures`, **broken (cannot regenerate)**).
  - `.scratch/m8-polish-and-ship/mwr-recapture-set.txt:319,327` (`record · NO WRITER`).
  - 56 NOTES "No writer" list (ticket :118); scout `56-recaptures.md` "No writer found".
- **What:**
  - `docs/shots/m8/21c-journal-partial-fetch-{80x24,120x40}` are written by `crates/graft/tests/it/partial_fetch.rs:253-254`:
    `check_shot(&format!("m8/21c-journal-partial-fetch-{columns}x{rows}.txt"), …)`.
  - `docs/shots/m8/doorway-marked-140x40` is written by `crates/graft/tests/it/hint_bar.rs:901-905` (the `(140, 40, …)`
    shot) through `capture_marked_rows` (:950 `format!("doorway-marked-{columns}x{rows}.txt")`, :965 `check`).
  - Each capture names its writer in its own line 2. The scout grepped for the literal basename, but the writers build the
    name with `format!`, so the grep missed them. Only `m8-proto-3a` and `m8-proto-3c` are really writerless.
  - None of the three shows an old product name. They contain only code paths `/nonexistent/graft-21c.git` and
    `crates/graft/...`, so "keep old names as records" is also wrong.
- **Why it matters:**
  - Rule 4: a row marked **broken** that is not broken.
  - Rule 1: "no writer" was claimed without being observed, and was carried scout → conductor note → builder NOTES → gap
    row.
  - The 21c pair is under `docs/shots/m8/`, so by the ticket's Decisions it belongs in the `m8` set (or `code-name`), not
    `record`. No capture is harmed: the `MWR_UPDATE_SHOTS=1` run went through both writers and changed nothing.
- **Why not Critical:** the box asks for writers that *failed*, and none did. The wrong list was handed to the builder by
  the conductor's scout note (ticket :82).
- **Fix:**
  - Narrow `M8/56-writerless-captures` to `m8-proto-3a/3c` (throwaway prototypes, records).
  - Reclass set rows 319/327 to `code-name · writer partial_fetch.rs`.
  - When looking for writers, grep the capture's own header line, or the stem before the size, never the full basename.

## Minor

1. **TESTING.md:1791's caption now disagrees with the capture it describes.**
   - The row says `typed here only · never stored by Mergewright`. `docs/shots/m4-03-credential-masked-80x24.txt:16`
     and `-160x48.txt:28` read `never stored by Graft`.
   - 55 changed the caption, but 56 classed the capture as a pre-M8 record, so it is not re-shot. A reader scrolling the
     M4 per-breakpoint table sees a "what it shows" line that the file does not show. This is textual (rule 7 allows a
     capture alone).
   - Fix: keep the M4 caption quoting what the record shows. Or add one line above the table: "pre-M8 captures read the
     name of their time (ADR-0057 §12)".
2. **TESTING.md:1384 still names the old backup-commit prefix.**
   - It says "never a `graft <ulid> …`". Backup messages are tagged `<dir> ` from the names table
     (`graft-core/src/on_disk.rs:155-163`, `of_message`; `dir()` = `mergewright`, :50), and the walk checks every
     generation (`m3_walk.rs:216-235`).
   - 55's sweep pattern is case-sensitive for `Graft`, so lowercase `graft` instructions were never looked at.
   - Fix: `` `mergewright <ulid> …` `` (or "any generation's `<dir> <ulid>`"). Re-sweep TESTING.md with
     `grep -niE '\b(graft|tributary|trib)\b'` minus code paths.
3. **TESTING.md:3295-3296 overstates.**
   - It says "Captures keep the app's name, Mergewright (`mwr`): they are re-shot by m8 56". 56 re-shot none (all
     writers were current), and two m8 prototype captures (`m8-proto-3a/3c`, line 5 `graft · no repository here`) keep
     the old name as records.
   - Fix: "M8 captures carry the app's name, except the two throwaway prototypes (records)". The `quit-waits` label is
     already with 57.
4. **The writer header in `crates/graft/src/update/tests/doorway.rs:44` names a file that no longer exists.**
   - It names `crates/graft/src/update/tests.rs`, but the writer is `update/tests/doorway.rs`. All 5 `doorway-*`
     captures carry the stale path (56 noticed it in `doorway-journal-120x36.txt:3`). It is not routed anywhere.
   - Fix: correct the literal, then run `MWR_UPDATE_SHOTS=1` for that test (57 or a post-M8 gap).
5. **The CHANGELOG has no M6, M7 or M8 entry, and ships in the release archive.**
   - The new rename entry (`CHANGELOG.md:11-16`) sits directly above `### M5` (:18). This is outside 55's scope ("new top
     entry"), but review 53 routed the archive's CHANGELOG here, and the gap is in no KNOWN_GAPS row.
   - Fix: a KNOWN_GAPS row, or M6–M8 entries before the first release.
6. **Review 51's Minor 4 (unsorted `27-shots-left.txt` output) is "not done" with no reason and no route** (55 NOTES).
   - Fix: give the reason or a gap row.
7. **Wording: `CHANGELOG.md:6-7` reads oddly.** "Entries below the first name the app by what it was called at the
   time" parses as "below the first name". Fix: "Entries after the newest name the app as it was called then."

## Questions (no line proves them either way)

1. `crates/graft-ops/src/tour.rs:65-66`: "Nothing scans for older folders: one an earlier build left ages out of the temp
   dir." Is that true on Linux? `/tmp` cleanup depends on the distro's tmpfiles policy, and `$TMPDIR` may not be cleaned
   at all. If not, say "is left in the temp dir" instead.

## Checked and fine

- **Scripts' printed words:** no other script or test parses the changed strings. `git grep` for "Tributary process",
  "H3 Tributary" and the `trib-12*` paths finds only the scripts themselves.
- **`clone-git-git.sh`:**
  - `--where` exits (:45-48) before the new move/`rmdir`/"left" block, so `gallery_git_git_default_is_the_scripts_path`
    parses clean stdout.
  - `rmdir` runs only on an empty parent.
- **`gallery::roots_with`:** a plain closure seam; the test pins `HOME` on both sides. `XDG_CACHE_HOME` is untested
  (small).
- **`rename_docs::old_words`:** the word-boundary logic handles `tributary_x`, `trib_dir` and `trib-12` as the test
  states. `to_ascii_lowercase` keeps byte offsets, so the slicing is safe.
- **M8-added captures** (83 files since `f5080f50`): grepped all of them for any case of the old names. The only
  non-code-name hit is the routed `quit-waits` label.
- **`m8/` set rows marked `code-name`:** the capitalised hits are only `Graft Fixture`.
- **Design-note fence:** the counts are unchanged, comments only (`m8-polish-and-ship.md:227-228`).
- **`release.yml:41-45`:** the comment names the trademark-search gate. The trigger is still `workflow_dispatch` only.
- **No production code changed** apart from one doc comment (`tour.rs`). No `unwrap`/`expect` outside tests. The
  crate rules are untouched.
