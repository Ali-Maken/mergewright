# G-look 5c — outside a repository: the mock-up's empty state

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: the screen is specified (DESIGN §1.15); the
in-process repo switch is the one structural part, small and tested.
Needs: `look-chrome-header-gutter-bar` (`APP_NAME`, bar form). Disjoint files from 5b — may run beside it.

## Read first
- `docs/design/01-view-mockups.md` §1.15 (whole)
- `docs/design/02-keymap.md` §2.13
- `docs/adr/0056-help-is-the-palette-on-this-place-and-the-tour-is-a-practice-repo.md` (`Exit::Open` paragraph)
- `docs/design-notes/m8-polish-and-ship.md`: Journeys J14, J21, "Where he lands" rows for startup/open/init;
  Interfaces `graft-ui::empty_state`, recents
- `docs/shots/m8-proto-3a-empty.txt`; `.scratch/m8-polish-and-ship/real-vs-mockup.md` difference 4
- `crates/graft/MAP.md` (startup, discovery, `Exit`, `event_loop::run`, config dir); `crates/graft-ui/MAP.md`
  (`TextField`/`FieldView`, `FieldWidget`)

## Scope
In:
- `graft-ui::empty_state` widget per §1.15: header `<APP_NAME> · no repository here`, folder line (`· no such folder`
  when missing), `o` and `i` actions drawn (`c`, `t` not drawn until built — §1.15's rule), RECENT (newest
  preselected; hidden when empty), bar `o open… · i init · ⏎ open the selected · q quit` (no doorway: nothing behind
  one yet; G-palette adds `? keys`, G-first-run `c`, G-tour `t`).
- Keys: `o` path prompt (`~` expanded, `⇥` completes a directory, `⏎` opens, `esc` cancels); `i` → `Make <folder> a
  Git repository?` `y` inits via the existing init path (or a new journaled call if none — scout), `n`/`esc` declines;
  `↑↓ j k` move; `⏎` opens the selected; `q` quits.
- Recents: `<config_dir>/graft/recent`, newest first, ≤ 20, deduped by canonical path; written on every successful
  open (inside a repo too). Missing file = empty list; a malformed line is skipped and counted in a status warning,
  never silently (rule 3).
- `Exit::Open(PathBuf)`: main re-runs startup for that path in-process (capability probe not repeated).
- Open fails → screen stays, bar row reads the reason.
Out: clone `c` and the first-launch line (G-first-run); tour `t` (G-tour); `?` (G-palette).

## Signatures
```rust
// graft-ui::empty_state
pub struct RecentRow { pub name: SanitizedText, pub path: SanitizedText, pub age: SanitizedText }
pub struct EmptyStateView { pub folder: SanitizedText, pub missing: bool, pub recent: Vec<RecentRow>,
    pub cursor: usize, pub prompt: Option<EmptyPrompt>, pub problem: Option<SanitizedText> }
pub enum EmptyPrompt { Open(FieldView), ConfirmInit }
pub struct EmptyState<'a> { pub view: &'a EmptyStateView, pub theme: &'a Theme }   // impl Widget
// graft (pub(crate))
pub(crate) fn read_recent(config_dir: &Path) -> Result<Vec<PathBuf>, RecentError>;
pub(crate) fn remember(config_dir: &Path, repo: &Path) -> Result<(), RecentError>;
// Exit gains Open(PathBuf)
```

## Acceptance
- [ ] Widget tests at 80×24, 120×36: rows as §1.15; RECENT hidden when empty; missing-folder wording; `c`/`t` not drawn.
- [ ] Recents: newest first, dedup, cap 20, malformed line counted (tests).
- [ ] J14 `open-recent-outside` = 2: pty walk — start the binary in a non-repo with a recents file, `⏎` → that repo's
      graph, cursor on HEAD; `open-second-recent` = 3 (`j ⏎`) (walk test counts keys).
- [ ] J21 `init-here` = 2: `i y` → the new repo's graph; `i esc` → empty state, no `.git` written (tests).
- [ ] Open of a non-repo path from `o` → screen stays, bar row reads the reason (test).
- [ ] `every_key_the_bar_draws_is_answered` covers the empty state (extend; name it).
- [ ] Captures: `docs/shots/m8-look-outside-repo-120x36.txt` (with recents), `…-no-recent-80x24.txt`, beside
      `m8-proto-3a-empty.txt`. **Colour unverified** (rule 5).

## Touches
- Startup path and what it prints outside a repo today (replaced); `event_loop::run` re-entry; config dir resolution;
  the init path (journaled? undo of an init is not offered — state it in NOTES).
- Keymap §2.13 (written); DESIGN §1.15 (written); MAP.md graft-ui, graft; G-rename list: `<config_dir>/graft/recent`.
- Tests asserting the old outside-repo message (grep the text).

## Scout questions
- What does the binary do outside a repo today (file:line); where is discovery; can `event_loop::run` run twice in one
  process (MAP says it may — confirm the terminal enter/restore pairing)?
- Is there an init path in graft-ops/graft-git?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens.
