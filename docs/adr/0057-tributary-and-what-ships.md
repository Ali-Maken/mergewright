# ADR-0057: Tributary, and what ships in v1

Status: accepted (user, 2026-10-02: "go ahead with your recommendation" — name, notarising, items past v1).
**Name superseded 2026-10-11: the product is Mergewright, command `mwr`** (see "Amendment (2026-10-11)" below).
**Open item: a person must run a trademark search (USPTO classes 9 and 42) before anything is published.**
Amends ADR-0010 (the on-disk names), PROMPT.md §5 ("image previews in the pixel tier").

## Decision

1. **The name is Tributary; the command is `trib`.** From `docs/naming-research.md`: the graph is a river system, and
   `trib` was free on every registry checked. Until the trademark search is done, **nothing is published,
   registered, tagged or pushed under either name** — not crates.io, not a tap repository, not a release. `dist`
   runs `plan` and `build` locally only.
2. **Every on-disk and environment name moves to the new name, once, at launch.** `refs/graft/**` →
   `refs/tributary/**`, `<common_dir>/graft/` → `<common_dir>/tributary/`, the config dir, `GRAFT_*` → `TRIB_*`.
   The first launch of the renamed binary migrates: journaled, idempotent, the old copy kept until the new one reads
   back the same. A migration that cannot finish leaves the old names in force and says so. Runs alone, last before
   the acceptance ticket.
3. **No Apple notarisation.** `brew install` and the curl installer do not quarantine what they fetch; only a binary
   downloaded in a browser would warn. Not worth $99/year for one user. dist codesigns ad hoc as it does by default.
4. **Carried past v1** (not in his daily use; each is a screen of its own): pixel-tier image diffs (KNOWN_GAPS row
   371), Map view mouse and remote rows (372, 374), undo-backup pruning (205).

## Why

- "Graft" is another Git GUI's name (CLAUDE.md). His live journal and backup refs must survive the switch, so the
  rename is a migration, not a find-and-replace.
- Keeping internal `graft` names forever would leave another product's name in his repositories' refs.

## Consequences

- The rename group is the only M8 group that touches every crate; nothing builds beside it.
- Old backups stay readable: the migration moves refs with `git update-ref`, journaled like any write (ADR-0009's
  snapshot rule does not apply: no working-tree content is touched).
- KNOWN_GAPS rows 371, 372, 374 and 205 re-expire to "after v1".

## Amendment (2026-10-09, ticket 26)

How §2's migration works. The user approved the migration itself on 2026-10-09: copy, read back, then delete the old
names; nothing kept afterwards.

1. **One migration per repository, covering every worktree.** One pass moves the refs, `<common_dir>/graft/`, every
   `<common_dir>/worktrees/*/graft/` and `<git_dir>/graft/` (duplicate paths counted once). Each tree is copied whole,
   every file is compared byte for byte, and journals are also read back through the real reader.
   *Why:* refs are shared, so moving them one worktree at a time would leave a worktree's journal naming refs that
   have already moved. A copy of the whole tree means no file is deleted without first being read back.
   *Rejected:* a separate migration per worktree; a fixed list of files to move.
2. **A marker with two states: `switched`, then `clean`.** It lives at `<common_dir>/tributary/migrated`.
   - Before the switch: new refs are created and read back, the trees are copied and read back.
   - Writing `switched` is the moment the new names come into force.
   - After the switch: the `Migrate` entry is journaled, old refs are deleted with their old oids as expected values,
     the old trees are removed, and the marker becomes `clean`.

   A failure before the switch leaves the old names in force and deletes nothing. A failure after it keeps the new
   names; the next launch sees `switched` and finishes the clean-up. When the marker reads `clean`, opening a
   repository costs one file read and starts no `git` process.
   *Why:* with one marker state, a clean-up that failed part-way would either strand old refs for good or force
   every launch to check for them.
3. **A lock stops the migration while another copy of the app is open on the repository.** The lock file is
   `<common_dir>/tributary/lock`, an advisory OS lock (`File::lock`), not a polling loop. A process running under
   the old names holds it shared for its whole life. The migration needs it exclusive and only tries once; if
   another process holds it, this run stays on the old names and says `another trib is open on this repository`.
   *Why:* otherwise a second window still on the old names would keep writing them after the switch, and the marker
   would hide those writes from every later launch.
4. **`OnDiskNames` (`New` / `Old`) lives in `graft-core`.** It sits on `SharedRepo`/`RepoHandle`, which set it at
   open from whether the marker exists. Every reader and writer of the ref prefix, the dir name or the
   backup-commit trailer takes it from there. The check for "is this one of our refs" matches both prefixes.
   *Why:* `graft-git` needs the names (ref filter, commit-graph offer) and cannot depend on `graft-ops`. Every
   caller that opens a repository, tests and headless paths included, gets the same answer for free.

