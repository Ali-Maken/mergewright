# Scout 18 — a stop opens the resolver (main c56b64e)

Paths relative to `crates/graft/src/update/`.

## What exists
- `operate.rs:740 operated`: clears `state.operating` (`:749`) first. `Conflicted` arm `:783-822`: `live_entries`, `state.stopped = Stop{again:false, paths_at_stop: paths.len()}`, status Ready/sticky(stalled), `banner_focus = banner_focusable(kind)` (`:809`), returns `refresh_scope(Everything)`. Does not open the resolver.
- `banner.rs:587 resumed`: clears `operating` (`:596`); `Conflicted` arm `:619-629` → `stop::stopped_again` (`banner/stop.rs:21`: banner_focus, `Stop{again:true}`, status) then `refresh_scope(Everything)`. No resolver.
- `resolver/open.rs:18 open_resolver(state, commands, path, from, next_id) -> bool`: refuses if a resolver is already open (`:25`), a box is open (`:61`), or `state.operating.is_some()` (`:66`); else `start` (OpenConflict job) + `files::reread_set` (`:39`). `operating` is already `None` in both call sites.
- Banner `⏎` already calls it with `first_conflicted` + `OpenedFrom::Banner` (`banner.rs:333-343`).
- `Esc` → `resolver.rs:363 close`: for `OpenedFrom::Banner` sets `banner_focus = banner_view(state).is_some()` (`:367-370`); Sticky status → Ready. Item 2 already holds, as code.
- Keys: none new. Keymap §2.6a (`02-keymap.md:284-290`) already states P5.

## Scout question — paths order
- `paths` come from `graft_ops/src/apply/resume.rs:130 unmerged_paths`: `graft_git::status::read` entries filtered `unmerged`, in entry order (`:131-138`); used at `apply.rs:373`, `resume.rs:245,264`.
- `banner.rs:77 unmerged_paths` / `:94 first_conflicted` filter the same `status.entries` of `state.working_tree`. **Same source, same order** → the answer's `paths[0]` is git's first; no need to wait for the re-read. At `operated` time `working_tree` is the pre-stop read, so `first_conflicted` would be stale/empty there.

## Contradictions
1. Acceptance "`file_cursor` is that file's index": `file_cursor` (`resolver/files.rs:135`) derives from the file list, which `reread_set` fetches asynchronously — at `operated` time the set is `Loading`; could not confirm what `file_cursor` returns before it lands.
2. Ticket reads `§update::banner` has `resumed` in `banner.rs`; the stop words live in `banner/stop.rs` — fine, but the call site for `stopped_again` is `banner.rs:627`, not `stop.rs`.
3. Ticket 12's test `a_stop_keeps_the_banner_keys_on_the_bottom_row`: location not checked — could not confirm.

## Could not confirm
- Stash-pop conflict path (no banner) — not traced; must stay out of `operated`'s `Conflicted` arm.
- Whether `open_resolver` sets the cursor on block 0 before the read lands (`answers.rs:59` sets it via `next_open_block` on arrival).
