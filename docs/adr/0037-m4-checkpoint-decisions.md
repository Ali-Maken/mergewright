# ADR-0037 — M4 checkpoint decisions (keymap, contact sheets, undo after push, the hint bar, the password buffer)

Date: 2026-09-20. Decided by the user at M4's checkpoint. Status: accepted.

Six decisions and two reclassifications. Each one binds from M5.

## 1. `docs/design/02-keymap.md` is the single source of truth for keys

The reconciled tables from `docs/reviews/keymap-reconciliation.md` are written
into `02-keymap.md`. The review file becomes a pointer to it and stops carrying
tables of its own.

## 2. A milestone's captures ship as one contact sheet

52 files is not reviewable one at a time. Each milestone produces **one file**
that concatenates every capture in ticket order, each preceded by its "what
right looks like" line. The user reviews by scrolling once.

The per-scenario capture files stay — the contact sheet is assembled from them,
not instead of them. The acceptance group builds it. The 200 KB cap is per
capture file; the sheet is exempt but stays trimmed.

M3's and M4's sheets are built retroactively.

## 3. `u` skips fetch and push, **and says so**

Approved, and written into DESIGN §6.4: fetch and push are transparent to the
undo journal, so `u` steps over them to the last local change.

**But a key that looks dead reads as broken.** After a push, `u` explains that a
push cannot be undone locally and offers the guided re-push. Silence is not an
acceptable implementation of "transparent".

## 4. The hint bar never hides every remote key

Rejected: hints that need ≥144 columns. The user works at 80 and 120 columns.
The bar is how anything is discoverable, so it shows the **three most relevant
keys for the current context** and drops or abbreviates the rest. Never all
hidden. A feature nobody can see doesn't exist.

## 5. The typed password gets a zeroizing buffer, in M5's first group

Not deferred. The user runs Graft for real work daily, so it is a real password
sitting in a plain `String`, cloned per keystroke and freed unwiped. M5's first
group replaces it with a zeroizing buffer; the value is never logged or
captured. Recorded in `KNOWN_GAPS.md` as a security row expiring M5.

## 6. Two entries were bugs, not gaps → M5 tickets

- **`Esc` cannot cancel `u`/`U`.** An operation that can't be cancelled is the
  opposite of what undo is for. Needs a between-step stop check and rollback in
  `graft-ops` `restore::execute`.
- **The `Ctrl+R` race leaves an old history job uncancelled.** A concurrency bug
  that worsens as job kinds multiply.

These stay gaps with named expiries: SSH untested (HTTP is the proven path);
lease push refused on any branch but the current one's upstream; the push box
draws no frame.
