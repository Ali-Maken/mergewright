# Scout: 07a ahead-counts

## What exists — every local branch already carries its count
- `refs::RefLabel::Branch { name, current, upstream: Upstream }` — `crates/mergewright/src/refs/mod.rs:140-150`. `Upstream::{None, InSync{remote}, Apart(AheadBehind)}` — `:158-169`. `AheadBehind { ahead, behind }` — `crates/mergewright-git/src/history/divergence.rs:26-32`.
- Filled by `refs::badges` for **every** `refs/heads/*` whose upstream is configured and exists: one `count(local, upstream)` per such branch — `refs/mod.rs:295-322`. No upstream / upstream ref missing → `Upstream::None` (:302). Counted 0/0 → `InSync` (:306), so `ahead == 0` is stored as `InSync`, not `Apart`.
- `RefBadges::branches()` already yields `(tip, name, current, &Upstream)` for every local branch — `refs/mod.rs:85-96`.
- Calls go through `AheadBehindCache::get_or_count` keyed `(tip, upstream_tip)` — `refs/mod.rs:171-224`; cache lives on the job runner for the whole run (`jobs/runner/mod.rs:48,122`). Has test-only `walked_on()` thread recorder (:209).
- Thread: `JobRunner::read_refs` queues `Task::ReadRefs` off the loop — `jobs/runner/reads.rs:91-102`. Issued at launch (`update/mod.rs:133`), on refresh (`update/keys/refresh.rs:56`), enum at `update/events/command.rs:47`; `place/write.rs:35`, `radar.rs`, `banner/stop.rs:189` also name it. Re-read after each operation: *could not confirm* the exact call sites. Never per frame.

## Who reads it
- ops `UpstreamFacts` is a translation of each branch's `Upstream` — `update/radar.rs:161-166` (per branch, via `branch_facts`).
- ui `Tracking::{Untracked, InSync, Apart{ahead,behind}}` — `mergewright-ui/src/graph_screen/badges.rs:69-83`, filled per branch label at `view/entries.rs:395-407`.
- Header `N commits to push`: `push_words` reads HEAD's branch from the same `state.refs` — `update/commit_editor/landing.rs:140-155`. Push box also: `update/remote/push_box.rs:204`.

## Contradictions with the ticket
1. "only HEAD's count (`counts.ahead`, divergence.rs:29)" — wrong: line 29 is the `AheadBehind` struct field, stored per branch (refs/mod.rs:304-311). Path A applies.
2. "The header's count keeps its own source" — it has no separate source; same `RefBadges` (landing.rs:141-143). The equality test is trivially true.
3. "exactly 200 `ahead_behind` calls per refs read" — false after the first read: cache skips unmoved pairs (refs/mod.rs:182-184). Holds only on a cold cache.
4. "None before landing" — before the first refs read `state.refs` is `None`; there is no separate landing.
5. Accessor takes `&RefLabel`, but `RefLabel::Branch` already holds `upstream`; no lookup needed.

## Build with
Sonnet: path A is an accessor plus tests; no new job.
