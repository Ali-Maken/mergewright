# commit-what-goes-in — "nothing picked means everything goes in", secrets left out, one undoable write

Status: draft
Build with: (blank — brownfield; conductor sets after the scout)
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: Journeys (commit-*), Mock behaviours C4–C7, C13–C14, Prior art rows "Commit start", "In vs out", "Secrets file".
- `docs/research/commit-stash.md` §1.4–1.5.
- `crates/graft-ops/MAP.md`: `CommitParams`, the commit operation, journal/undo entries. `crates/graft-git/MAP.md`: `WorkingTreeStatus`, `StatusEntry`.
- The scout's file.

## Scout questions
- How today's commit writes (index used, hooks, sign-off), what its journal entry records, and how `u` restores it.
- Every caller of `CommitParams` (commit editor, amend, reword, fixup paths?).
- How a partially staged file is represented in `StatusEntry`.

## Scope
In (graft-ops; graft-git only if a read is missing — then stop and report `blocked: needs graft-git <signature>`):
- `Include` on `CommitParams`; `first_choice` (pure); `looks_like_secrets` (pure).
- The commit writes **only the chosen paths**, in one journaled operation. Out-files' index and working state unchanged. `u` restores HEAD **and** the index exactly as before.
- Refusal `NothingChosen` when `Include::Chosen` is empty and `allow_empty` is off.
Out: the commit screen (S); keys (V).
Recommended approach: build the commit's tree in a scratch index (HEAD + chosen parts), commit, then update the real index for the chosen paths only. Builder may differ if the scout shows a cleaner path; say why.

## Signatures
```rust
pub enum Include { Staged, Chosen(Vec<Chosen>) }
pub struct Chosen { pub path: PathBuf, pub part: Part }
/// The note's `Take`; renamed so it never meets `graft_git::Take` in one file.
pub enum Part { Whole, StagedPart }
pub struct CommitParams { pub message: Vec<u8>, pub sign_off: bool, pub no_verify: bool, pub allow_empty: bool, pub include: Include }
                                     // existing fields keep their types; `include` is added
pub enum OutWhy { LooksLikeSecrets, NotStaged }
pub struct FileChoice { pub path: PathBuf, pub goes_in: bool, pub part: Part, pub why_out: Option<OutWhy> }
/// Nothing staged → every change goes in except new untracked files where
/// `looks_like_secrets`; something staged → only staged files (`StagedPart`), rest `NotStaged`.
pub fn first_choice(status: &WorkingTreeStatus) -> Vec<FileChoice>;
/// Case-insensitive basename: `.env`, `.env.*` (not `.example`/`.sample`/`.template`), `*.pem`, `*.key`,
/// `*.p12`, `*.pfx`, `id_rsa`/`id_dsa`/`id_ecdsa`/`id_ed25519` (not `.pub`), `.npmrc`, `.pypirc`, `.netrc`,
/// `credentials.json`, `*.keystore`, `*.jks`, `secrets.*`, `.htpasswd`.
pub fn looks_like_secrets(path: &Path) -> bool;
```
Existing callers that commit "the index as is" pass `Include::Staged` — behaviour unchanged.

## Touches
Every `CommitParams` construction (list from scout); journal entry shape and its replay/undo; the activity/journal view's wording if it names files; amend path (must keep `Include::Staged` semantics); captures of the commit editor (recaptured in S).

## Acceptance
- [ ] `first_choice`: nothing staged, 3 modified + new `.env` → 3 in, `.env` out `LooksLikeSecrets`; one staged → that one in, rest `NotStaged`.
- [ ] `looks_like_secrets` table test incl. `.env.example` false, `id_rsa.pub` false, `ID_RSA` true.
- [ ] Fixture: chosen `Whole` on a partially staged file commits the working file; `StagedPart` commits the staged content; an out file's staged state is identical before/after (index entry compared).
- [ ] Deleted and untracked chosen paths commit as removal / addition.
- [ ] `u` after the commit: HEAD, index (`git ls-files -s` equal) and working tree equal to before.
- [ ] `NothingChosen` refusal; hook failure leaves HEAD and index untouched. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~50k).
