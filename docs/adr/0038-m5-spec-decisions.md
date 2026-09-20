# ADR-0038 — M5 spec decisions (confirm order, the mask, two keys, re-push, ghost anchors)

Date: 2026-09-20. Decided by the user before M5's first group. Status: accepted.
Supersedes the M5 note's Proposed changes 1 and 6.

## 1. `[ Cancel ] [ Confirm ]`, cursor on Cancel — everywhere

Uniform across **every** confirm box. **The default must not depend on how dangerous
the action is**: a default that moves is a default the user stops reading. Written into
DESIGN §1.7.

## 2. The mask is a fixed 8 glyphs, and the field still answers the keyboard

The masked field draws 8 glyphs whatever the real length, so it cannot leak it.

**Condition:** a keystroke must still visibly register — a brief pulse, a caret, some
response — and **that response must not encode length**. A field that looks dead while
typing is not acceptable; neither is one whose feedback counts characters.

## 3. `M` opens the mergetool, `e` edits — one meaning each, everywhere

`e` already means "open in `$EDITOR`" in the graph pane. **It keeps that one meaning
everywhere**, including the conflict banner and the CONFLICTS row. `M` is free and sits
mnemonically beside `m` for merge.

Both go into `docs/design/02-keymap.md` **as part of the same change that binds them**,
because the keymap is now the single source of truth (ADR-0037 §1).

## 4. "Re-push old tip" moves origin back and keeps the local commits

The undo-after-push box's third choice puts the remote branch back where it was and
leaves the user's local commits alone.

**Conditions, all three:**
- `--force-with-lease` only. Never a bare force.
- An explicit confirm of its own.
- The box says plainly **that this changes the remote other people may have already
  pulled**, and is labelled so it cannot be mistaken for a local undo.

## 5. Ghost parents are clipped at the slice edge, as anchors

A preview never loads more history mid-preview; parents outside the laid-out slice are
clipped and drawn as anchors.

**Conditions:**
- **An anchor must look like an anchor, not like a real parent.** No ambiguity at any
  tier.
- If the part of the preview that matters falls outside the slice, **the preview says
  so** rather than drawing a truncated shape the user could misread.
