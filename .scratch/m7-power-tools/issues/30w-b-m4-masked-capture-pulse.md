# m4-masked-capture-pulse — the masked-field captures stop comparing a coin flip

Status: ready
Build with: **Sonnet.** A harness script and two caption rows; the cause is read from two MAP lines and confirmed by
source lines and repeated runs before any change. No product code.
Needs: none (no crate code). Base: **26b's branch with `main` merged** — 26b split `m4-contact-sheet-captions.tsv`
rows 5/6 (26b NOTES "Other changes"); basing on `main` would conflict there. The conductor prepares the worktree.
Runs: concurrently with 30w (disjoint: no crate, no shared file).

## Why
26b NOTES, pty: `docs/shots/m4-03-credential-masked-80x24.txt` :14 committed `▏••••••••`, drawn `▏◦•••••••`; 3 reruns
clean. 26b filed it as a question on `.scratch/m4-remotes/pty-walk-m4.py:213-215` (`wait_quiet` ending on a pulse frame).
The planner's reading (docs only, **unverified** — V1/V2 check it): it is not a timing race.
- graft-ui MAP `text_input`: the masked row draws 8 glyphs, the first swapped for `◦` by the `pulse` bit, which flips on
  every edit from `pulse_seed`; **at rest the row is `seed XOR (edits mod 2)`** — by design (ADR-0038 §2), so it cannot
  encode the length.
- graft MAP `askpass`: `pulse_seed()` is drawn **at random per prompt** (`getrandom`) by the running app; "every test
  pins `pulse_seed`; the running app draws it". The capture runs the real loop (`m4_walk --capture`, script docstring).
- So the settled frame is a coin flip per run; `wait_quiet` settles correctly on either face. The caption's "the settled
  frame has none" (`m4-contact-sheet-captions.tsv`, the 160x48 row) is wrong for the same reason.

## Ruling (planner)
- **The harness normalises the one designed-random cell; the product is not touched.** For a capture whose stem
  contains `masked`, the script rewrites the field's 8-glyph run (`[•◦]{8}`, Unicode tier) to `••••••••` before the
  capture is written or compared, and adds the note line `(the field's first glyph is the keystroke pulse, random per
  prompt by design — ADR-0038 §2; drawn here as •)`. Same class as the port and temp-path redaction the writer already
  does: a value random by design, not a layout fact. Cause, at the harness: it compared a coin flip.
  - Rejected: a seed knob in the app (env or flag) for the capture. A product switch that fixes the pulse is exactly
    what ADR-0038 §2 forbids leaking; not worth it for a capture.
  - Rejected: a retry, or accepting either face by comparing twice (rule 2).
  - Cost: the capture no longer shows which face was drawn. The pulse itself is pinned by graft-ui's `SecretField`
    unit tests (name them in NOTES, by grep).

## Verify first (NOTES `### Verify`, each with the command and what it printed)
- **V1, the seed is random on the capture path.** The line in graft that draws `pulse_seed()` for a credential prompt
  (`jobs/askpass*`), and `grep -rn pulse_seed crates/graft/examples/` showing `m4_walk` pins nothing. A pin found →
  **stop**, `blocked: m4_walk pins the seed at <line>; the drift is elsewhere`.
- **V2, the drift is exactly that cell.** Run `python3 .scratch/m4-remotes/pty-walk-m4.py 80x24 <masked scenario>` 10×
  in compare mode, output to a file (`GRAFT_UPDATE_SHOTS` unset); count `DRIFTED`. Expect a mix. For each drift, the
  diff is the field's first glyph and nothing else. 0 of 10 with any `◦` → **stop**, the question stays open (say so).
  A drift in any other cell → **stop**, name it.

## Files (exact)
- `.scratch/m4-remotes/pty-walk-m4.py`: the normalisation and the note, in the masked branch beside the existing
  `"masked" in stem` check (:227); nothing else.
- `docs/shots/m4-03-credential-masked-80x24.txt`, `…-160x48.txt`: re-shot once under `GRAFT_UPDATE_SHOTS=1`; `git diff`
  shows only the added note line.
- `docs/shots/m4-contact-sheet-captions.tsv`: the two masked rows' parentheticals — drop "A run can catch the field's
  pulse … the settled frame has none"; say the first glyph is normalised (ADR-0038 §2).
- `docs/shots/m4-contact-sheet*.txt`: rebuilt by `scripts/contact-sheet.sh` (≤ 200 KB each).

## Acceptance (name the verifying command at tick time)
- [ ] V1, V2 in NOTES.
- [ ] Masked scenario 10× in compare mode after the change: 0 `DRIFTED` (the run file's count).
- [ ] The "no `•`" check (:227) and the password check still run on the **drawn** frame, before the rewrite: show the
      order in the diff.
- [ ] Every m4 capture at 80x24 and 160x48 in compare mode: all `settled`, none drifted.
- [ ] `grep -c '◦' docs/shots/m4-03-credential-masked-*.txt` = 0; each file's diff is one note line.

## Touches
- Other pty scripts with a masked capture (`grep -ln masked .scratch/*/pty-walk-*.py scripts/`): name each; one that
  captures the field too gets the same rewrite **only if** V2's reasoning holds for it — else NOTES, not fixed.
- `TESTING.md` Credential prompt (masked) row: "a field of `•` only" stays true; no edit (grep it).
- `scripts/m7-accept.sh` step 2 runs this script: its m4 run goes green by this; nothing in the script changes.
- No crate, no keymap, no product words.

## Gates
No Rust changed: `cargo fmt --check` only, as a guard. The 10× runs and the full m4 compare, output to files, summary
lines read.

## Size
~20k.
