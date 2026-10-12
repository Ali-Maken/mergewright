# Scout: esc clauses in bars while a write runs

## Gate
- `update/keys.rs:262-268` Esc → `cancel_operating` before `pane_and_overlay_key` (:283).
- `update/remote/in_flight.rs:19-28` returns false when `operating` None, `text_field_open()`, `confirm`, `context_menu`.
- `text_field_open` `update/state.rs:950-965`: search_prompt, palette, place prompt, J find typing, commit_editor, stash.prompt, credential, name_prompt, picker shown, rewrite_editor field, finish with focus != Files.
- Commit editor while operating: its own Esc calls `stop_write` (`update/commit_editor/keys.rs:22-28`). Its bar: `view/commit_editor.rs:20` already reads `busy = state.operating.is_some()`.
- Writes are claimed only in `operate.rs:277-289 claim_write`; I found no overlay-closing on claim.

## Bars offering esc (clause site) / preempted by cancel?
| Bar | esc clause | Preempted? |
|---|---|---|
| J | `graft-ui/src/journal_screen.rs:252` | yes (not a field unless find typing) |
| Stash list | `graft-ui/src/stash_screen.rs:122,128` | yes (prompt excepted) |
| Map | `map_screen/render/footer.rs:23` (Loading `esc cancels`), `:62` | yes |
| Blame/history place | `blame.rs:249`, `history.rs:358` | yes (unless place prompt) |
| Resolver | `conflict_resolver.rs:284`; hints `graph_screen/hints/lists/resolver.rs:67-125` | yes |
| Finish summary | `finish_summary.rs:187` | only when focus == Files |
| Graph focus/BACK hints | `graph_screen/mod.rs:327`, `hints/keys.rs:24`; FIELD_ESC `:131` | BACK yes; FIELD_ESC no |
| Preview box | `preview_box/footer.rs:34,42,86` | yes (preview not in the guard) |
| Palette | `palette.rs:244` | no (field) |
| Picker | `picker.rs:277` | no (field) |
| Commit editor | `commit_editor.rs:184,201` | no (field; own stop_write) |
| Credential | `credential_prompt.rs:60` | no (field) |
| Empty state | `empty_state/bar.rs:30-52` (`Running` → `esc cancels` :51) | could not confirm if clone uses `operating` |

## Open while a write runs?
- J/stash/Map/blame/history: no `operating` check found on open (grep of `operating` in update/ hits none of them). Could not confirm each open path line-by-line.
- Stash put-back from preview: launched with `launch` (`preview/answer.rs:238-239`); stash list not closed there. Recover put-back closes J on launch (`:254-256`). `RevertTo` from J (`:259-262`): could not confirm J closes.
- Resolver: refuses to open while operating (`resolver/open.rs:97-104`, says `esc stops it`), but stays open while its own staging write runs (`resolver/resolved.rs:64-66`, `whole_file.rs:114-118`); finish commit runs with summary open (`finish/keys.rs:266-268`).
- Preview: confirm closes it; `modal_owns_bottom_row` includes preview (`state.rs:1009-1014`).

## Data/flags drawers see
- No shared "write running" flag in graft-ui views; only `view/commit_editor.rs:20`, resolver `busy_words` (`update/resolver.rs:277`).
- `JournalView` built in `view/journal.rs:37-71`; `status_row` filled in `view/frame/overlays.rs:26` via `journal_status_row` (:56-77) = problem else `footer_notice`. So J DOES draw `push … · esc cancels` in its status row.
- `footer_notice` `view/frame/bars.rs:290-307`: None under credential/name/confirm/preview; Busy → the description (with esc hint) or `…`. Graph uses it at `view/frame.rs:141`; history place via `draw_status_or_hints` (overlays.rs history fn).
- Stash (`overlays.rs:86-109`) and Map: no footer_notice call found → could not confirm a status row under them.
