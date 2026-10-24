# 25 — Failed setting bar has no ✗; help ranks HERE rows above a setting named by the query

Status: ready · Build with: Sonnet · Blocked by: none
Source: KNOWN_GAPS ease/06i-failed-no-cross, ease/06i-graph-query-steps (both broken, rulings in the rows).

## Need / scope
1. A failed setting write draws its bar without `✗` (`view/status.rs:108` builds the line from the message alone).
   Ruling: a failure shows `✗`, as every other failure bar. Red-first test on the drawn bar.
2. Typing `graph` in help: three HERE rows match first; `? graph ↓ ⏎` lands on "Write the commit-graph file"; switching the
   graph setting takes 7 steps vs ≤4 (`? every ↓ ⏎` = 4). Ruling: a setting whose name starts with the query lists before
   HERE rows. Red-first test on the ranking; the journey counted in a test (≤4 steps).
3. Captures that show the help list or a setting failure: re-shoot and list; user judges by eye.

## Acceptance
- [ ] ✗ on a failed setting write; test named.
- [ ] Ranking: setting prefix-match before HERE rows; test named; `graph` switch journey ≤4 steps, test named.
- [ ] Captures re-shot and listed (≤200 KB), or none affected with the grep that shows it.
- [ ] Five gates + `scripts/walks.sh`; Touches: (keymap doc, help catalogue, other screens drawing status bars).
