# health — MAP.md as a map: guard test, plus core, term, graph, git, fixtures and a new capabilities MAP

Status: done (planner, from `docs/reviews/m8-architecture.md` Major M1). **§ Addition is open** (planner, 2026-10-09); it must merge before 38.

Build with: **Sonnet** (conductor, after scout `scouts/37-health-map-libraries.md`). The planner expects **Sonnet**. A grep-style test checks the work, and each MAP line is judged against the item it names.

Needs:
Blocked by: none (conductor, from Needs)
- 27 merged, because 27 rewrites MAP entries that name env vars and the binary.
- Any `health-split-*`/`health-ops-surface-narrowed` that has landed. It must run before 28.

This is the first of four MAP tickets. The others are `health-map-ops`, `health-map-ui` and `health-map-app`. This one writes the guard, and each later ticket adds its crates to it.

## Why
- The MAPs have grown into change logs: about 2,700 lines across the crates, with ticket ids, rulings, measurements and private items.
- A builder told "MAP before source" can't read them inside a ticket. The architecture pass couldn't read graft's MAP whole.
- `graft-capabilities` has no MAP.

## Read first
- `docs/reviews/m8-architecture.md` M1.
- `crates/graft-core/MAP.md` (87 lines) and `crates/graft-term/MAP.md` (88) whole: these are the nearest to right.
- `crates/graft-graph/MAP.md` :1–40 and :110–125.
- `crates/graft-git/MAP.md` :1–60. Then grep the old git MAP per item rather than reading it whole.
- `ARCHITECTURE.md`'s graft-capabilities paragraph (grep `graft-capabilities`).

## The MAP format (the guard enforces items 1–4)
1. One `## <module>` heading per public module, in source order.
2. Under it, one line per `pub` item: `` - `name` (kind) — signature or shape; one clause on what it is for. ``
3. Each line is at most 240 characters. History, ticket ids, review ids, rulings, measurements and test names are not allowed: they live in `docs/log/` and the ADRs.
4. `pub(crate)` and private items aren't listed. **"Private" includes a `pub` item no caller outside the crate can name** (§ Addition, ruling).
5. A two-to-four-line crate preamble: what the crate does, and what it may and may not depend on, quoted from ARCHITECTURE.md's table.

## Scope
In:
- A new test `crates/graft/tests/it/map_coverage.rs`. For each crate in `const CHECKED: &[&str]`:
  - (a) For every `pub` (not `pub(…)`) `fn|struct|enum|trait|type|const|static|mod` declared in `src/**` outside a `#[cfg(test)]` module, `` `name` `` appears in that crate's MAP.md. **Superseded by § Addition: (a) runs over the public set.**
  - (b) Every non-heading line is at most 240 characters.
  - (c) No line matches the ticket-id pattern `\bm\d+(-[a-z0-9]+)*/\d+[a-z0-9-]*\b`.
  - (d) Every `` `name` `` the MAP lists as an item resolves to a `pub` declaration. This catches stale entries. **Superseded by § Addition: (d) resolves against the public set.**

  `CHECKED` starts as `graft-core`, `graft-term`, `graft-graph`, `graft-git`, `graft-fixtures` and `graft-capabilities`.
- Rewrite those five MAPs in the format above.
- Write `crates/graft-capabilities/MAP.md`.
- An item that is `pub` but used only by tests (git `Mailmap::from_bytes`, `TipStats`, `GitInvocation::stops_hooks_first`; review Minor 6) is listed as is, and NOTES names it for the conductor. Don't change its visibility in this ticket. **Superseded by § Addition.**

Out:
- Code changes other than the test.
- ops, ui and graft MAPs.
- ARCHITECTURE.md.

## Signatures
None public. Test fns:
- `fn every_pub_item_is_in_its_map()`
- `fn map_lines_are_short_and_carry_no_ticket_ids()`
- `fn every_map_item_still_exists()`

