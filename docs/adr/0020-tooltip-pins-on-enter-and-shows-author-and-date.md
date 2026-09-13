# The tooltip pins on Enter, and shows author and date now

Decision: **`Enter` pins the tooltip open — hover can no longer replace or
close it, and pointer movement no longer moves it — so the user can
Shift+drag to copy its text. Author and date move from M2 into M1's
tooltip now, rather than waiting.** Both are user decisions made while
triaging m1-the-graph/34 (a real Ghostty bug: a long wrapped subject pushed
the body and parents off the bottom of the box with no sign anything was
missing).

## Pin on Enter

- `Enter` toggles a *pinned* tooltip, held in a new `AppState::pinned_tooltip`
  (row index), separate from the existing hover-tracking `AppState::tooltip`.
  Rendering prefers the pinned tooltip when both are set; hover keeps
  tracking the pointer underneath it, invisibly, so un-pinning (`Esc` or a
  second `Enter`) doesn't leave a stray hover box the pointer already left.
- **Esc closes the pinned tooltip first**, before anything else Esc does
  (m1-the-graph/17's "lightest thing on screen goes first" order is
  unchanged; pinned tooltip is now the lightest thing). A second `Enter`
  also closes it — a pin is a toggle, not a one-way door.
- **The pinned tooltip stays on the commit it was opened for, not the
  cursor or the pointer.** `j`/`k` and scrolling move the cursor and the
  visible rows freely without moving or closing the pin: the point of
  pinning is to hold still text on screen while the user's hands are busy
  elsewhere (selecting it, or scrolling to compare it against something
  else). Only `Esc` or a second `Enter` closes it.
- A pinned tooltip's title reads `pinned · esc close`; an ordinary hover
  tooltip has no title at all. A permanent label would read as a status
  field with nothing to report the rest of the time (a project rule) — so
  hover, the common case, stays unlabelled, and the pin is the one state
  that says so.

## Author and date now, not M2

DESIGN.md had deferred author/date to M2 alongside "contained in". Doing
"contained in" needs ref-walk machinery M1 doesn't have; author and date
need none — the commit already carries them. Splitting the tooltip's
content into two milestones for that reason bought nothing but a second
round of layout work, so the user asked for both now. The tooltip's
content, top to bottom: full hash (40 hex, selectable), author
(`Name <email>`, mailmap-applied — the same resolution `row_entry` already
does per row), date (absolute in the commit's own offset, and relative:
`2026-09-13 14:02 +0500 · 4h ago`), committer only when it differs from
the author, `parent`/`parents`, a blank line, subject, body. "Contained
in" stays out of scope for M1 (`KNOWN_GAPS.md`).

No date/time crate: `absolute_date` uses Howard Hinnant's `civil_from_days`
(integer days-since-epoch to Gregorian calendar), matching the crate's
existing minimal-dependency stance (`relative_date` already avoids one).

## The root cause, fixed alongside

The box's height was sized from the tooltip's raw logical line count, not
the wrapped row count `ratatui` would actually render — so a long subject
or body silently pushed later lines outside the box. Fixed by measuring
the wrapped height with `Paragraph::line_count` (behind ratatui's
`unstable-rendered-line-info` cargo feature — compiles today, no API
contract, acceptable here because the alternative is reimplementing word
wrap ourselves to measure it). When even the tallest box the terminal
allows still can't fit everything, lines are dropped from the bottom and
a `truncated` marker appended — never a silent gap.

## Consequences

- `graft-ui` depends on an `unstable-*` ratatui feature. Revisit if a
  ratatui upgrade breaks it; the fallback is reimplementing wrap-aware
  line counting locally.
- The footer hint for `Enter` is now `Enter pin` (was `Enter details`),
  coordinated with m1-the-graph/33's hint bar so it still drops as a whole
  hint under width pressure.
