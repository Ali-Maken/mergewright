# What M0 built where DESIGN.md said something else

The M0 review (`docs/reviews/m0-foundations.md`) found six places where the
code and DESIGN.md disagree. None of them is a bug, and in each the code is
the better answer, so the ruling is the same six times: keep the code, amend
the document. This ADR is the record of why, so that a later reader who finds
the original wording in git history knows it was replaced deliberately.

## 1. The sanitiser takes the whole `Cf` category, not five named ranges

§8.2 listed the characters to neutralise as bidi overrides and isolates
(U+202A–202E, U+2066–2069) plus zero-width and invisible formatting
characters (U+200B–200F, U+2060, U+FEFF). `term::sanitize` instead takes all
of Unicode's `Cf` (format) category, plus U+2028 and U+2029.

The rule being the category rather than a hand-picked shortlist is what makes
it maintainable: a Unicode revision that adds a format character adds it to
the rule, and the only work is refreshing the ranges. (They are still spelled
out as literal ranges in `term::sanitize`, pinned to Unicode 16.0, because
nothing in the dependency set answers character-property questions; that pin
is in `KNOWN_GAPS.md`.) The old five-range list was also already incomplete:
the soft
hyphen, the Arabic letter mark, the interlinear annotation characters and the
U+E0000 tag block are all `Cf`, all invisible, and the tag block can carry a
whole hidden sentence inside a branch name. Every one of them does the thing
the list exists to prevent — making what is drawn differ from what git acts
on. U+2028 and U+2029 are not `Cf` and are named separately because they end a
line in anything that understands them.

The cost is that a `Cf` character with a legitimate use is neutralised too.
The only real candidate is the zero-width joiner in emoji sequences, and a
broken-up emoji in a commit subject is a far smaller harm than an invisible
one; there is nothing in a repository that *needs* an invisible character to
read correctly.

## 2. `sanitize_line`, `sanitize_path` and `sanitize_path_bytes` exist

§8.2 described one function and assumed widgets would deal with `\n` and `\t`
structurally. They can't, because whether those two characters are structure
or an attack depends on where the text is going, and only the caller knows
that. A commit message drawn in a multi-line pane needs its newlines; the same
newline in a single-row status line scrolls the screen. So `sanitize` keeps
`\n` and `\t`, `sanitize_line` replaces them, and the two path functions are
`sanitize_line` for a path — a file name may legally contain a newline, and
`sanitize_path_bytes` exists because a git ref name is bytes that are only
mostly UTF-8.

Leaving this to widgets would have put the decision in the layer that has the
least information and repeated it at every call site.

## 3. `term::detect::detect()` is a free function, not `Capabilities::detect()`

§8.2 sketched the constructor form. `Capabilities` lives in `core`, and
ADR-0007 says `core` holds plain data with no behaviour — an inherent method
that talks to the terminal is exactly what that rule forbids. The detection
lives in `term`, where the probing, the parsing and the `TerminalHost` seam
already are, and returns a `Capabilities`.

The sketch also took a `probe_budget` parameter. It isn't one: the budget
follows from the environment (§4.4 — 200 ms locally, 500 ms over SSH), so
passing it in would only let a caller contradict the rule.

## 4. Under tmux, iTerm2 images are off unless the passthrough answered

§4.1 lets an environment hint stand in for a probe answer for iTerm2. That
hint describes the *outer* terminal and says nothing about whether tmux will
carry the image bytes out to it, so inside `$TMUX` the code requires the
wrapped re-probe to have answered before it will claim `iterm2_images`.

The asymmetry is deliberate: a wrong "no" costs a nicer graph, a wrong "yes"
paints escape sequences into the user's scrollback.

## 5. Protocol-less tmux gets Unicode, not ASCII

§4.2's table would have sent a tmux session with no working graphics
passthrough to ASCII, by falling through to its last row. §4.3's prose already
said Unicode, and the code agrees with the prose. tmux draws Unicode perfectly
well; what the failed passthrough tells us is that the *image transport* is
missing, not that the character set is. ASCII is for a terminal that can't be
trusted with box-drawing characters at all.

## 6. The probe deadline is per round

§4.4 sized one budget and read as though it covered startup. §4.3's tmux
re-probe is a second round and takes a full budget of its own, so two silent
rounds cost 400 ms locally, 1 s over SSH.

(Amended by the M0 round-2 review, which found a third bound this section
had missed: `tmux -V`. Its deadline was 2 s, which made the real worst case
~2.4 s. It is now the local probe budget, 200 ms, so the worst case before
the first frame is 600 ms locally and 1.2 s over SSH, and only inside tmux.
docs/design/04-terminal-capability-detection.md §4.4 carries the current numbers.)

Sharing one budget across both rounds would mean a slow first round leaves the
second too little time to get an answer, and a passthrough that works would be
reported as one that doesn't — the wrong answer, arrived at to save
milliseconds. The worst case only happens inside tmux, only when the terminal
answers nothing at all twice, and it is bounded; the trade is a slower startup
in the rare case for a correct answer in it.

## Consequences

docs/design/04-terminal-capability-detection.md §4.2, §4.3 and §4.4 and docs/design/08-architecture.md §8.2 are edited to match, each change marked
with this ADR's number. The code is unchanged. A reader who wants the reasons
comes here; a reader who wants the current design reads DESIGN.md and finds
only one story.