## Acceptance
- [x] The three tests pass over `CHECKED` — `map_coverage::every_pub_item_is_in_its_map`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, `map_coverage::every_map_item_still_exists`.
- [x] Removing one item line from the git MAP fails `every_pub_item_is_in_its_map` with that name. Done by hand and reverted, with the line quoted in NOTES (`map_coverage::every_pub_item_is_in_its_map`).
- [x] Adding `m8-polish-and-ship/25` to a MAP line fails the ticket-id check. Done by hand and reverted, and quoted (`map_coverage::map_lines_are_short_and_carry_no_ticket_ids`, plus `map_coverage::ticket_id_pattern_matches_ids_and_nothing_else`).
- [x] NOTES gives the line count before and after for each MAP (see NOTES; counted with `wc -l`).
- [x] `graft-capabilities/MAP.md` exists, and the crate is in `CHECKED` (`map_coverage::every_pub_item_is_in_its_map`).

## NOTES (builder)
- Red first: all three guard tests failed on the old MAPs.
- Mutation 1: removed the git line `` - `CHUNK` (const) — commits per chunk a walk hands back. `` → `every_pub_item_is_in_its_map` failed: ``graft-git: `CHUNK` ``. Reverted.
- Mutation 2: appended `m8-polish-and-ship/25` to `` - `Order` (enum) — the order a walk emits rows in. `` → failed with `graft-git/MAP.md:129: ticket id: …`. Reverted.
- Lines before → after: core 86→95, term 87→107, graph 160→141, git 358→314, fixtures 50→120, capabilities none→26.
- How the guard reads things:
  - A module counts as named when its `## heading` names it.
  - It skips `#[cfg(test)] mod x;` files and anything named `tests`, so git's `fixture` module isn't listed.
  - A listed name resolves if it is declared anywhere in src.
- Guard vs format point 4: `pub` items in private modules are listed, because check (a) as specified demands it:
  - fixtures: stream, shapes, digest, filesystem, permissions
  - graph: `Ring`, `RasterPool`
  - term: `Decoded`, `Decoder`
  
  Conductor: should the guard skip them?
- `pub` but used only by tests, listed as is: git `Mailmap::from_bytes`, `TipStats`, `GitInvocation::stops_hooks_first`.
- grep `MAP.md` in CLAUDE.md/ARCHITECTURE.md: only CLAUDE.md:65, and it doesn't misdescribe the format.
- Spot-checked 4 MAP lines against source (`git_editor`, `InputMux::give_back`, `Decoder::feed`, `Unavailable`): 4 held. The rest were drafted per crate by forks reading source and weren't judged one by one here.

## Touches
- the five MAPs plus the new one
- `crates/graft/tests/it/main.rs`, one `mod` line
- CLAUDE.md/ARCHITECTURE.md text that describes MAP.md: grep `MAP.md`, and put any line that now misdescribes the format in NOTES for the conductor; don't edit it
- the later `health-map-*` tickets, which extend `CHECKED`

## Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

## Size
About 70k tokens.

---

## Addition (planner, 2026-10-09): private-module `pub` and test-only `pub`

Status: open. Build with: **Sonnet**. It tightens a test, deletes MAP lines and makes three visibility moves; the tests below judge it. Brownfield: the conductor decides on a scout.
Needs: none. **Must merge before 38**; 38, 41 and 42 build on `public_items`.

### Ruling
- **Format point 4 wins.** A MAP lists what a caller outside the crate can name. Check (a) asked for every `pub` keyword instead of every public item, and that was the bug.
- **The public set** is what (a) requires and what (d) accepts. The guard computes it from `src/`:
  1. Every `pub` item in a file or inline module reached from `lib.rs` through `pub mod` links only. One `mod`, `pub(crate) mod` or `pub(super) mod` link in the chain → not public.
  2. Plus every name a public module re-exports with `pub use`. A rename (`as X`) adds `X`. `pub use m::*` makes `m`'s `pub` items public, as in 1.
  3. Plus each `pub fn` in an inherent `impl T` block whose `T` is public by 1 or 2. For a renamed re-export, match on the original name.
  4. Minus any item carrying `#[cfg(test)]` itself, and anything in a `#[cfg(test)]` module (as now).
