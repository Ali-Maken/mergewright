# M0 Foundations — tickets

Checkpoint test: **opens a repo; quitting leaves the terminal clean.**
Scope source: PROMPT.md §8 (M0 row). Build with: Sonnet (PROGRESS.md).
Builders run one at a time; the order below respects every `Blocked by`.

| # | Ticket | One line | Blocked by |
|---|---|---|---|
| 01 | [workspace-scaffold](issues/01-workspace-scaffold.md) | Workspace, 7 crates, lint/format/parallelism config | — |
| 02 | [dependency-edge-test](issues/02-dependency-edge-test.md) | Fast-tier `cargo metadata` edge-set assertion (ADR-0006) | 01 |
| 03 | [terminal-session-and-panic-hook](issues/03-terminal-session-and-panic-hook.md) | RAII setup/teardown + panic hook with crash report | 01 |
| 04 | [capability-detection](issues/04-capability-detection.md) | `term::detect` per docs/design/04-terminal-capability-detection.md §4; tier decision (largest ticket) | 01 |
| 05 | [sanitizer](issues/05-sanitizer.md) | `term::sanitize` → `SanitizedText` newtype | 01 |
| 06 | [theme-tokens](issues/06-theme-tokens.md) | Tokens, 3 themes, lane palette, depth mapping | 04 |
| 07 | [config](issues/07-config.md) | TOML config: `ui.theme`, `graph.renderer`, `ui.icons` | 04, 06 |
| 08 | [open-repo](issues/08-open-repo.md) | `git::RepoHandle::open` via gix + git-version check | 01 |
| 09 | [event-loop](issues/09-event-loop.md) | Event → update → view spine; quit; sync output | 03 |
| 10 | [job-runner](issues/10-job-runner.md) | Worker pool, cancellation, results as events | 09 |
| 11 | [open-repo-screen](issues/11-open-repo-screen.md) | Wire 04–10 together; the checkpoint itself | 04–10 |
| 12 | [m0-wrap](issues/12-m0-wrap.md) | README, CHANGELOG, KNOWN_GAPS sweep, final gates | 02, 11 |

Notes for the conductor:
- 04 is the riskiest/largest; if a Sonnet builder stalls there twice, escalate per PROGRESS.md (debugger on Fable) rather than a third blind retry.
- 05 (sanitiser) is not named in PROMPT.md §8's M0 list; it is pulled in because 11 renders repo-derived strings and ARCHITECTURE.md forbids them crossing into `ui` unsanitised. Minimal version only.
- Startup probe typeahead loss and resize re-probe stay open as recorded M1 gaps (KNOWN_GAPS.md, ADR-0012); no M0 ticket tries to fix them.
