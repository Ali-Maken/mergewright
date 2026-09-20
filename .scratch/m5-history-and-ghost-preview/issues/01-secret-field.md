# secret-field: the typed password lives in a zeroizing buffer, never a `String`

Status: done
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
- **A keystroke must still visibly register** — a brief pulse, a caret, some response — and **that response must not encode length**. A field that looks dead while typing is not acceptable; neither is feedback that counts characters. **Met**: `SecretField.pulse: bool` flips on every edit; the drawn row swaps exactly one glyph (position 0) between `•` and `◦` (`*`/`+` at the ASCII tier) on each keystroke. Same test (`eight_glyphs_and_an_identical_pulse_for_any_length`) proves the *mechanism* — same column, same two glyphs swapped — is identical for a 3-char and a 30-char field. It deliberately does **not** assert the two fields' absolute drawn rows are equal at any instant: `pulse` is a toggle, so two fields with a different total edit count can be at opposite parities, and asserting equality there would itself be asserting something about length by the back door (see the test's own doc comment). What's proved is the mechanism, not the absolute state — that is what the condition actually requires.
- Acceptance must name the test that proves both: 8 glyphs for a 3-char and a 30-char secret, and a visible change on keypress that is identical for both — both proved by `graft_ui::text_input::tests::eight_glyphs_and_an_identical_pulse_for_any_length`, described above.

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
