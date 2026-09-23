# Scout — 10 resolver line numbers / side names

## Projection today (graft `view::resolver`)
- `resolver_view` builds `DocView` at `crates/graft/src/view/resolver.rs:25-45`; whole-file variant `:58-79`. `explainer` set from `resolver_explainer` at `:45`, `:79`.
- `regions()` `:267-306` walks `file.regions` with one counter `line` = **disk line of `render()`**: Plain adds its len `:279`; open Conflict passes `line` (the `<<<<<<<` line) then adds `conflict_lines` = 3 + ours + base(+1) + theirs `:290`, `:362-365`; Resolved adds `taken_lines` `:301`, `:368-374`. Only one number per block (`first_line`); no per-side counter exists.
- `block_view` `:309-339`: `ours_label`/`theirs_label` = git's raw marker label, fallback `"ours"`/`"theirs"` `:320-321`, `:342-347`.
- graft-git: `Block.ours_label`/`theirs_label: Option<Vec<u8>>` `crates/graft-git/src/conflict.rs:150`; `BlockAt.first_line` doc (marker line open / first taken line) `:244-253`; `render` `:581`, `blocks` `:646`. No `Projection` / `first_lines` yet.

## Where numbers are drawn (graft-ui `conflict_resolver.rs`)
- Plain: `{line_no:>5}` `:563`, fixed 5-wide; whole-file lines numbered `offset+1` `:307`; Note blank `:577`.
- **Stray marker `2`:** `draw_header` prefix `format!(" {} ", block.first_line)` `:635` — the `<<<<<<<` disk line, on the Header row.
- Pair (≥100): ours = `first_line+1+i` `:684`; theirs `numbered(None,…)` `:690` — **why only OURS numbered**.
- Stacked (<100): Ours `:729-734`; Theirs `None` while Open or Took(Ours) `:743-749`; Base `None` `:759`. `numbered` fixed 5-wide `:810-814` (no computed gutter width).
- Labels: `"OURS · {ours_label}"`/`"THEIRS · …"` `:651-655`, `:712-718`; header keys `"o ours  t theirs  b both"` / `took ours` etc. `:617-622`; explainer row drawn `:915-921`.

## Names / state sources
- Explainer text `view/resolver.rs:198-219` (`ours = … · theirs = …`, stash case `:203`); `onto_name` `:224-232` (branch else 7-sha); `branch_on` `:239-251` (local branches only, no remote badge); `current_branch` `:254-264` (None when detached — no short-sha fallback).
- AppState: `refs` `crates/graft/src/update/state.rs:70`, `in_progress` `:217` (`InProgressDetail.onto: Option<Oid>` `graft-git/src/state.rs:133`), `resolver` `:281`. MERGE_HEAD label: only via marker `theirs_label`; no separate field found (*could not confirm* another source). Replayed-commit label for rebase: *could not confirm* any field beyond marker label.
- Hints: `Hint.verb: &'static str` `graft-ui/src/graph_screen/hints.rs:71`; `resolver_list` verbs `"ours"`,`"theirs"`,`"both"` `:961`; card list `:989-990`. Runtime strings impossible (Q3: static only).

## Tests touching this
graft-ui `conflict_resolver/tests.rs` builds `ours_label: text("HEAD")` `:29,75,319`, `explainer` `:52,316,356`; graft `view/resolver.rs` tests `resolver_view_sanitises` `:434` (reads labels `:458`), `explainer_names_the_sides` `:478` — all break with the Signatures.

## Contradictions with the ticket
1. Crates say graft **`view::resolver` only** (ticket:8), but took-lines/notices use `side_word` in `crates/graft/src/update/resolver.rs:406-412`, and whole-file keys `"o keeps the file  t deletes it"` in `update/resolver/whole_file.rs:38` — scope 2 needs `update::resolver` too.
2. Scope 1 wants "every fixture file"; ticket names `StoppedMerge`/`StoppedRebase` — not checked here.
3. Detached HEAD: `current_branch` returns None, no sha fallback (`:254`) — new code, not a rename.

## Build with
Sonnet — rules fully specified, mechanical threading; but conductor must first widen crate scope for contradiction 1.
