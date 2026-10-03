# Scout 08 — outside a repository (as of 35d1a96b)

## What happens today
- `crates/graft/src/main.rs:27-38` parses CLI → `Invocation::Open(path)`; `config::load()` at :43 before the terminal is touched.
- `event_loop/startup.rs:106` `run(Startup)`: `open_repository` (:61-73, `RepoStore::shared()`) runs **before** the session; failure → `Exit::Fatal(reason)` (:114-117). `main.rs:69-72` prints `graft: <reason>` on stderr, exit 1. No screen, no alternate screen, no state held. Nothing is drawn.
- No empty-state widget exists anywhere in `crates/graft-ui/src` (file list checked). No recents file, no `recent` code.
- `Exit` enum: `event_loop/mod.rs:53-66` — `Quit`, `HandedBack(KeyCode)`, `Signal(i32)`, `Fatal(String)`. No `Open`.
- Config dir: `config/load.rs:127-152` resolves `$XDG_CONFIG_HOME/graft/` (relative XDG ignored, falls back to `HOME/.config`); `config/mod.rs:1`.

## Can `run` run twice in one process?
- Each `run` opens its own `TerminalSession` (`startup.rs:95-99`, `crash::install` re-sets the panic hook each time, `crash.rs:27`) and restores via `wind_down` (:195). Pairing per call looks self-contained.
- `InputMux::on_stdin` (`graft-term/src/input/mux.rs:94-104`) is a process singleton: a second call reuses the mux and hands out fresh `Inputs` — built for re-entry (comment `startup.rs:120-124`, "the gallery").
- **Contradiction:** ticket says "capability probe not repeated", but `probe(...)` is called inside `run` unconditionally (`startup.rs:136`). Re-running `run` re-probes. Avoiding it needs `run`'s shape changed — could not confirm any existing cache.
- Headless path (`startup.rs:118-127`, non-tty stdout) also only follows a successful open; the empty state has no headless answer today.

## Init path
- **None in production code.** `git init` appears only in tests/fixtures: `graft-git/src/fixture.rs:48` (`#[cfg(test)]`, `lib.rs:16-17`), `graft-fixtures/src/git.rs` (`init_repo`), test files in graft-ops. No graft-ops operation, no journal entry kind. Ticket's "existing init path" does not exist → new call needed.

## Keys
- Keymap §2.13 (`docs/design/02-keymap.md:535-548`): `o i c t ⏎ ↑↓ j k ? q` — all claimed for this screen. Ticket draws `o i ⏎ jk q`; `c t ?` left to other groups — consistent with doc. `esc`/`⇥`/`y`/`n` inside prompts are not in the §2.13 table (prompt-local; could not confirm a rule naming them).
- `every_key_the_bar_draws_is_answered` lives in `graft::update::tests` (ref `graft-ui/src/graph_screen/hints.rs:28,87`); it iterates `HintContext` — no empty-state context exists; extension needs a new context or a separate sweep.

## Tests / captures that change
- `crates/graft/tests/it/not_a_repo.rs:9-31` asserts exit 1, stderr `graft: …not inside a git repository`, and **no `\e[?1049h`** — directly contradicted by the new screen; must be rewritten.
- Other `Exit::Fatal` users in tests (grep hit): `repo_truth.rs`, `m3_walk.rs`, `undo_over_drift.rs`, `m4_walk/run.rs`, `m5_walk/{run,resolving}.rs`, `m7_walk/rewrite_editor.rs` — not checked whether they hit the not-a-repo case.
- Existing shots: only `docs/shots/m8-proto-3a-empty.txt`, `m8-proto-3c-clone-prompt.txt` (prototypes, unchanged). No golden for this screen.

## Overlap with ticket 07 (cursor row / dark)
- 07 touches `graft-ui` theme tokens (`selection`, `legible_on`) and every screen painting `selection` (07 Touches :60-66). New `empty_state` would paint its RECENT cursor row with `selection` → depends on 07's token/helper if built after; otherwise new file disjoint. Shared file risk: `graft-ui/src/lib.rs` (mod list), `graft-ui/MAP.md`, `theme.rs` if 08 adds tokens.