**Consequences:**
- Backup commits stay readable forever: their trailer is read as `Graft-Link:` or `Tributary-Link:`, and only the
  new spelling is written.
- Journal lines are never rewritten; ref names are mapped to the new prefix when read, only once the new names are
  in force.
- The crash-report dir and the backup author identity are renamed without moving anything (ticket 27).

## Amendment (2026-10-11): the name is Mergewright, the command `mwr`

User, 2026-10-11: rejected Tributary ("dont like tributory"), chose Scion, then the same day: "lets keep it
mergewright". Final. §1's name and §2's `tributary` / `TRIB_` spellings are replaced; everything else in §1–§4 and
the 2026-10-09 amendment stands. Checked by the conductor 2026-10-11: `mergewright` and `mwr` free on crates.io and
Homebrew, no command by either name on his Mac, no tool by either name found on the web; `mwr` is taken on npm
(irrelevant). Accepted by him: the spelling (silent w, gh) and the echo of Playwright. **§1's rule stands: nothing is
published, registered, tagged or pushed under Mergewright until the trademark search is done.**

### 1. The spellings

| What | Spelling |
|---|---|
| Product (prose, empty state, journal, status words) | `Mergewright` |
| Command: `[[bin]]`, usage, `--version`, stderr prefix, man page | `mwr`; `docs/man/mwr.1` |
| Shipping package (dist archive, formula, installer, `cargo install`) | `mergewright` (its `[lib]` stays `graft`) |
| Refs | `refs/mergewright/**` |
| Dirs | `<common_dir>/mergewright/`, `<git_dir>/mergewright/`, `<config_root>/mergewright/`, crash `<state_home>/mergewright/`; config lock `<config_root>/mergewright.lock` |
| Backup trailer | `Mergewright-Link:` written; `Graft-Link:`, `Tributary-Link:` read forever |
| Backup identity | `mergewright` / `mergewright@backup` |
| Tag in messages written into git (stash, backup commits, index, absorb, forecast) | `mergewright` written; every earlier tag read |
| Environment | `MWR_*`; `GRAFT_*` and `TRIB_*` are not read (nothing shipped under them) |
| Crates, crate dirs, Rust paths, temp-dir prefixes | stay `graft*`: the codename (decision 9) |

- **Dirs and refs say `mergewright`, not `mwr`.** Someone reading `ls .git`, `~/.config` or `git for-each-ref` should
  see whose files they are; nobody types these names. Cost: longer ref names nobody types.
- **Environment `MWR_*`, not `MERGEWRIGHT_*`.** Matches the command he types (`MWR_FRAME_TRACE=1 mwr`), as `TRIB_` did
  for `trib`. The long prefix adds 7 characters to every dev and test variable for no reader's gain.

### 2. One table holds every name the app has shipped under

`graft-core::on_disk` holds one row per **generation**: Graft, Tributary, Mergewright, oldest first, plus `CURRENT`.
Each row spells the dir, ref prefix, trailer, git-message tag, title, command and env prefix. Nothing else in the
code spells a generation, with three exceptions, each held by a test:
- `Cargo.toml`'s bin and package names: pinned equal to the table's command and dir.
- Env names: literals; a grep test fails on any earlier generation's prefix.
- Prose words: literals; a grep test fails on any earlier generation's title or command.

A later rename is one new row and a flip of `CURRENT`. The migration code reads the table and does not change.
*Rejected:* a second hard-coded pair (`New`/`Old` → Tributary/Mergewright). It would strand repositories still on
Graft, and it would be rewritten at the next rename.

### 3. Which names are in force

Every generation after Graft has a marker at `<common_dir>/<its dir>/migrated`. The names in force are the newest
generation whose marker exists, up to `CURRENT`. With no marker, Graft is in force: the first generation needs none.
Checking costs at most two stats at open and starts no process.

### 4. One move, from whatever is in force straight to `CURRENT`

- **A repository still on Graft names (never opened by a Tributary build) goes straight to Mergewright.** It gets one
  move and one `Migrate` entry. There is no Tributary step, and nothing named `tributary` is created.
- A repository on Tributary names (marker `switched` or `clean`) moves from Tributary to Mergewright. Its Graft
  leftovers are strays (decision 5).
- The order, the read-back, the pre-switch mirror (ticket 26r M1) and the two marker states are unchanged. "Old"
  means the generation in force, and "New" means `CURRENT`.

### 5. Every other earlier generation is a stray source

- After the switch, and on every `clean` open, the stray pass covers **every** earlier generation, not one. The
  `clean` check is stat plus `read_dir` per generation, and still starts no process.
