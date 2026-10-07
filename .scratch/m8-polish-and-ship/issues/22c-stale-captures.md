# G-hygiene 2c — 09h's `?`-at-Map capture; the three hand-shot search captures; the two literals 10i could not find

Status: draft (planner spec-only pass 2026-10-07; from 09h NOTES "Next (split)", 10h NOTES "Captures box OPEN", 10i
NOTES "Not found in Files", KNOWN_GAPS `M8/10i-literals-not-found`)
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
- [ ] A: `tests/it/look_polish.rs::help_at_the_map_lists_only_live_keys` green after one `GRAFT_UPDATE_SHOTS` run;
      `docs/shots/m8/help-at-map-120x36.txt` exists; HERE/EVERYWHERE as A1 (read and quoted in NOTES). Judged by the
      user (rule 6); colour unverified (rule 5). Conductor ticks 09h's Captures box with this test.
- [ ] A3: `git diff --stat <base> -- docs/shots` output in NOTES; every file in it new or explained (cited).
- [ ] B1: NOTES answers the mechanism with file:line, and the `rg` output.
- [ ] B2: `<file named in NOTES>::the_search_hint_draws_his_esc_spelling` (via `check_shot`) green;
      `docs/shots/m8/search-hint-120x36.txt` holds the three frames, `esc cancel` / `esc close`, no `Esc`. Colour
      unverified.
- [ ] B3: the three files' bytes unchanged (`git diff --stat <base> -- docs/shots/26-search-far.txt
      docs/shots/26-search-typing.txt docs/shots/m2-18-git-git-search-enter-160x40-unicode.txt` empty); records listed
      where B1 says (diff shown). Conductor closes 10h's Captures box with B2's test.
- [ ] C: the `rg` output and classification in NOTES; conductor closes or re-files `M8/10i-literals-not-found` from it.

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
