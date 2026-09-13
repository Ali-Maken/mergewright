# Issue tracker: Local Markdown

Tickets and specs for this repo live as markdown files in `.scratch/`. This repo has no
git remote, so there is no `gh`/`glab` tracker to call: everything is local files.

## Conventions

- One milestone per directory: `.scratch/<milestone-slug>/`, where the slug comes from the
  milestone table in `PROGRESS.md` (`m0-foundations`, `m1-the-graph`, `m2-details-and-diffs`, …).
  Work that isn't a milestone (a spike, a one-off fix) gets its own `.scratch/<feature-slug>/`.
- The spec, when a skill writes one, is `.scratch/<slug>/spec.md`. The product spec for the
  whole project stays at `PROMPT.md`; don't duplicate it here.
- Tickets are one file per ticket at `.scratch/<slug>/issues/<NN>-<ticket-slug>.md`, numbered
  from `01`, never a single combined tickets file.
- The ticket ID is `<slug>/<NN>` (for example `m1-the-graph/03`). The conductor passes that ID
  and the file path to each builder.
- Near the top of each ticket file:
  - `Status:` one of `draft` (unfiled spec), `ready`, `in-progress`, `partial`, `done`, `blocked`, `wontfix`.
  - `Build with:` the model the spec names (`Sonnet`, `Opus`, `Fable`, `Fable (signature)`), so the
    conductor picks the right builder. From M2, `planner-fable` sets it; the conductor doesn't change it.
  - `Blocked by:` a comma-separated list of ticket numbers, when the ticket depends on others.
    A ticket is unblocked when every file it lists is `done`.
- Comments and conversation history append to the bottom of the file under a `## Comments`
  heading.

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<slug>/issues/` (creating the directories if needed). Use the
next free number.

**Who drafts a milestone's tickets:** M0 and M1 were drafted with `/to-tickets`. **From M2,
`/to-tickets` is not used:** `planner-fable` writes unnumbered specs (`<ticket-slug>.md`,
`Status: draft`) in its design pass, and the conductor numbers them `NN-<slug>.md`, sets
`Blocked by:` from each spec's `Needs:` and sets `Status: ready`. Write milestones: the user
approves them at the checkpoint before the milestone starts.

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The conductor normally passes the path or the ticket ID
directly.

## Rules that override the generic skill defaults

- **Never mark a ticket `done` when the work is stubbed or half-finished** (`CLAUDE.md`). Record
  the shortfall in `KNOWN_GAPS.md` and leave the ticket open.
- Commit after each ticket. Never push or force.
- `.scratch/` is committed with the repo: tickets are part of the project record, alongside
  `PROGRESS.md` and `docs/reviews/`.

## PRs as a request surface

Off. There is no remote, so no pull requests enter the queue.