- The byte rule is the same: equal → delete; no new copy → carry forward; differs → keep aside. Journals compare by
  prefix.
- A file kept aside keeps its path relative to the common dir, so it carries its generation's dir.
- A ref kept aside goes to `refs/mergewright/older-version/<n>/<generation dir>/<rest>`, so Graft and Tributary refs
  with the same rest never collide.

**One exception: a failed earlier move's scratch.** This is a generation **newer than the one in force with no marker**,
for example `refs/tributary/**` or `.git/tributary/` in a repository still on Graft. It is removed after the switch
without being compared. *Why:* by 26r M1 no run ever read it as his, and its source is still in force and is
copied. Comparing it would only produce a false "an older version wrote" warning.

### 6. Locks

- A run that stays on earlier names (`KeptOld`, `Busy`) holds `CURRENT`'s lock shared for its life.
- The move takes **every** generation's lock file that exists, exclusive, trying once each. If any is held, the result
  is `Busy` and the status row says `another Mergewright window is open on this repository`.
- So a Tributary build left open on Graft names blocks the move: it holds `tributary/lock` shared.
- A build of any generation running on the names **in force** holds nothing. What it writes after the switch is
  caught by the stray pass on the next open. This is 26r's residual, unchanged.

### 7. The journal

- The `Migrate` row names the generation the move went **to**: `rename to Mergewright (N refs, M files)`. An entry
  written by the Tributary build still reads `rename to Tributary (…)`.
- Journal lines are never rewritten. On read, ref names from every generation older than the one in force are mapped
  to the in-force prefix.

### 8. Config folder and crash reports

- **Config folder.** The newest earlier folder present is moved: copy, read back, delete. Every older folder goes
  through 26r's both-present byte rule against the new folder.
- **Crash reports.** They go to the new dir only. Old crash dirs are his files and are left untouched.

### 9. Crate names stay `graft-*`, for good; only the shipping package is renamed

- **What stays.** "Graft" is the codename (CLAUDE.md), so crate names, `crates/graft*` dirs, Rust paths and test temp
  prefixes keep it.
- **Why.**
  - No crate is ever published (§1).
  - Every `crates/graft…:line` citation in the records, gaps and reviews stays valid.
  - A crate name is not the product's name.
  - A future rename never touches them.
- **The one exception: the package that ships the binary** (`crates/graft`, package `graft`) becomes package
  `mergewright`, with `[lib] name = "graft"` kept so `use graft::…` does not change. *Why:* dist names the archive,
  the formula and the installer after the package. `cargo install` prints it too. So that package name is user-facing.
- **Cost, accepted.** Every live `-p graft` (scripts, TESTING.md, README, crate comments and examples' run lines,
  `.claude/agents/reviewer.md:20`) becomes `-p mergewright` in the same commit as the rename, or it stops working.
  Records keep `-p graft`. `reviewer.md` is the user's file: listed, and until he says yes the conductor gives the
  reviewer the new command in the dispatch.
- **Rejected:**
  - Renaming every crate: it breaks thousands of citations and buys nothing unpublished.
  - Depending on a dist naming override: dist 0.33 takes only `tap`/`formula` (ticket 24 B+2), nothing for archive
    or installer names, and `graft-installer.sh` would ship another product's name.
  - A second, bin-only package: `CARGO_BIN_EXE_*` exists only inside the bin's own package, and 18 test sites spawn
    the binary from `crates/graft/tests`.
- If the source is ever published on crates.io, decide crate names then. `scion` and others are taken there; the
  check that `mergewright` is free was made on 2026-10-11.

### 10. The journal's source column: `●  Mergewright`, `$  outside`

- The column stays 14 cells. A `●` row says `Mergewright` (11 cells), a `$` row says `outside`. The legend keeps the
  full sentence: `● done in Mergewright: u can undo it · $ done outside Mergewright: u never touches it`.
- *Why:* `outside Mergewright` is 19 cells. Widening the column costs 5 cells on every row, and at 80 columns the
  summary is already cut. Under `$`, with the legend on screen for as long as the rows are (J1), `outside` reads
  right alone. Gap M8/27-j-column-graft.
- The word comes from `Generation::CURRENT.title()`. `outside` holds no product word, so no later rename cuts it.
- *Rejected:* `outside mwr` (the command is not the product, §1); a wider column; a short form of the product name.

### 11. The Homebrew formula is ours; dist builds the archives and the shell installer

- dist 0.33 compiles its formula template in (`templates.rs:11`). That template puts the man page in `pkgshare`, and
  `brew style` finds 3 offences in it (ticket 24, gap M8/24-brew-style).
