# G-hygiene 2c — 09h's `?`-at-Map capture; the three hand-shot search captures; the two literals 10i could not find

Status: done (builder 2026-10-07; was ready, conductor 2026-10-07; from 09h/10h/10i NOTES, KNOWN_GAPS
`M8/10i-literals-not-found`)
Build with: **Sonnet** — three decided steps; the one judgement (is a capture diff explained) has a stop rule. Not Fable.
Blocked by: **12 merged** (met, per conductor). **Capture run serial** with 11c's and 12d's (all rewrite
`docs/shots`); may run **beside 17c** (17c writes only its new `docs/shots/m8/polish-ghost-pixel-120x36.txt`; shared
`tests/it/look_polish.rs` only if 17c adds its capture test there — additive fns, conductor merges). Two cargo helpers
at a time.
Needs: 09h's merged work (`sweep_tables`, `rewrite_editor` AFTER); `captures::check_shot`, `captures::frame_at`;
the `search` hint words as 10h left them (`esc close` / `esc cancel`, shipped keymap).
Crate set: graft tests only (+ graft-fixtures only if step B1 shows the record list belongs there).
**Files:** graft `tests/it/look_polish.rs` (one capture test), the `tests/it` file holding search pty/capture tests
(or a new `tests/it/search_hint.rs`; NOTES names it); `docs/shots/m8/help-at-map-120x36.txt` (new),
`docs/shots/m8/search-hint-120x36.txt` (new); `TESTING.md` "Captures" (one paragraph). Any other `docs/shots` file
only if the full run rewrites it and NOTES explains it (rule A3).
**Not:** any production source; `docs/shots/26-search-far.txt`, `26-search-typing.txt`,
`m2-18-git-git-search-enter-*-unicode.txt` (bytes untouched — Decision B); `README.md` (23); `KNOWN_GAPS.md` (conductor).

## Read first
- `09h-keymap-review-last.md` Captures box (:147–152) and NOTES "Next (split)" (:204–205); R4 (:99–101, the allowance)
- `10h-keymap-words-undo-journal.md` NOTES "Captures box OPEN" (:71–74)
- `10i-keymap-words-preview-remote.md` NOTES "Not found in Files" (:69)
- `TESTING.md` "Captures" (:27–85)
- `crates/graft/MAP.md` `captures` / `check_shot` entries; `crates/graft-fixtures/MAP.md` `capture`, `Captures`
- Source after the maps: `tests/it/look_polish.rs` (an existing capture test as the pattern)

## A — 09h's `?`-at-Map capture (09h's open box, now unblocked)
1. Add `help_at_the_map_lists_only_live_keys` to `tests/it/look_polish.rs` via `captures::check_shot`: Map open, graph
   focused, `?`, 120x36. HERE = Map's own keys; EVERYWHERE = Global keys that act there; no `c commit`, `R`,
   `m merge`, `/ Find…`.
2. One full `GRAFT_UPDATE_SHOTS=1 cargo nextest run` (red by design), then plain, green.
3. `git diff --stat <base> -- docs/shots` = the two new files (A and B2) plus 09f's `?`-at-overlay allowance (R4). Any
   other file: NOTES gives file, first differing line, and the merged ticket whose NOTES explains it (cite the line).
   Explained → keep; **unexplained → stop**, do not commit it (a capture nobody can account for is not evidence).

## B — the three hand-shot search captures (10h's open box)
**Decision (planner): records, not regenerated, not hand-edited; the current search hint gets its own generated
capture.**
- Why not hand-edit: the file would claim a frame the app never drew (rules 1, 6). Why not regenerate in place: they are
  records of their tickets — `26-search-typing.txt` is m1/26's timing run on the 100k synthetic repo (F2 latency lines),
  `m2-18-git-git-…` is M2 acceptance stop 18 on git/git (TESTING.md :1269, heavy tier). Every row of them is already
  stale against today's chrome (old bar `q quit  F2 status …`, no header rule, m8/06), so `Esc` is one drift of many;
  re-shooting would replace the record with a different scene and still need the 85k-commit clone.
