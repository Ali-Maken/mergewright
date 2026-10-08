# 18c — `c` on the empty state: the clone prompt, its bar, the J15 walk

Status: done (builder 2026-10-08)
Build with: **Sonnet** — every decision is in R6 and 18b's state; drawing and keys follow `o`'s prompt and M4's
credential box, the walk follows `m4_walk`'s credential scenarios.
Blocked by: 18b
Needs: 18b's `Prompt::Clone(CloneState)`, `CloneRun`, `OutsideStep::{StartClone, StopClone, Credential, Cloned}`,
`clone_request`, `outside_job`, `prompt_key`/`prompt_paste`; 18a's `CloneError::explain` words
Crate set: **graft-ui** (`empty_state`), **graft** (`empty_state` keys and view, sweep tables, tests, capture script)

## Read first
- `.scratch/m8-polish-and-ship/issues/18-rulings.md` R6 (the landing table is the spec)
- `docs/design/01-view-mockups.md` §1.15; `docs/design/02-keymap.md` §2.13
- Design note `m8-polish-and-ship.md`: J15 (`clone 3`), "Where he lands" clone rows
- `crates/graft-ui/MAP.md` `empty_state` (`bar_keys`, `EmptyStateView`), `credential_prompt`, `hints` `Credential`
- `crates/graft/MAP.md` `lib.rs` `empty_state`, `update::tests::keymap_edge`; 18b's `## empty_state` lines
- `tests/it/m4_walk/` `credentials.rs`, `run.rs` (`askpass_helper`, `no_secret_leaked`)

## Scope
In: `c` (gated `!missing`, as `i`) opens the prompt; typing, paste, `⇥` none; `⏎` → `clone_request` → `StartClone`
or the reason; keys per R6 in each `CloneRun`; credential box over the empty state with the `Credential` bar;
header actions and bar gain `c clone…` (§1.15 order); `CloneView` drawing; sweep tables drop `c` from
not-yet-answered; walk tests; captures; MAP.md.
Out: the job host (18b); clone options; first-launch (18d).

## Signatures
```rust
// graft_ui::empty_state
pub enum EmptyPrompt { Open(FieldView), ConfirmInit, Clone(CloneView) }   // FieldView held as `Open` holds it
pub struct CloneView { pub url: FieldView, pub into: SanitizedText,        // `~/code/x`, or `~/code/…` while none
    pub shown: CloneShown }
pub enum CloneShown { Typing, Running { progress: Option<SanitizedText> }, Ended { reason: SanitizedText } }
// EmptyStateView gains `credential: Option<CredentialView>`: drawn last, over the screen, band cleared full width
// (as graft `view` does for the repo screen); bar = `Credential` context while `Some`.
```
Layout: the clone prompt takes the open prompt's slot; `→ <into>[ · <progress>]` on the row under it (muted path,
left-truncated `…/` when narrow — the tail is the part he checks); `Ended`'s reason on the next row in Problem ink;
RECENT moves down by the rows used. Bars: Typing `⏎ clone · esc cancel`; Running `esc cancels`; Ended
`⏎ retry · esc cancel`. Running: every key but `esc` and `⌃C` swallowed.

## Acceptance (each tick names its test, at tick time)
- [x] J15 `clone` ≤ 3 inputs (`c`, `Type`, `⏎`): `file://` bare → that repo's graph, cursor on HEAD; walk fails one
      key over. Use 08's empty-state walk driver if one exists (`rg -n 'J21|init_here' crates/graft/tests`); else
      drive `outside_edge` + `outside_job` with a real `JobRunner` and say so in NOTES.
      (`tests/it/m8_walk/clone.rs::clone_from_the_empty_state`)
- [x] HTTP Basic: credential box drawn over the empty state (frame has its title), `TypeSecret`, `⏎`, clone lands;
      secret absent from every event's `Debug` (`jobs::clone::tests::child_asks_through_the_pool_and_clones`) and
      from pty frames, stderr and the new `.git` (`m8_walk/clone.rs::clone_asks_for_a_password`).
      *Re-ticked 2026-10-08 by 18e A1, box amended: unticked earlier by the conductor as a false tick (review m8-clone
      C1), since it named `no_secret_leaked` over events and no test saw events. The event scan was seen red in 18e.*
