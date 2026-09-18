# Field notes

What real use turned up. **Your daily use is a test tier** (ADR-0029), not a
fallback for when testing fails — it's the only layer that reaches how things
look, repo shapes nobody made a fixture for, and sequences nobody thought to
script. The fast tier covers logic on fixtures we invented; heavy and perf cover
scale at review; the acceptance walk covers the promise along a path someone
wrote down.

**How to use it:** one line, dated, no ceremony. Write it yourself or say it and
the conductor writes it. Half a note beats a forgotten one — "diff looked wrong
on the big repo, Tuesday" is enough to start from.

Unfixed notes feed the next milestone's design pass. Fixed ones get a commit
hash and stay, so the same thing isn't rediscovered.

See also: [`KNOWN_GAPS.md`](../KNOWN_GAPS.md) (what we already know is broken or
unverified — check here first, it may be a known one),
[`docs/log/`](log/) (what was built when).

---

## Open

_(nothing yet — M0–M2 were read-only and never left the fixtures)_

## Fixed

_(nothing yet)_
