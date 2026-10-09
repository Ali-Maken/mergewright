# Scout 35 — split graft `jobs/runner.rs`
Root main @ fc6972fe. Crate set: **graft only** (`src/jobs/`, `MAP.md`).

## File
`crates/graft/src/jobs/runner.rs` = **1481 lines** (ticket says 1457). Declared `mod runner;` `jobs/mod.rs:91`, `pub use runner::JobRunner;` :100.
- :4–30 `use`.
- :35–88 radar probes: `radar_context`:35, `evil_probe`:50, `force_push_probe`:59, `duplicates_probe`:75 (54).
- :89–121 `pub struct JobRunner<S>`.
- :122–575 impl #1 (454): `start`:128, `with_askpass_helper`:193, `with_clock`:207, `answer_credential`:219, `with_default_size`:228, `worker_count`:235, `submit`:243, `announce`:254, `queue_job`:264, `dispatch_detail`:270, `dispatch_history`:335, `read_stash_only`:371, `blame_job`:380, `detail_job`:389, `queue_as`:410, `cancel`:456, `row_requests`:462, `unstoppable`:476, `force_stop`:487, `writes_running`:494, `shutdown`:525, pub item :537.
- :576–1389 impl #2 (814), job fns: repo/history/refs :587–683; conflict/finish :684–790; commit msg/rewrite/row tags :791–888; resume/operate/credential :889–982; history/undo/preview/journal/tour/recover :983–1168; stash/switch/palette/index/search/remotes :1169–1306; leaves/fold/map/show_rows :1307–1389.
- :1390–1478 `impl Dispatch for JobRunner` (`dispatch`:1397).
- :1479 `not_a_job`.

## Seams / counts
lifecycle+queue (:89–575) ~490 → over; split submit/dispatch_* (:243–455 ~210) from start/config/cancel/shutdown (~280). impl #2 → 3 families of ~270: reads (:587–888), operations+undo+recover (:889–1168), lists+map (:1169–1389). Dispatch+not_a_job ~92. Probes ~54.

## Pins
No inline tests in runner.rs. `jobs/tests.rs` (1285 lines). event_loop calls `JobRunner` via `jobs::JobRunner`. No captures.

## Contradictions
- "Inline tests move out first": **none inline**.
- `over_400.txt` / `file_size` test **absent** on main (ticket 33 unlanded).
- Shares `crates/graft/MAP.md` with 36 (ticket says so) and with 39 (39 reads/edits graft MAP:1085–1100).
