# M1 design pass: the graph after the gallery walk (2026-09-13) — Where I'd differ from DESIGN.md / PROMPT.md (Proposed changes)

[See also: m1-the-graph/README.md](README.md) for the full section index.
Previous: [Hard-to-reverse choices (the three decisions)](03-hard-to-reverse-choices-the-three-decisions.md) · Next: [New UI](05-new-ui.md)

---

## Where I'd differ from DESIGN.md / PROMPT.md (Proposed changes)

| Change | Text it changes | Trade-off |
|---|---|---|
| Fold draws stubs + rail; count in header | DESIGN §3.5 bullets 3–4 ("single glyph carrying a count"; edges to the marker); PROMPT §1.2 "`+N` indicator" wording | `+N` no longer a per-row glyph to click; header count is farther from the graph |
| `graph.max_lanes` default auto by width | DESIGN §3.2 step 5, §3.5 "default 8"; config docs | Wide terminals draw up to 12 lanes; a cap that moves with resize |
| No hover tooltip; footer reveal; `Enter` peek; `y` copies SHA | PROMPT §1.3 "Hover shows a tooltip…"; DESIGN §2.15 hover line; §2.3 `Enter`; ADR-0020 superseded | No commit body/parents in Graft until M2 |
| Thread emphasis by alpha, not width (pixel) | none — aligns with DESIGN §5.2; reverses tickets 16/22's call | Thread relies on contrast; Focus mode's fade is the only stronger cue |
