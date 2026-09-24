# Draft KNOWN_GAPS rows: review m6-amend-18, Minors 2–7

Critical 1, Majors 1–2 and Minor 1 are in ticket 29, so they are not here.
Minor 6 gets no new row: the review says the existing M6/06 row (`operate.rs`/`undo.rs`, PROMPT.md 6.2) already tracks it (`docs/reviews/m6-amend-18.md:134-136`). The conductor may append "+ ticket 18: 4 code + 63 test lines, now 2432 lines (991 non-test)" to that row.

| M6/18 | **`banner_focus` assertions that cannot fail** (`conflict_interim.rs:156`, `banner/tests.rs:1053`). Both run after the `Esc`, but `banner_focus` is already true while the landed resolver is open. The m5_walk `BannerFocused` predicate (`m5_walk/run.rs:518-524`) does not require `resolver.is_none()`, so the walk never checks that its new `Esc` closed the resolver. Fix: add `state.resolver.is_none()` to `BannerFocused`. Source: `docs/reviews/m6-amend-18.md` Minor 2 | **broken** (vacuous asserts; the behaviour itself is unverified by these asserts, not known wrong) | M7 |
| M6/18 | **Stale captions** (`docs/shots/m5-contact-sheet-captions.tsv:10-12`). They still describe the banner and a sticky `… resolve it and s… · esc dismiss`, but the regenerated m5-06 captures show the resolver. Textual finding. Fix: reword them from `TESTING.md:2019`. Source: `docs/reviews/m6-amend-18.md` Minor 3 | **broken** (docs) | M7 |
| M6/18 | **`crates/graft/MAP.md` stale for the landing in the resolver.** The item list for `update/banner/stop.rs` (`:299-306`) does not include `land_in_resolver`. `operated` (`:225`) describes `Conflicted` without the landing. `resumed` (`:295`) says "the banner **stays focused**" and does not mention the resolver. Fix: add one item line and one clause to each of the two entries. Source: `docs/reviews/m6-amend-18.md` Minor 4 | **broken** (docs) | M7 |
| M6/18 | **At 187x50 the second file's name is cut while its column has room?** `docs/shots/m6/fix-stop-lands-in-resolver-187x50.txt` row 5 reads `   …ared.rs  …` in a 24-column list. `shared.rs` plus its state needs 15 columns. It looks cut to the width `notes.md` left after `1 of 1 open`. This is ADR-0043's list column (ticket 13), not ticket 18's code. Question for the user's eye and the list column's owner: should names take the width that a short state leaves? Source: `docs/reviews/m6-amend-18.md` Minor 5 | **unverified** (question; no line proves wrong logic) | M7 |
| M6/18 | **Release perf run 1 missed one frame.** `frames::fifty_thousand_refs_keep_input_to_frame_under_16_ms_with_the_map_view_open`: key 132 took 16.31 ms, 16.1 ms of it queued behind key 131, and its own frame took 0.17 ms. Run 2, alone, passed (p99 7.9–9.0 ms). The machine was loaded (load average about 4, other agents' cargo). The cause is **not verified as load**. Ticket 18 touches no frame or Map code. No existing row covers queue-time misses under load (grep of KNOWN_GAPS for "queue" on 2026-09-24 found none about frames). Source: `docs/reviews/m6-amend-18.md` Minor 7, Gates `:21` | **unverified** | M7 (next release perf run) |

## The snapshot question: answered, no row drafted

Question: does `add -A` into the temporary index (`crates/graft-ops/src/snapshot/plumbing.rs:188`) collapse the unmerged stages so that undo cannot bring a conflict back?

No. Source code and ticket 08 both answer it:
- `crates/graft-ops/src/snapshot.rs:231-243` (`both_trees`) reads the index side (`index_state_of`, which gives the stage tree when the copy is unmerged) **before** `add_all` runs on the same copy. The doc comment at `:232-233` says: "The index side is read **before** `add -A`… after it, the stages are gone." The `add -A` tree is used only as the worktree tree.
- `.scratch/m6-conflicts/issues/08-ops-undo-an-unmerged-index.md:58` (Decision 1) and its tick at `:128`: `unmerged_index_round_trips` restores `ls-files --stage` byte-for-byte after the index was wrecked with `add -A`.

I did not run that test. My answer rests on reading the source and on ticket 08's tick.