- List a re-exported item under the heading of the module that re-exports it. For the crate root, that is the root's heading.
- **(d) tightens.** A listed name must be in the public set; being declared somewhere in `src/` is no longer enough. This catches the crate-private entries M1 names.
- **Test-only `pub` is a visibility defect** (review Minor 6), not a MAP question. Fix the cause. The guard gets no exemption list.
  - Used only by the crate's own unit tests → put `#[cfg(test)]` on the item. Rule 4 then drops it.
  - Used by the crate's `tests/` or by another crate's tests → move it into `pub mod testing` in its crate. The module's heading gets the one-line preamble "test support; no production caller". It stays listed, so a builder looking for it finds it.
  - Has a production caller → it isn't test-only. Leave it, and NOTES says where the caller is.
- **Rejected:** skipping `#[doc(hidden)]` items. That would hide them from the one place a builder looks.

### Scope
In:
- `map_coverage.rs`: add `public_items`, which builds the public set. (a) and (d) both use it. On failure, (a)'s message starts with `<crate>: <N> missing`.
- Delete the MAP lines the tightened (d) rejects, across the six `CHECKED` MAPs. The likely ones are fixtures' stream, shapes, digest, filesystem and permissions items; graph's `Ring` and `RasterPool`; and term's `Decoded` and `Decoder`. Anything re-exported stays; the guard decides.
- graft-git: apply the test-only rule to `Mailmap::from_bytes`, `TipStats` and `GitInvocation::stops_hooks_first`, and update their MAP lines to match.

Out: any other visibility change, and the ui, ops and graft MAPs.

### Signatures
- `fn public_items(src: &Path) -> BTreeSet<String>`: private to the test module.
- Test fns:
  - `fn public_set_skips_private_module_items()`
  - `fn public_set_keeps_reexports_and_their_methods()`
  - `fn public_set_skips_cfg_test_items()`
- Each one builds a small `src/` tree in a temp dir, the way the `it` suite already makes temp trees. Add no new dependency.

### Acceptance
- [ ] `lib.rs` = `mod hidden; pub mod shown;`, with `pub struct H` in hidden and `pub struct S` in shown → the set has `S` and not `H` (`map_coverage::public_set_skips_private_module_items`).
- [ ] `mod hidden; pub use hidden::H; pub use hidden::K as Renamed;`, with `impl H { pub fn go() }` → the set has `H`, `go` and `Renamed`, and not `K` (`map_coverage::public_set_keeps_reexports_and_their_methods`).
- [ ] `#[cfg(test)] pub fn only_tests()` in a public module → absent from the set (`map_coverage::public_set_skips_cfg_test_items`).
- [ ] The three existing guard tests pass over `CHECKED`, with (a) and (d) on the public set (`map_coverage::every_pub_item_is_in_its_map`, `map_coverage::every_map_item_still_exists`, `map_coverage::map_lines_are_short_and_carry_no_ticket_ids`).
- [ ] Mutation: put back one deleted private-module line (for example graph's `Ring`) → `every_map_item_still_exists` fails and names it. Reverted, with the failure quoted in NOTES (`map_coverage::every_map_item_still_exists`).
- [ ] Each of the three git items is either `#[cfg(test)]` or in graft-git's `testing` module. NOTES quotes the `rg` of its callers that decided which. Verified by the workspace gates compiling those callers, plus `map_coverage::every_pub_item_is_in_its_map`.
- [ ] NOTES gives graft-ui's public-set size. Measure it by adding graft-ui to `CHECKED` for one run, quoting the `graft-ui: <N> missing` line, then reverting (`map_coverage::every_pub_item_is_in_its_map`). 38 plans its passes on this number.
- [ ] NOTES gives the line count before → after for each MAP changed (`wc -l`).

### Touches
- `crates/graft/tests/it/map_coverage.rs`.
- The six `CHECKED` MAPs.
- graft-git `src/` for the three items and every caller `rg` finds (production, `tests/`, other crates).
- 38, 41 and 42: they rely on `public_items` and the tightened (d); they were updated on 2026-10-09.
- For the conductor: review Minor 6 is closed for git.

### Gates
Workspace-wide: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`.

### Size
About 40k tokens.