- **B1.** Answer with file:line: how does a `docs/shots` file with no generator coexist with TESTING.md :29 ("every
  file under `docs/shots/` is asserted by every `cargo nextest run`")? An exemption list, or is the claim false for
  hand-shot files? Confirm no generator: `rg -n '26-search|m2-18-git-git' crates scripts tests` (output in NOTES).
- **B2.** New generated capture `docs/shots/m8/search-hint-120x36.txt` (fast-tier fixture, shipped keymap), three
  frames: search running (`… matches so far …  n/N next/prev  esc cancel`), settled with matches (`… (1/N)  n/N
  next/prev  esc close`), no match (`no matches  esc close`). Words as the app draws them — the test asserts nothing
  beyond `check_shot`.
- **B3.** Make the records explicit where B1 says they live: an existing exemption list → add the three (with "record
  of m1/26" / "record of M2 stop 18"); no such list → one TESTING.md "Captures" paragraph: hand-shot files from
  before m5/40 are records of their ticket, not current output, not asserted; list them by name. Check the five other
  files holding `Esc close|Esc cancel` (`17-mouse-select-and-tooltip`, `m2-explain-panel`, `m2-map-view-loading`,
  `m3-contact-sheet`, `m3-commit-editor-rejected`): generated (cite the test) or record (listed). More than ~15
  records → list them in NOTES and stop B3 for a conductor ruling.

## C — the two literals 10i could not find (`M8/10i-literals-not-found`)
Planner evidence, not source (check it): `p pull first` was reworded in M7/36 to `p pull, then m or r`
(`10b-grep-hits.txt:111`) and spelled by 10e (its acceptance :88); `c commit  S stash` appears as M6's **proposed fix**
for graft-ops "commit or stash first" (KNOWN_GAPS :767) and as hint-bar text (uncommitted row, spelled by 10c). So
neither may exist as a user-facing literal.
- `rg -n 'c commit  S stash|p pull first|commit or stash first' crates` → NOTES, each hit classed: test/doc-only;
  spelled from the keymap (cite the fn); or **live literal → report with file:line, not fixed here** (outside this
  crate set; conductor files it).
- Docs hits, report only: `README.md:179` (→ 23), `TESTING.md:1672, :1751` (M4 records).

## Signatures
None (test fns only).

## Acceptance (each tick names its test, at tick time)
- [x] A (`look_polish::help_at_the_map_lists_only_live_keys`): green after one `GRAFT_UPDATE_SHOTS` run;
      `docs/shots/m8/help-at-map-120x36.txt` exists; HERE/EVERYWHERE as A1 (read and quoted in NOTES). Judged by the
      user (rule 6); colour unverified (rule 5). Conductor ticks 09h's Captures box with this test.
- [x] A3 (`git diff --stat b089a72c -- docs/shots`, NOTES): `git diff --stat <base> -- docs/shots` output in NOTES; every file in it new or explained (cited).
- [x] B1 (`rg` + `grep read_dir`, NOTES): NOTES answers the mechanism with file:line, and the `rg` output.
- [x] B2: `crates/graft/tests/it/search_hint.rs::the_search_hint_draws_his_esc_spelling` (via `check_shot`) green;
      `docs/shots/m8/search-hint-120x36.txt` holds the three frames, `esc cancel` / `esc close`, no `Esc`. Colour
      unverified.
- [x] B3 (`git diff --stat` of the three, NOTES): the three files' bytes unchanged (`git diff --stat <base> -- docs/shots/26-search-far.txt
      docs/shots/26-search-typing.txt docs/shots/m2-18-git-git-search-enter-160x40-unicode.txt` empty); records listed
      where B1 says (diff shown). Conductor closes 10h's Captures box with B2's test.
- [x] C (`rg` over crates, NOTES): the `rg` output and classification in NOTES; conductor closes or re-files `M8/10i-literals-not-found` from it.

## Touches
- Contact sheet: m8 sheet re-assembled with the two new captures (conductor, ADR-0037).
- 09f's / 09h's tests: untouched; full nextest green.
- 17c: `tests/it/look_polish.rs` if shared (additive); its capture file not in this diff (shown).
- 11c, 12d: capture runs serial (conductor).
- TESTING.md: "Captures" only; no checklist rows edited (C's hits are reported, not changed).
- Production source, keymap, journal/undo: none — `git diff --stat <base> -- crates/*/src` empty (shown).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
~30k: A ~10k (incl. one full update run); B ~12k; C ~3k; NOTES ~5k.

## NOTES (builder, 2026-10-07)
Scout: none — the ticket's sources (09h/10h/10i NOTES, TESTING.md) plus two greps were enough; test-only, two files.
No compaction. Base b089a72c.

**A.** `look_polish::help_at_the_map_lists_only_live_keys`: `M` opens the Map, then `?`. (First try pressed `?` on the
graph — that is not the Map; fixed before the file was written.) Quoted from the capture: HERE = `M Close the Map
view`, `⏎ Seek the graph to this branch`, `S Stash…`, `J Undo journal`; EVERYWHERE = `j`, `k`, `esc`, `⌃P`, `?`, `u`,
`U`, `⌃R`, `⌃C`, `— Open / drill in` (unbound). No `c commit`, `R`, `m merge`, `/ Find…`. **Question (not a defect,
rule 7):** `S Stash…` and `J Undo journal` are listed under HERE with ids `graph.stash`/`graph.journal` at the Map —
whether they act there was not checked. Judged by the user; colour unverified.

**A3.** `GRAFT_UPDATE_SHOTS=1` full run: 3 red by design (the two new tests, `graft-graph raster::write_shots`
"regenerated 10 capture(s)" — it rewrites always). `git diff --stat b089a72c -- docs/shots` afterwards: **empty**;
untracked only `m8/help-at-map-120x36.txt`, `m8/search-hint-120x36.txt`. 09f's R4 allowance did not occur.
`git diff --stat b089a72c -- crates/*/src`: empty.

**B1.** No exemption list: the claim at TESTING.md :29 is false for hand-shot files. Assertion is per call —
`captures::check_shot` (crates/graft/tests/it/captures.rs:188) → `graft_fixtures::capture`; nothing enumerates
`docs/shots/` (`grep read_dir` over graft-fixtures/src hits only cache.rs:157, scratch.rs:91, permissions.rs:39 —
none on shots). `rg -n '26-search|m2-18-git-git' crates scripts tests` → no hits (`tests` dir absent).

**B2.** File `crates/graft/tests/it/search_hint.rs`, corpus OctopusMerge, query `topic`/`zzqx`. Drawn lines:
` /topic  4 matches so far (1/4)  n/N next/prev  esc cancel`; ` /topic  4 matches (1/4)  n/N next/prev  esc close`;
` /zzqx  no matches  esc close`. The running frame is the settled AppState with `search.state` set to `Running`
(caption says so) — a fast-tier search finishes before any frame. No `Esc` in the file.

**B3.** Three files' `git diff --stat` empty. Paragraph "Records, not asserted" added to TESTING.md Captures. Other five:
`m2-explain-panel` → `view/tests.rs:1780`; `m2-map-view-loading` → `view/tests.rs:2027`; `m3-commit-editor-rejected` →
`view/tests.rs:2709` (generated); `m3-contact-sheet` → `scripts/contact-sheet.sh` (its header); `17-mouse-select-and-
tooltip` → record of m1/17 (listed). 4 records total.

**C.** `rg -n 'c commit  S stash|p pull first|commit or stash first' crates` → **no hits**. Neither literal lives in
crates; `M8/10i-literals-not-found` can close. Docs hits (report only): README.md:179 `p pull first` (→23);
TESTING.md:1687, :1766 `p pull first` (ticket's :1672/:1751 shifted); TESTING.md:2177, :2865 `c commit  S stash`
(hint-bar checklist rows).

**Touches:** production src empty (diff above); look_polish.rs additive; main.rs `mod search_hint`; TESTING.md
Captures only; m8 contact sheet not reassembled (conductor). Gates: fmt, clippy, nextest 5876/5876, doctests green.
