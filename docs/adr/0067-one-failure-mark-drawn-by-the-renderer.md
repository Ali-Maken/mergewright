# ADR-0067: One failure mark, `⊗`, drawn by the renderer

Date: 2026-10-24 · Status: accepted (user: "yes")

## Context
Scout `.scratch/ease-pass/scouts/25q-failure-marks.md`: failure bars were inconsistent. `Status::Failed` drew no glyph
(`view/status.rs:108`), producers passed bare words, and only the setting write (ticket 25, `palette/setting.rs:89`) drew
`✗` — which the design already uses for "lost" (`docs/design/01-view-mockups.md:386`, `02-keymap.md:548`). The ruling
that put it there ("as every other failure bar", ease/06i) was wrong.

## Decision
- Every failure reads `⊗ <why>` (ASCII `(x) <why>`), the mark in red (the `removed` colour).
- One ui module (`failure`) draws it; producers pass words only; a test fails on a producer that types its own mark.
- `✗` stays "a commit no branch holds"; `!` attention, `⚠` warning, `ⓘ` info, `gfx:` graphics status. `⊗` and `(x)` are
  taken app-wide.

## Consequences
- The setting-failure mock line from 06i (`✗ Graph: … not kept`) reads `⊗` instead.
- New failure producers pass words only; new failure bodies call the `failure` module.
Source: `.scratch/ease-pass/issues/26-one-failure-mark.md`.
