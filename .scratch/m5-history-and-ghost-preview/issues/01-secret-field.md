# secret-field: the typed password lives in a zeroizing buffer, never a `String`

Status: done (fix round for `docs/reviews/m5-01-secret-field.md` landed — see FIX ROUND below)
Blocked by: none
Build with: (blank — brownfield; recommendation: **Sonnet**, mechanical once the scout has named every site the field's text passes through)
**Conductor set after the scouts: Sonnet.**
Crates: graft-ui, graft
Needs: none
**First group of M5 (ADR-0037 §5).** Nothing else starts before this is merged.

## Scout first (Haiku/Sonnet `Explore`, ≤ 1 page)

- `crates/graft-ui/src/text_input.rs`: where `TextField::text()` is read for a masked `FieldView`; what `display_text` does; what `Debug for FieldView` redacts.
- `crates/graft/src/update/credential.rs`, `view/credential.rs` (or wherever `credential_view` lives), `update/mod.rs` (the per-event `AppState` clone): every place `CredentialPrompt.field` is read, cloned or turned into `Answer`.
- `crates/graft/src/jobs/askpass.rs`: how `Answer::into_secret` reaches `Secret`.
- Report: does any test, capture script or log line read the field's text?

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Interfaces (G1 block only); Proposed change 6.
- `docs/adr/0037-m4-checkpoint-decisions.md` §5.
- `crates/graft-ui/MAP.md` §text_input, §credential_prompt; `crates/graft/MAP.md` §update::credential, `AppState::text_field_open`.
- `crates/graft-git/MAP.md` §run/askpass (`Secret`: the shape to match; do not move it).
- `KNOWN_GAPS.md` row "M4/06 review SECURITY" (the row this closes) and m4-07 review Minor 9.

## Scope

In, in order (one commit each):

1. **`graft-ui/src/text_input.rs`**: `SecretField` (signatures below). Backed by `zeroize::Zeroizing<Vec<char>>` (add `zeroize` as a graft-ui dependency; no `unsafe` — the crate forbids it). `Clone` allowed (the app clones `AppState` per event) — the clone is its own `Zeroizing`. `Debug` prints `SecretField(<len>)`. No `text()`, no `Display`, no `as_str`. `PartialEq`/`Eq` compare length only (so `Command`/`AppState` stay comparable) — document why. Editing ops mirror `TextField` (cursor is a char index; `insert_str` refuses control characters like `TextField` does).
2. **`FieldView`**: `masked: bool` → `secret: Option<usize>` (`Some(len)`); `FieldWidget` draws `len` mask glyphs (`•`, ASCII `*`), cursor at the same column, guide/counter suppressed. `display_text` never receives a secret's text because there is none to pass. Fixed-width mask (8 glyphs regardless of length) **only if the conductor says the user chose it** (Proposed change 6); default stays one glyph per char.
3. **`graft/src/update/credential.rs`**: `CredentialPrompt.field: PromptField` — `Secret(SecretField)` when `is_masked_prompt`, `Open(TextField)` otherwise. `credential_key`/`credential_paste` route to whichever; `Enter` → `Answer::new(secret.into_bytes().to_vec())`… no: `Answer::new` must take the `Zeroizing<Vec<u8>>` by value and move it into `Secret` without an intermediate plain `Vec` — change `Answer::new(bytes: Zeroizing<Vec<u8>>)` (graft-git's `Secret::new(Vec<u8>)` is fed by unwrapping at the last step; the wrapper is zeroed when dropped). `Ctrl+U` → `clear()`.
4. **`view::credential_view`**: builds `FieldView { secret: Some(field.len()), .. }` for the secret case; `text` empty.
5. Tests: (a) `SecretField` round-trips typing/backspace/cursor by length and by `into_bytes` (b) after `drop`, the backing allocation is zero — prove with `zeroize`'s own guarantee plus a test that a `Zeroizing<Vec<u8>>` from `into_bytes` is zero after drop via `std::mem::take`-free inspection (use `zeroize::Zeroize` on a clone and assert equality of the pattern — if impossible without unsafe, state it as unverifiable in NOTES and rely on the crate's contract) (c) `format!("{:?}", prompt)` never contains the typed text (d) the M4 walk's `no_secret_leaked` still passes; add the credential prompt's `Debug` output to what it scans.
6. MAP.md lines (graft-ui `text_input`, graft `update::credential`); KNOWN_GAPS row closes in the conductor's commit — say "closed by <hash>" in NOTES, do not edit KNOWN_GAPS.

Out: any change to `graft-git::Secret`; unifying the two secret types (M8 architecture pass); the username field (stays a `TextField`, it is asked in the open).

## Signatures

```rust
// graft-ui/src/text_input.rs
pub struct SecretField { /* Zeroizing<Vec<char>>, cursor: usize */ }
impl SecretField {
    pub fn new() -> Self;
    pub fn insert(&mut self, c: char); pub fn insert_str(&mut self, s: &str);
    pub fn backspace(&mut self); pub fn delete(&mut self);
    pub fn left(&mut self); pub fn right(&mut self); pub fn home(&mut self); pub fn end(&mut self);
    pub fn clear(&mut self);
    pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool; pub fn cursor(&self) -> usize;
    pub fn into_bytes(self) -> zeroize::Zeroizing<Vec<u8>>;
}
impl Clone for SecretField; impl fmt::Debug for SecretField /* `SecretField(<len>)` */;
impl PartialEq for SecretField /* by len */; impl Eq for SecretField;
pub struct FieldView { …, pub secret: Option<usize> }   // replaces `masked`
// graft/src/update/credential.rs
pub enum PromptField { Open(TextField), Secret(SecretField) }
pub struct CredentialPrompt { pub title: SanitizedText, pub prompt: SanitizedText, pub field: PromptField, pub job: JobId }
impl Answer { pub fn new(bytes: zeroize::Zeroizing<Vec<u8>>) -> Self; }
```

## Acceptance (every tick names its test)

- [x] Typing, backspace, `Ctrl+U`, paste into the secret prompt behave as the `TextField` did, judged by length and by `into_bytes` — `graft_ui::text_input::tests::secret_field_edits_by_length`, `graft::update::credential::tests::ctrl_u_clears_the_field`, `graft::update::credential::tests::editing_keys_work_and_paste_lands_in_the_field`
- [x] `Debug` of `SecretField`, `FieldView`, `CredentialPrompt`, `AppState` (with a prompt open) never contains the typed bytes — `graft_ui::text_input::tests::secret_field_debug_and_eq_never_compare_content` (widget-layer redaction), `graft::update::credential::tests::secret_never_in_debug` (app layer: state, field, view, and the `Command` an `Enter` produces, all scanned for the literal typed password)
- [x] **Nothing about a `Debug` print, a log line or a panic message tells you how long the password is** (fix round, review Major 1 — it printed `Secret(SecretField(7))`). `SecretField`'s `Debug` is now one constant string for every field: two fields holding 7 and 30 characters, and an empty one, all print the same bytes, and the print contains no digit at all — `graft_ui::text_input::tests::secret_field_debug_and_eq_never_compare_content` (asserts the exact string `SecretField(..)`, asserts no numeric character, and asserts a 30-char field and an empty field print identically); at the app layer, `graft::update::credential::tests::secret_never_in_debug` (prints the live `PromptField` for `hunter2` and for `a-much-longer-passphrase` and asserts the two prints are equal and digit-free)
- [x] **A `{state:?}` taken on a real run, while a credential prompt is open, contains no password** — scope item 5(d), which the first round silently dropped (review Major 3). `m4_walk`'s `Recorded` now keeps one `{state:?}` per state the loop passed through with `state.credential.is_some()`, and `no_secret_leaked` scans those alongside the journal, events and status lines; a scenario that types a secret but records no prompt state is a failure, so the scan cannot pass by never running — `m4_walk::every_scenario::wrong_password_then_right`, `m4_walk::every_scenario::password_prompt_esc_cancels`, `m4_walk::every_scenario::username_then_password` (all three type a secret, all three pass with the non-empty guard in force). Honest boundary: `CredentialPrompt`'s own `Debug` omits the field entirely, so this walk proves nothing walks around *that* omission; the field's own redaction is proved by `secret_never_in_debug`, which prints the field directly.
- [x] The rendered frame shows a **fixed 8** mask glyphs, not `len` — deviation from the draft signature (`Option<usize>`) to the ADR-0038 §2 binding condition below; see NOTES — `graft_ui::text_input::tests::masked_prompt_draws_only_glyphs`, `graft_ui::text_input::tests::masked_field_at_the_ascii_tier_draws_stars`, `graft_ui::credential_prompt::tests::masked_field_at_the_ascii_tier_draws_stars`, `graft::view::tests::the_credential_prompt_draws_over_everything_and_masks_what_is_typed`
- [x] `into_bytes` → `Answer` → `Secret` → git receives the password: the M4 walk's `password-prompt-*` scenarios pass unchanged — `m4_walk::every_scenario::wrong_password_then_right`, `m4_walk::every_scenario::password_prompt_esc_cancels`, `m4_walk::every_scenario::username_then_password` (all in `cargo nextest run --workspace`, full 1834/1834 pass, see NOTES)
- [x] No `String` or `Vec<u8>` copy of the secret exists outside `Zeroizing`/`Secret` on the path keypress → git: `rg -n 'text\(\)|to_string\(\)|clone\(\)' crates/graft/src/update/credential.rs crates/graft-ui/src/text_input.rs` reviewed line by line in NOTES — inspection, listed
- [x] `cargo tree -i zeroize --workspace` shows only `graft` and `graft-ui` depend on it, and both keep `#![forbid(unsafe_code)]` (build succeeds under the forbid, so `zeroize`'s volatile-write internals never require `unsafe` in either caller crate) — build, `cargo tree -i zeroize --workspace` + `grep -n forbid(unsafe_code) crates/graft-ui/src/lib.rs crates/graft/src/lib.rs`
- [x] MAP.md updated; NOTES says whether a scout was used and what the zero-after-drop test could and could not prove

## Gates

Ticket says `-p graft-ui -p graft`; the worktree's own `CLAUDE.md` overrides this — **"Run them workspace-wide, never `-p`"** (a crate-scoped gate hid two app-crate breaks in M4 group 03). Ran workspace-wide, per that rule: `cargo fmt --check` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo nextest run --workspace` (1834/1834 passed, 23 skipped) · `cargo test --doc --workspace`. All four clean. Noted here as a deviation from the ticket's own line, per the worktree CLAUDE.md's authority.

## Size

One context. Two crates, both edited by one builder (disjoint from every other M5 group: run first, alone).

## Conductor: user conditions (ADR-0038 §2) — binding

- The masked field draws **a fixed 8 glyphs** whatever the real length. Length must not be inferable from the drawn field, its width, a counter, the cursor column or `Debug`. **Met**: `FieldView.secret: Option<bool>` (a pulse flag, not `Option<usize>`/`Option<len>` as the draft signature sketched — deliberately, so no future caller can read a length back out of the type at all) drives `display_text` to always emit `SECRET_GLYPHS = 8` glyphs; proved by `graft_ui::text_input::tests::eight_glyphs_and_an_identical_pulse_for_any_length` (3-char vs 30-char secret, both draw 8 glyphs before and after a keystroke).
- **A keystroke must still visibly register** — a brief pulse, a caret, some response — and **that response must not encode length**. A field that looks dead while typing is not acceptable; neither is feedback that counts characters.
  - **Second half met; first half only partly, and the shortfall is the honest answer, not a shortcut.** First round: `pulse` was a `bool` flipped by every edit. That gave per-keystroke feedback but left the field **resting at the parity of the edit count** — for a straight-typed password, one bit of its length, visible on screen and already committed in `docs/shots/m4-credential-prompt-in-app.txt:38` (review Major 2). That fails the second half of the condition outright.
  - Now: `pulse()` is `!is_empty()` — "something is typed in this field". **Nothing on the masked row varies with length, at any moment**, proved by `graft_ui::text_input::tests::eight_glyphs_and_an_identical_row_for_any_length`, which asserts a 3-character and a 30-character field draw the *same row cell for cell*, before a further keystroke and after it (the first-round test deliberately could not make that comparison, because the parity made it false).
  - **What the user loses:** the first character lights one fixed glyph (`•`→`◦`, `*`→`+` at the ASCII tier) and `⌃U` puts it out; the 2nd..nth keystroke changes nothing on screen — proved, and its limits stated, by `graft_ui::text_input::tests::the_first_keystroke_and_ctrl_u_visibly_register`.
  - **Why not both:** no deterministic function of the buffer can change on every keystroke and be constant at rest, because every row a field passes through while typing is also the resting row of a shorter password. Per-keystroke feedback needs something the password does not determine. The review's option (b) (a flag set on edit, cleared by any other `AppEvent`) does not supply it: during a blocked askpass no other events arrive, so (b) rests lit after any edit — exactly what is shipped, without touching `update/mod.rs`. The review's option (a) (a random starting parity carried on `Progress::CredentialNeeded` from `jobs/askpass.rs`) does supply it and leaks nothing, but it needs `jobs/askpass.rs` and `update/mod.rs`, both outside this fix round's file fence. **Handed back to the conductor as a gap** — see FIX ROUND below.
- Acceptance must name the test that proves both: 8 glyphs for a 3-char and a 30-char secret, and a visible change on keypress that is identical for both — `graft_ui::text_input::tests::eight_glyphs_and_an_identical_row_for_any_length` (8 glyphs, and the same row, for 3 vs 30) and `graft_ui::text_input::tests::the_first_keystroke_and_ctrl_u_visibly_register` (the visible change: the same column, the same two glyphs, at 1 character and at 30) — with the shortfall above stated, not papered over.
- **Never freed unwiped.** First round failed this for any password over 8 characters: the `Zeroizing<String>` started at capacity 0 and `String::insert` grew it 0→8→16→32, each reallocation freeing the old block with the prefix still in it (review Critical 1). Now the buffer is reserved once at 256 bytes and never reallocates on its own: growth past it builds a fresh `Zeroizing` and drops the old wrapper (whose whole capacity zeroes), `backspace`/`delete` shrink in place and then overwrite the bytes past the new end, and `clear` zeroes the capacity in place and keeps the allocation — `graft_ui::text_input::tests::secret_field_never_reallocates_silently` (types 256 characters asserting the capacity never moves, then asserts growth past it, then that backspace/delete/clear/`Clone` leave it alone). **Not verified, and not verifiable here:** that a freed or shrunk-past block actually reads as zero. A `#![forbid(unsafe_code)]` crate cannot look at memory it no longer owns; that half rests on zeroize 1.9.0's `Zeroize for Vec` contract ("Ensures the entire capacity of the `Vec` is zeroed"), read in its source, not on anything observed here.

## NOTES

- **Scout used**: yes. Haiku/Sonnet `Explore`, brownfield rule, before any edit. It located every site `TextField::text()`/`CredentialPrompt.field`/`Answer::into_secret` were read, cloned or logged, confirming no test/capture/log reads the field's text outside what's listed in the scope section.
- **Deviations from the draft signatures** (both justified above, not silent):
  - `SecretField` backed by `Zeroizing<String>`, not `Zeroizing<Vec<char>>` — `zeroize` implements `Zeroize` for `String` natively (confirmed by reading its vendored source: `volatile_write`/`volatile_set` + an optimisation barrier, no `unsafe` needed in this crate), and it reuses `TextField`'s existing char-index helpers (`byte_of`/`typable`) instead of a hand-rolled char-vector.
  - `FieldView.secret: Option<bool>` (a pulse flag), not `Option<usize>` — the ADR-0038 §2 binding condition (fixed 8 glyphs, no length signal anywhere) landed after the draft signature was written; a length-carrying `Option<usize>` would have been exactly the leak the condition rules out, so the type itself now makes that leak impossible rather than relying on every future caller to not read `.len()`.
- **Zero-after-drop, what was and wasn't proved**: `zeroize`'s own `Drop` impl for `Zeroizing<T>` is not independently re-tested here — verifying memory is actually zero after drop without `unsafe` (to read freed/moved-from memory) isn't possible from safe Rust, and the ticket's own escape hatch says to state that as unverifiable rather than fake a proof. What **is** verified: (a) direct inspection of `zeroize`'s vendored source shows real `volatile_write`+barrier, not a plain assignment liable to dead-store elimination; (b) `SecretField::into_bytes` and `Answer::new` both move the buffer out via `std::mem::take` rather than clone, so the only value left in place after handoff is `Zeroizing<String>::default()` — already empty, nothing left to zero; (c) `#![forbid(unsafe_code)]` holds in both crates, so nothing in `graft`/`graft-ui` can defeat the zeroing by reaching around it. The actual zeroing-on-drop behaviour is **unverified** (relies on the crate's contract, per CLAUDE.md rule 5's spirit — a capture/test can't observe freed memory) — marked apart from broken: nothing here is known broken, this specific claim is simply outside what a safe-Rust test can observe.
- **`rg` leak-path review** (`text()`/`to_string()`/`clone()` in `crates/graft/src/update/credential.rs` and `crates/graft-ui/src/text_input.rs`): two hits, both benign — `credential.rs:129` is `PromptField::Open(field) => field.text().as_bytes().to_vec()`, the **username** field's own text (never masked, never a secret — usernames are asked in the open by design); `text_input.rs:910` is `marker.to_string()` in `draw_field_label`, the focus-marker glyph (`▸`/space), unrelated to any field's content. No hit touches `SecretField`'s backing buffer.
- KNOWN_GAPS row "M4/06 review SECURITY" — closed by this ticket; NOTES says so, KNOWN_GAPS.md itself untouched (conductor commits the close).
- Nextest caught one genuinely flaky, unrelated test during the gate run: `graft-fixtures::remote::tests::http_remote_frees_its_port_on_drop` failed once in the full workspace run (port still accepting right after drop — a timing race in that fixture, not part of this ticket's crate set), passed clean in isolation and on a full re-run. Not touched, not silenced — flagged here per CLAUDE.md rule 1 (green gates are not evidence; say what happened).
- Two `docs/shots/` captures regenerated by hand via their existing `#[ignore]`d tests (both now show a fixed 8 mask glyphs for "hunter2" instead of the old 7): `docs/shots/m4-credential-prompt-and-hints.txt` (`cargo test -p graft-ui --lib credential_prompt::tests::capture_for_docs_shots -- --ignored --nocapture`, output pasted in by hand per the file's existing convention) and `docs/shots/m4-credential-prompt-in-app.txt` (`cargo test -p graft --lib view::tests::capture_m4_credential_prompt_in_app -- --ignored --nocapture`, self-writing). Both ≤200 KB (6.5 KB and 8.2 KB). Out of scope, not regenerated: the `m4-03-credential-masked-*`/`m4-04-credential-username-*` captures and stems, which come from the `m4_walk` example's live `--capture` mode (real HTTP remote, pty-driven) rather than a unit test — those still show the old 7-glyph mask for a real password and will need a manual pty-capture pass; flagging for the conductor rather than silently leaving it unmentioned.

## FIX ROUND (`docs/reviews/m5-01-secret-field.md`)

Handed to a second builder after the first was killed mid-work by a session
rate limit; the first builder's code changes were on disk, uncommitted and
unverified, and this round verified them rather than trusting them. Gates
were run here for the first time. Scout: none — the review names every file
and line, so a scout would have re-derived what was already written down.

What was addressed, and whether it was a cause fix or a symptom fix:

1. **Critical 1 — the zeroizing buffer leaked on growth. Cause.**
   `crates/graft-ui/src/text_input.rs`: `SECRET_CAPACITY = 256` reserved in
   `new()`; `make_room` grows by building a fresh `Zeroizing` and assigning
   (the old wrapper is dropped while it still owns the block, which is the
   only arrangement where zeroize's `Drop` can reach it); `wipe_after_end`
   overwrites the spare capacity after `backspace`/`delete`; `clear` calls
   `Zeroize::zeroize` in place instead of replacing the buffer; `Clone`
   carries the capacity over so a clone does not reallocate on its next
   keystroke. The type's doc comment was rewritten to state that rule
   instead of the false "every mutator replaces `text` wholesale" — and to
   say plainly which half is unverifiable. Test:
   `secret_field_never_reallocates_silently`. Cost to the user: 256 bytes
   per open prompt, and a `backspace` writes up to 256 NUL bytes.
2. **Major 1 — `Debug` printed the length. Cause.** Now the constant
   `SecretField(..)`. Tests named in the Acceptance tick above.
3. **Major 2 — the pulse's resting state encoded length. Cause, with a
   disclosed loss of function.** `pulse()` is `!is_empty()`. The full
   argument, the cost and the two rejected alternatives are in the
   conductor-conditions block above. Captures: both regenerated and
   compared byte for byte (below).
4. **Major 3 — the `no_secret_leaked` scan (scope 5(d)) was missing and
   undisclosed. Cause.** `crates/graft/tests/it/m4_walk/run.rs`:
   `Recorded.prompts` + the guard that a scenario typing a secret must have
   recorded at least one open-prompt state. Tick above.
5. **Major 4 — the per-event `AppState` clone. Not fixed; documented
   accurately and handed back.** `update` takes `&AppState` and returns a
   new one, so while a prompt is open the typed secret is copied once per
   event — every keystroke, every job progress message, every resize — with
   two copies live for the length of the call. Each is a `SecretField`, so
   each zeroes when it drops and none is freed unwiped. Removing it needs
   `update/mod.rs`'s own shape to change, outside this fix round's fence.
   The module doc of `crates/graft/src/update/credential.rs` and
   `SecretField`'s own doc now both say it happens, in those words, so no
   comment implies otherwise.

Also corrected in passing, inside the fence: `CredentialPrompt`'s `Debug`
doc still said `SecretField`'s `Debug` "prints only a length" (stale after
fix 2), and it now also records why the walk-level `{state:?}` scan cannot
by itself prove the field's redaction.

### Captures

- `docs/shots/m4-credential-prompt-in-app.txt` — regenerated
  (`cargo test -p graft --lib view::tests::capture_m4_credential_prompt_in_app
  -- --ignored --nocapture`, the test writes the file). **Byte-identical to
  the committed version**, confirmed by `git status` reporting no change:
  `hunter2` typed drew `◦•••••••` under the old toggle (parity of 7 edits =
  odd) and draws `◦•••••••` under the new rule (something is typed). The
  glyph is the same; what it now means is different, and it no longer
  differs between a 7- and an 8-character password.
- `docs/shots/m4-credential-prompt-and-hints.txt` — the widget body
  regenerated (`cargo test -p graft-ui --lib
  credential_prompt::tests::capture_for_docs_shots -- --ignored --nocapture`)
  and compared line by line: **unchanged**. Its caption was wrong and is
  fixed (review Minor 7): it claimed `hunter2` was typed into a view built
  with `secret: Some(false)`, which under the new rule means *nothing*
  typed, so the state it described could not draw that row. The caption now
  says what the capture actually is, points at the in-app capture for the
  typed state, and names the test that proves eight-for-any-length, since a
  single capture cannot.
- Both files are well under the 200 KB cap (8.2 KB, 7.0 KB). Per CLAUDE.md
  rule 5, neither proves colour; per rule 6, both still want the user's eye.

### Gates (this fix round, in the worktree, run workspace-wide)

`cargo fmt --check` clean · `cargo clippy --all-targets --message-format=short
-- -D warnings` clean · `cargo nextest run` **1848 tests run: 1848 passed, 23
skipped** · `cargo test --doc` clean. The worktree branches from `4da442b`,
before the M5 group 02 fix merged, so 1848 here against main's 1852 is the
base difference, not a loss — the four extra tests on main are group 02's.
Not run here: heavy tier, release perf, fuzz, bench.

### Handed back to the conductor

- **Per-keystroke feedback on the masked row is gone** (only the first
  character and `⌃U` change it). Restoring it without leaking length needs
  the review's option (a): a random starting parity carried on
  `Progress::CredentialNeeded` from `jobs/askpass.rs`, XORed with the edit
  count. Touches `crates/graft/src/jobs/askpass.rs` and
  `crates/graft/src/update/mod.rs` — outside this fence. **Broken? No.
  Unverified? No. A disclosed loss of function**, traded for closing a real
  length leak.
- **The per-event `AppState` clone of the secret stays** (item 5). Belongs
  in `docs/gaps/closed.md`/ADR-0038 as a decision, per the review's own
  wording, not as an open defect.
- **Review Minor 8 is still open**: the `m4-03-credential-masked-*` /
  `m4-04-credential-username-*` captures come from the `m4_walk` example's
  live pty `--capture` mode and still show the old 7-glyph mask. Not
  regenerable from a unit test, not touched here; the M5 acceptance group's
  contact sheet will otherwise concatenate stale captures.
- **Zero-after-drop remains unverified** (not broken): unobservable from
  `#![forbid(unsafe_code)]`. It rests on zeroize 1.9.0's `Zeroize for Vec`
  contract, which the review and this round both read in the vendored
  source.
