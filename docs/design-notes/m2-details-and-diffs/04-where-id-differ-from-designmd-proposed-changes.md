# M2 design pass: details and diffs (2026-09-15) — Where I'd differ from DESIGN.md (Proposed changes)

[See also: m2-details-and-diffs/README.md](README.md) for the full section index.
Previous: [Hard-to-reverse choices](03-hard-to-reverse-choices.md) · Next: [New UI](05-new-ui.md)

---

## Where I'd differ from DESIGN.md (Proposed changes)

| Change | Text | Trade-off |
|---|---|---|
| Markers are a paint-time lookup; `Row.marker` stays unused (remove in M8 pass) | DESIGN §8.2 "layout takes a `Map<CommitId, MistakeRadarKind>`", §3.1 `Row.marker` | strip/map see findings only for rows on hand (§5.3 already accepts this) |
| No column glyph for an unchecked merge; the pane's radar line says `evil merge: unchecked` → `checking…` → `clean`/finding | PROMPT §1.4 "the marker reads unchecked" | a `?` on every merge of git/git would be the loudest thing on screen |
| Word and syntax spans computed on sanitised text in `ui`; `git` keeps line pairing only | DESIGN §8.2 `WordDiff` in `git` | byte offsets survive sanitising (`�`/`␛` change lengths); ui gains `similar` + `syntect` |
| "Contained in" through the git binary | PROMPT Technology "gix for reads" | one spawn per selection, off-thread, cached |
| Map view draws text glyphs at every tier in M2; pixel curve deferred | DESIGN §5.1 tiers | a toggle the user reaches rarely; Unicode is the daily view |
| Map view rows are local branches only; a remote never collapses into its upstream's row; the merge elbow marks merges into the default branch only, at the tip's own bucket | DESIGN §5.1 "Shows" | one walk per local branch, no merge-commit search; remotes and cross-branch merges are M8 polish gap rows |
| No `[ Fix… ]` in the Explain panel until M5 | DESIGN §1.8 | nothing implies Fix exists |
| Keymap: `x` mark for compare; `Tab` = Graph→Files→Diff; `<`/`>` shrink/grow the pane on the focused divider (ADR-0028; was `Ctrl+arrows`, which macOS takes); `L` load the rest of a truncated diff; `Enter` opens the pane and drops the peek (ADR-0023) | DESIGN §2.2/2.3/2.5 | four new letters plus one bracket pair; `+`/`-` stay off (`+` unfolds lanes) |
| `Ctrl+R` after `B` rebinds the commit table and restarts the walk on the graph path, re-anchored on the selection; `Written` footer hints `Ctrl+R to use it`; no automatic restart | KNOWN_GAPS row 54 | the user asks; an unasked restart mid-scroll is worse than a hint |
| KNOWN_GAPS expiries: snapshot-cost row (52) and older-stash badges (175) → M3 (no staging or stash list exists in M2); `full_topological` residual (227) → not planned | KNOWN_GAPS.md | honest owners |
| **`?` help overlay (DESIGN §1.16) is not built in M2; no footer ever shows `? help` until it exists** | DESIGN §1.1–1.3 mockups' `? help` footer hint | keys off the footer (`Space` `1` `H` `x` `w W v [ ]` `<>`) are learned from README/TESTING.md until M8 (spec fix 2026-09-18, below) |
