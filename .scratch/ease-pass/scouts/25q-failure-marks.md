# 25q failure marks
- `Status::Failed(text)`: no glyph added by the renderer. `view/status.rs:108` (graph_problem) passes `sanitize_line(message)` as reason + dismiss keys. Colour/word only.
- Failed producers, all bare words: update/dispatch.rs:282 `"{id} failed: ..."`; remote.rs:471 refusal (why) / :492 busy.words(); fold_count.rs:96 `couldn't count ...`; facts.rs:106,128 failure text.
- Only `✗`-prefixed Failed: update/palette/setting.rs:89 `"✗ Graph: ... not kept: why"` (ticket 25).
- Other failure marks: graph gfx `gfx: <msg>` Warning ink (graph_screen/layout.rs:245); `⚠` Warning severity (graph_screen/render.rs:316); `! ` for warnings (commit_editor/words.rs:249, preview/line_rows.rs:132). Screen bodies (Stash/Journal/Map Failed) centred text: stash_screen.rs:376, journal_screen.rs:456, map_screen/render.rs:382 (glyph not checked).
- `✗` elsewhere means "lost/dropped" (journal pieces.rs:80, rewrite_editor), per design 01-view-mockups.md:386, 02-keymap.md:548. No design line says failure bars draw `✗`.
- Only source of the failure `✗`: 06i ticket line 32 + its mock (:137), one bar.
Verdict: inconsistent (bare words / `gfx:` / `⚠` / `!`). Design names no single failure mark. Ruling "as every other failure bar" was wrong: none draws ✗. `✗` is also the journal's "lost" glyph. Not checked: colour of Failed ink.
