# Draft KNOWN_GAPS rows — M6/13 (review m6-amend-13 + ticket 13 NOTES)

Draft only; the conductor pastes into KNOWN_GAPS.md. Format follows the M6/11+12 rows (`KNOWN_GAPS.md:629-638`).
Review Minor 1 is ticket 26's; NOTES D1/D2 are ticket 25's — not drafted.

## From `docs/reviews/m6-amend-13.md` Minors 2–13

| M6/13 | **The split is computed twice, and the MAP and NOTES say once.** `view/frame/conflicts.rs:44-46` builds its own `placement`/`resolver_areas` from the drawn rect; the mouse uses `view::resolver::resolver_split` (`view/resolver.rs:218`), which rebuilds it from `state.size`. They agree today only because `draw_resolver_banner` (`conflicts.rs:86-90`) does the same arithmetic. Fix: `draw_resolver` calls `resolver_split`, or one shared fn over the body rect. **Fixed by 14 — drop on merge.** Source: `docs/reviews/m6-amend-13.md` Minor 2 | **broken** (docs wrong; latent drift) | M7 |
| M6/13 | **No gap on either side of the column's `│`** (textual). `fix-file-list-200x50.txt` row 4 `▏▸ notes.txt  2 of 2 open│  1 one`; the design note's mockup has a space each side. Cause: `column_width` is exactly the longest row (`files.rs:537`) and the doc starts at `list.right() + 1` (`files.rs:179-184`). Fix: pad by one cell. Source: `docs/reviews/m6-amend-13.md` Minor 3 | **broken** (textual) | M7 |
| M6/13 | **Unused parameter** `whole_file_view(…, _columns: u16)` (`view/resolver.rs:64`). Fix: remove it. Source: `docs/reviews/m6-amend-13.md` Minor 4 | **broken** (code hygiene) | M7 |
| M6/13 | **`listed_files` runs about 4 times per frame, each O(n²).** `contains` scans over `Vec<PathBuf>` at `update/resolver/files.rs:77-81`, `:89`; called by `file_list_view`, `file_cursor`, from both `resolver_view` and `draw_resolver`, and again in `body()` for keys/mouse. No test with hundreds of files. Fix: `HashSet` membership; cursor from the same `Vec`. Source: `docs/reviews/m6-amend-13.md` Minor 5 | **unverified** (cost unmeasured) | M7 |
| M6/13 | **Two full status reads per `⏎`.** `jobs/conflict.rs:258-273` runs `read_facts` → `graft_git::status::read` (`graft-ops/src/facts.rs:108`), beside the `ReadWorkingTree` the same Stage queues. Fix: a scope that skips status, or `operation_conflicts` takes the app's status. Source: `docs/reviews/m6-amend-13.md` Minor 6 | **unverified** (cost unmeasured on a large tree) | M7 |
| M6/13 | **Untested branches:** `switch_refusal`'s `staging`/`restarting` arm (`update/resolver/files.rs:198-200`, `a write is already running`), and the box case (review Major 1). Source: `docs/reviews/m6-amend-13.md` Minor 7 | **unverified** | M7 |
| M6/13 | **A resolver mouse test was narrowed.** `tests/it/resolver/modal.rs:106-109` `mouse_uses_the_key_functions` now clicks rows 5..20, not 0..14; rows 0–4 (banner, strip, rule) no longer checked for "no click reaches the graph". Fix: keep rows 0..5, assert only the graph selection there. Source: `docs/reviews/m6-amend-13.md` Minor 8 | **unverified** (coverage lost) | M7 |
| M6/13 | **Half of `file_at_matches_what_is_drawn` is vacuous.** `graft-ui/src/conflict_resolver/files/tests.rs:201-210` probes `dy == rows`, `dx == width`, both outside the plan. Nothing covers a click on the `│`. Fix: an app-level click on the `│` at 200 columns, asserting nothing changed. Source: `docs/reviews/m6-amend-13.md` Minor 9 | **unverified** | M7 |
| M6/13 | **Stale docs quote the old title** `file 12 of 13 · 2 of 3 conflicts open`: `TESTING.md:2245`, `docs/shots/m6/m6-contact-sheet-captions.tsv:1,3,9` (and the sheet), design note :152; `m6-01…m6-05` captures predate the change. Fix: regenerate and recaption at the acceptance run (ticket 21). Source: `docs/reviews/m6-amend-13.md` Minor 10 | **broken** (stale captures and checklist) | M7 |
| M6/13 | **`(ours)` in the footer of every `fix-file-list-*` capture** (`resolved gone.txt (ours) ✓`): trailing parenthesised word, git's side word not the side's name — against ADR-0043 §4 and 02-keymap §2.1. From `graft-ops/src/operation/conflict.rs:129-135`; predates ticket 13. Source: `docs/reviews/m6-amend-13.md` Minor 11 | **broken** (textual, pre-existing) | M7 |
| M6/13 | **Question for the user's eye: `Unread` draws `…` after its name** (`fix-file-list-many-120x40.txt` row 2 `src/handlers/handler_02.rs …`); next to `… 7 more` it can read as a cut name. The design chose `…` (note :674-675 at `b0e08fd`). Wording only. Source: `docs/reviews/m6-amend-13.md` Minor 12 | **unverified** (question, user's eye) | M7 |
| M6/13 | **Question: does `gone.txt` read `Unread` for a moment, as the NOTES say?** By the lines it should read its last-seen state: `conflict_arrived`'s card branch calls `remember` (`update/resolver/answers.rs:56`); `listed_files` falls back to `seen` (`files.rs:94`). Not verified either way; the builder should name the run that showed `Unread`. Source: `docs/reviews/m6-amend-13.md` Minor 13 | **unverified** (question) | M7 |

