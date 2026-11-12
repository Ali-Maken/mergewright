# pre-publish: raise three heavy budgets, delete stale codename shots

Status: done
Build with: conductor-run builder (worktree `ppc-final`), no scout (known sites)
Source: `docs/perf/pp-final.txt`, `docs/perf/pp-12-no-writer.txt`; user ruling 2026-11-12.

## Acceptance
- [x] A1. `held_key` bytes-per-frame bound 512 KiB to 2 MiB, comment with measured 1.15-1.55 MB. Verified by `held_key::held_j_stops_on_release_pixel_free`, 5/5 release (`docs/perf/pp-final-budgets.txt`).
- [x] A2. `piece_gate` per-frame bound 100 % to 135 % (`FRAME_PERCENT`; measured 120.3 %); the 25 % walk gate unchanged. Verified by `piece_gate::no_frame_rasterises_more`, 5/5 release; `pieces_localise_on_his_walk` untouched.
- [x] A3. 100k pixel-tier p99 budget 16 ms to 24 ms for that path only (`frames::budget`; measured p99 17.7 ms, worst key 21 ms). It failed on the p99, not on blank frames. Verified by `frames::scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_pixel_tier`, 5/5 release. The blank-frame (`waited`) assertion is unchanged.
- [x] B1. 243 listed shots removed (`git rm`); 23 listed files kept (16 are the eight M4 stems, both sizes, read by `m4_walk::every_scenario::captures_are_named_keyed_and_listed_in_the_matrix`; still name the codename, held by eight narrow PENDING rows) because tests read them (m5, m6, m8, ease-pass caption .tsv; ease-pass sheets) and the m7 .tsv because `scripts/m7-accept.sh` builds its sheet from it. Their stale rows removed, caption words reworded. Verified by `m5_walk::every_scenario::contact_sheet_captions_cover_every_m5_capture`, `..._m6_capture`, `m8_walk::contact_sheet::contact_sheet_captions_cover_every_m8_capture`, `ease_walk::shots::the_contact_sheet_names_every_capture`.
- [x] B2. Broad `docs/shots/` `PENDING` row removed; eight exact-stem rows remain for the kept M4 captures. Verified by `codename_absent::no_shipped_file_names_the_codename`.
- [x] B3. Capability citations to deleted captures dropped, `docs/generated/capabilities.html` regenerated. Verified by `mergewright-capabilities::checker::every_capability_citation_checks_out`.
- [x] KNOWN_GAPS: pp/12-shots-stale-codename closed to `docs/gaps/closed.md`; pp/12-heavy-reds and pp/12-stale-contact-sheets updated.
- [x] Gates: fmt, clippy, heavy clippy, doc, nextest 6796/6796, walks 459/459 (tests: all of the above).

## NOTES
- Not a pure bound: none. All three failed on a number. The ease/19 blank-frame check was not hit and was left alone.
- Left as history: reviews, logs, ADRs and design notes still name deleted captures as past evidence.
- Colour unverified (no visual change).
