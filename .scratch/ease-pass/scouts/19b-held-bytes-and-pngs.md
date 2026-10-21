# 19b scout: held-frame bytes, two PNGs

## Q1 held_key bytes_per_frame_held
- Budget line: held_key.rs:330-331 (`<= 512 * 1024`, Pixel only, 325). Test's number; added in 844c1e5b (m7-42, 2026-10-01). No `512 KiB` in docs/design or docs/adr (grep). Design source: not found.
- Metric is bytes / frames_while_held. docs/perf/16-heavy.txt:63-65: frames_while_held=3/27/9, keys=188/188/189, bytes_per_key=103923/289877/117537, strip_transmits_held=10/40/17, emphasis_transmits_held=24/68/32, deflated_transmits=0.
- Earlier records: docs/perf/m8-settle.txt:7-9 has frames_while_held=0, bytes_per_frame_held=0, bytes_per_key=328303, verdict "ok" (vacuous: zero frames). Other records: 13113157 (47-scroll-acceptance etc.), 22837655 (one record). So a green Pixel per-frame result: only with 0 frames. Never genuinely under 512 KiB in perf records seen.
- bytes_per_key now (104k-290k) is not above m8's 328k. Per-frame figure is large because paced loop coalesces ~60 keys into 3 frames; not a re-send growth per key. Whether strips are re-transmitted vs placed by id: strip_transmits_held=10-40 for 188 rows, so strips mostly placed; emphasis transmits 24-68. Per-frame emit path not read; cause "not found" beyond the above.
- ease-pass-13 role strokes (bcc58b40, 9450a77a, 2026-10-19) changed pixel drawing; no pre/post held_key run found. Likeliest, unconfirmed.

## Q2 PNGs
- ghostty_shot.rs:77 compares to docs/shots/m7/fix-scroll-ghostty.png; last changed ea4b9b36 (m8-05, 2026-10-11; Place.stack/z). Before: 48f3041a (m7-48).
- pieces_shot.rs:60 -> docs/shots/m8-05-emphasis-pieces.png; last changed ea4b9b36.
- Drawn output changed after that: ease-pass-07 f2269602, 13 bcc58b40 + 9450a77a (2026-10-18/19; lanes by role, tall pixel row, strips shift). Not decoded or run: whether 1720 B is blank NOT confirmed. Neither PNG re-shot since 10-11.