## From ticket 13 NOTES Gaps (`.scratch/m6-conflicts/issues/13-resolver-file-list.md:162-167`)

| M6/13 | **Colour of the file list, the rule and the `│` is unverified; the look at every size is the user's call.** Not in a character grid (rule 5); by eye only (rule 6). Source: `.scratch/m6-conflicts/issues/13-resolver-file-list.md:163`, `:122` | **unverified** (colour, look) | M7 |
| M6/13 | **A failed set read's reason is never shown.** It is kept in `Loading::Failed(why)`, but only the label `git's list unavailable` is drawn. Source: `.scratch/m6-conflicts/issues/13-resolver-file-list.md:164` | **broken** (minor) | M7 |
| M6/13 | **A done stash-pop file drops out of the list at the next re-read.** Stash pop has no record (`recorded: false`; graft-ops MAP: no `MERGE_MSG`). Source: `.scratch/m6-conflicts/issues/13-resolver-file-list.md:165`, `:42` | **broken** (minor) | M7 |
| M6/13 | **Every whole-file shape draws `keep or delete`.** `FileState::WholeFile { how }` carries the conflict shape, but the list does not use it. Source: `.scratch/m6-conflicts/issues/13-resolver-file-list.md:166` | **unverified** | M7 |

Not drafted: NOTES `:167` — `view/frame/tests.rs:408` uses an old status string as an arbitrary sample; the builder marks it harmless.

## Release perf under load — duplicate, no new row

Review (`docs/reviews/m6-amend-13.md:23-31`): 2 of 13 failed on the first run under load (load average 3–4, `:13`), both passed alone; review itself says the shape matches "KNOWN_GAPS row 211". Covered by:
- `KNOWN_GAPS.md:211` — "The release perf p99 measurement is too noisy to trust", expiry M7. Suggest appending this run as evidence (p99 55.3 ms / max 136 ms loaded vs 5.37 / 5.50 ms alone).
- Related: `KNOWN_GAPS.md:446-447` — "The frame gate asserts on the max of 420 keys, so one outlier fails it", M7.
