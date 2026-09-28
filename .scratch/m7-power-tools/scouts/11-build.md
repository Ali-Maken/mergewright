# Scout 11 cherry-pick-key (main HEAD 00243c98)

## graft-ops (11a, done)
- `check_pick` operation/history.rs:595: refuses only `NoCommits` + moved HEAD (`head_is`). Does not refuse HEAD/ancestor/merge itself.
- Those refusals live in `PickParams::check_commit` history.rs:149-160 (on_head -> `AlreadyOnBranch`, parents>=2 -> `PickMerge`). Called by preview/kinds.rs:133 and apply.rs:508 (both read git). Refusal variants operation.rs:564/572, Display :949/:952. Answers KNOWN_GAPS :806: preview + apply refuse; the app/menu does not.
- `sets_aside` operation.rs:1329-1331: `CherryPick` already `true` (unconditional, not "touched paths only" as the Signatures line says).
- Title source: `describe_pick` history.rs:351-356 -> `cherry-pick <sha7> "<subj>"`; preview title `Preview: {describe}` graft/update/preview.rs:997. No `take ... onto <branch>` wording anywhere.

## graft (app)
- `o` on the graph: unbound. Only `o` uses: resolver (update/resolver/take.rs:27,257), strip test (strip.rs:69). Keymap 02-keymap.md:205 gives `o` = take onto branch (§2.3), :165-166 slot 2 carries key and dims.
- History keys: `HistoryKey{Merge,Rebase}` update/preview.rs:1410-1427, `history_key` :1436, routed from update/keys.rs:335; place mode history_keys at update/place/history_keys.rs:24. No `o` there.
- Slot 2: `pick_verb` update/context_menu/verbs.rs:119-128, label `Cherry-pick <sha7>`, no `(o)`; comment verbs.rs ~:140 says cherry-pick "own no letter" (now stale vs keymap).
- Slots: update/context_menu/slots.rs:47-105. Only one dim mechanism: `dim_first: Option<(label, reason)>` = slot 1 (`Checkout — already on <b>`, :88-95). Pick is always pushed live (:98). No way to dim slot 2 today.
- graft-ui context_menu.rs:130-175 draws `dim_first` then `history[..leading]`; a second dimmed item needs a graft-ui change.
- Confirm bar: graft-ui preview_box.rs:55 hardcodes `y confirm`; ticket wants `y take it` — contradiction, graft-ui edit.

## Contradictions
1. Title/bar wording (`take ... onto`, `y take it`) need graft-ops history.rs:353 (held) or app-side override, and graft-ui preview_box.rs:55.
2. `sets_aside` is already unconditional for CherryPick (operation.rs:1331).
3. Slot-2 dim has no carrier; graft-ui ContextMenu API change.
Could not confirm: hint-bar catalogue lines for `o` (graft-ui graph_screen/hints/catalogue.rs not read).

## Files the build edits
- graft: update/preview.rs, update/keys.rs, update/place/history_keys.rs(?), update/context_menu/{slots,verbs}.rs, update/context_menu.rs, update/tests.rs, context_menu/verbs/tests.rs, tests m7_walk, MAP.md
- graft-ui: context_menu.rs, preview_box.rs, graph_screen/hints/catalogue.rs
- graft-ops (only if title changes there): operation/history.rs (HELD)
- docs: TESTING.md, docs/shots/m7/*, captures showing slot-2 label