- **Decision:** `packaging/homebrew/mergewright.rb.in` is our template. `scripts/formula.py` fills it from dist's
  `dist-manifest.json`: one URL and sha256 per target. It installs `bin/mwr` and `man1/mwr.1`. dist's `homebrew`
  installer is turned off, and `tap` stays unset until publishing.
- *Why:* the cause is a template we can't configure. Owning about 30 lines fixes it there. Patching dist's output
  after every run would be the workaround rule 2 forbids, and forking dist is a second project.
- *Rejected:* post-processing dist's formula (`brew style --fix` plus an inserted line); a custom publish job doing
  the same; forking dist; dropping the man page from the brew install.
- **Cost:** keeping the formula's shape is ours. `scripts/release-local.sh` runs `brew style` on it (read-only, never
  `brew install`), and a test pins that it names the table's package, command and man page.
- `allow-dirty = ["ci"]` goes in the dist metadata, because `release.yml` is hand-edited to `workflow_dispatch` only
  (gap M8/31-dist-plan-hand-edited-workflow).

### 12. Where the codename stays, and where the product name goes

- **Codename stays** where only the project reads it:
  - code names: crates, Rust paths, temp-dir prefixes (§9), fixture identities (`Graft Fixture`), thread names, dev
    caches that already say `graft` (`~/.cache/graft-git-matrix`, …);
  - internal docs: CLAUDE.md, `.claude/**`, DESIGN.md and `docs/design/**` prose, ARCHITECTURE.md, MAP.md prose,
    design notes, ADRs. There "Graft" names the project, not the product (CLAUDE.md).
- **The product name goes** wherever someone using the app reads it: everything the app draws or prints, README,
  CHANGELOG's next entry, the man page, `--help`, TESTING.md instructions, scripts' printed words, UI strings that
  `docs/design/**` mock-ups quote, and PROMPT.md's product lines (the user's file: listed, edited only on his word).
- Dev caches that say `tributary` move to `mergewright`. The script does a one-time `mv` if the old one exists, so
  git/git is not re-cloned.
- **Captures.** TestBackend captures that a test compares regenerate under the gates, whatever their milestone. pty
  captures from milestones before M8 are records of what the app drew then and are not re-shot. If a live doc (README)
  shows a pre-M8 capture as the app today, that capture is replaced with an M8 one.

**Consequences:**
- The 2026-10-09 amendment's `New`/`Old` becomes the generation table. Every site that said `New` now means `CURRENT`.
  Every site that said `Old` names which generations it reads.
- Records keep their words: ADRs, logs, reviews, field notes, KNOWN_GAPS, design notes, past tickets. So do the
  captures of milestones before M8, which record what the app drew then (§12). Live docs, the code and M8's captures
  say Mergewright, `mwr`, `MWR_`.
- Closed by the 2026-10-11 tickets (specs in `.scratch/m8-polish-and-ship/issues/`, conductor numbers them):
  - decided here: M8/27-crate-names-codename (§9);
  - `mergewright-names-table-and-migration`: the generation table, the move from any generation;
  - `mergewright-package-and-command`: M8/27c-dist-artifact-names (package side);
  - `mergewright-app-words`: the product word in code, help and keymap text;
  - `mergewright-env-names`: `MWR_*`;
  - `mergewright-git-messages-and-journal-column`: M8/27-j-column-graft, M8/27-git-messages-graft;
  - `mergewright-release-and-formula`: M8/27a-plan-txt, M8/28-release-docs-stale, M8/24-brew-style,
    M8/31-dist-plan-hand-edited-workflow;
  - `mergewright-docs-sweep`: README, TESTING.md, CHANGELOG, design-doc UI quotes;
  - `mergewright-recaptures`: M8/27d-shots-old-name;
  - `mergewright-protected-lines`: the list for the user (no edits).

## Amendment (2026-10-25): earlier names dropped

- **Ruling (the user):** before publishing, the app reads and writes only its current names; earlier generations' names are not recognised.
- **Went:** the §2–§8 machinery — the generation table (`Generation`, `GENERATIONS`, `CURRENT_DIR`), the rename marker (`MIGRATED_MARKER`) and the move, `names_in_force` / `on_disk_names`, the journal ref mapping, the fixtures' marker.
- **Stays:** the `Migrate` operation kind (read-only, never written, never undone) and the current-name consts in `mergewright_core::on_disk`.
- **Cost:** a repository not opened since the rename keeps its history in the old folder, unread; old stashes, backup refs and practice folders under earlier names aren't recognised (old backup refs are no longer hidden from the graph).
- **Pre-rename journal entries** behave on `u` exactly as before the drop, as 11b3's parity tests (`journal_unmapped.rs`) pin. Open question, not decided here: undo of a gc-pruned entry leaves a pending line and a backup ref before failing.