- [x] R6 rows, words asserted from the drawn frame: no name; target non-empty (nothing ran); unreachable → fails
      row, URL kept, `⏎` retries; `HungRemote` + `esc` → stopped row, no folder; `esc` not running → list, nothing
      written. (`…::clone.rs::{bad_url_stays, target_exists_refused, failure_keeps_the_url_and_retries,
      esc_mid_clone_stops_and_keeps_the_url, esc_closes_the_prompt}`)
- [x] Missing folder: `c` neither drawn nor answered. (`graft::empty_state::tests::no_clone_in_a_missing_folder`)
- [x] Every key each clone sub-state's bar draws is answered (Typing, Running, Ended, credential).
      (`graft::empty_state::tests::empty_state_bar_keys_are_answered` — the real name; forms in NOTES)
- [x] `c` leaves `NOT_YET_ANSWERED` in `empty_state/sweep_tests.rs:74` and `update/tests/keymap_edge/sweep.rs:23`;
      `ACTING_AT_LANDING` grows, never shrinks. (`graft::empty_state::sweep_tests` + `update::tests::keymap_edge::sweep`, both green)
- [x] Widths 60, 80, 120: prompt rows never overlap the rule; `→` path left-truncated. 
      (`graft_ui::empty_state::tests::clone_prompt_fits_every_width`)
- [x] Captures (pty, `launches` = `done`): `docs/shots/m8/clone-prompt-120x36.txt` (URL typed, `→` line),
      `docs/shots/m8/clone-failed-120x36.txt`, `docs/shots/m8/clone-credential-120x36.txt`. **Colour unverified**.
      His to judge (rule 6). (written by `clone_asks_for_a_password`, `failure_keeps_the_url_and_retries`)

## Touches
- §1.15 header and bar line: now built in full except `t` (G-tour) — mock-up unchanged.
- Palette on the empty state: `empty_state.clone` row now runs (`palette_rows`); check `⏎` on it opens the prompt
  (test above covers via the sweep; NOTES says which).
- `bar_keys` callers and every empty-state capture from 08/11b (`rg -l 'no repository here' docs/shots`): bar gains
  `c clone…` — re-shoot those, list them in NOTES; no other line may change.
- Keymap §2.13 `c` row: matches; no keymap edit. MAP.md graft, graft-ui.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide.

## Size
One context, ~60k tokens.

## NOTES (builder, 2026-10-08)
- Scout: none new; `scouts/18-first-run.md` plus direct reads were enough.
- 18b glue observed on a real pty (`tests/it/m8_walk/clone.rs`): file:// clone lands on the graph, cursor on HEAD,
  recent list updated; HTTP credential box drawn, password typed, clone lands; stop/fail words on the prompt.
- Walk driver: pty (real binary), not outside_edge+JobRunner. Fence `clone 3` asserted.
- Secret check: password absent from pty output, stderr, and every file under the new `.git`. **Events-level
  `no_secret_leaked` unverified** — events are not observable from a pty.
- Bar sweep forms: list, no-recents, missing, open, init, clone typing, running, ended, credential.
  Palette: `palette_tests::the_clone_row_opens_the_clone_prompt`.
- Deviations: `EmptyStateView::credential: bool` not `Option<CredentialView>` (no Debug/PartialEq; graft draws the
  box via extracted `view::credential::prompt_view`); `CloneView` holds `url`+`cursor`, not FieldView; long URLs
  scroll so end+cursor stay visible.
- Re-shot (only actions + bar lines changed): `m8-look-outside-repo-120x36.txt`,
  `m8-look-outside-repo-no-recent-80x24.txt`, `m8-look-outside-repo-open-prompt-80x24.txt`,
  `m8/first-launch-empty-state-120x36.txt`.
- Gap (broken): open prompt's long path cuts its end, hiding the cursor — same fix as clone's `window()`, untouched.
- Question: 18a failure words for a missing file:// read "no remote named '/nonexistent/…'" — odd wording?
- Colour unverified (rule 5).
