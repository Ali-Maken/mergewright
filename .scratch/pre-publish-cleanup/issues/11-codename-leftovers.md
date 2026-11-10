# pre-publish: codename leftovers (superseded)

Status: superseded (spec-only pass, 2026-11-10)

Split by size and by the user's 2026-10-25 ruling, U1 = drop:
- `11b1`–`11b4`: old-generation code removed (it replaces the `old-generation` allow-list reason, which is gone).
- `11a`: the app crate, the shipped scripts and docs, and the guard (written first).
- `11c`: every other crate, ops included.

D1 (shipped set), D2 (guard), D3 (replacements) and D5 (m3 script) moved to 11a. D4 changed: earlier-name strings in production now go to the `on_disk` consts (current state only), not a stop.

Scout file: `scouts/11-codename-hits.md`.
