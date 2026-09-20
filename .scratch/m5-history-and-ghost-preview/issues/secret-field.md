# secret-field: the typed password lives in a zeroizing buffer, never a `String`

Status: draft
Build with: (blank — brownfield; recommendation: **Sonnet**, mechanical once the scout has named every site the field's text passes through)
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

- [ ] Typing, backspace, `Ctrl+U`, paste into the secret prompt behave as the `TextField` did, judged by length and by `into_bytes` — `secret_field_edits_by_length`
- [ ] `Debug` of `SecretField`, `FieldView`, `CredentialPrompt`, `AppState` (with a prompt open) never contains the typed bytes — `secret_never_in_debug`
- [ ] The rendered frame shows exactly `len` mask glyphs and no character of the secret — `masked_prompt_draws_only_glyphs` (existing test adapted)
- [ ] `into_bytes` → `Answer` → `Secret` → git receives the password: the M4 walk's `password-prompt-*` scenarios pass unchanged — `m4_walk::every_scenario::*`
- [ ] No `String` or `Vec<u8>` copy of the secret exists outside `Zeroizing`/`Secret` on the path keypress → git: `rg -n "text\(\)|to_string\(\)|clone\(\)" crates/graft/src/update/credential.rs crates/graft-ui/src/text_input.rs` reviewed line by line in NOTES — inspection, listed
- [ ] `cargo deny`/`cargo tree` shows `zeroize` with no `unsafe` needed in graft-ui (`#![forbid(unsafe_code)]` still on) — build
- [ ] MAP.md updated; NOTES says whether a scout was used and what the zero-after-drop test could and could not prove

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-ui -p graft` · `cargo test --doc -p graft-ui`.

## Size

One context. Two crates, both edited by one builder (disjoint from every other M5 group: run first, alone).
