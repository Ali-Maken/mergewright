# Graft: Design — Repetition

[See also: docs/design/README.md](README.md) for the full section index and cross-references.
Previous: [Proposed changes](09-proposed-changes.md)

---

## 10. Repetition

**Not an ADR.** This is a standing principle, not a decision between
alternatives — there was nothing else on the table, just a rule the two
cases below make obvious once stated. (User-approved 2026-09-21.)

### 10.1 The principle

> **Repetition teaches when the second instance is the only one the user
> will be looking at; it wastes when both are in the same glance.**

Operationally: **is the duplicate reachable without moving the eye?**
Yes → cut it. No → keep it, at every width.

### 10.2 Two worked examples, opposite rulings

**Keep: the `.` context menu lists every verb, even ones the hint bar also
shows.** A user who has opened the menu is reading *the menu*; the hint bar
is peripheral, not in the same glance. A verb discoverable only at 160
columns and invisible at 80 is a verb that does not exist. This is already
the rule in [`02-keymap.md` §2.1](02-keymap.md#21-principles) ("The menu's
contents never depend on terminal width") — cited here, not restated.

**Cut: the conflict banner drops the filename.** On a 24-row screen the
changes pane names the conflicted file four rows below the banner — one
glance down, still on screen. Saying the filename in both places spends a
row on nothing. Seen in the user's eye-check of
`docs/shots/m5-ui-banner-conflict-80x24.txt`; built by ticket 16.

The two rulings are opposite on purpose: one keeps a duplicate, one cuts
one. What decides it is not "is this shown twice" but where the user's eye
already is when they need the second instance — inside the open menu, or
already scanning a small screen that has the fact on it regardless.
