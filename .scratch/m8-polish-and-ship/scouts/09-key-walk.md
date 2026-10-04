# Scout 09: key walk (what the code actually matches)

Paths are relative to `crates/graft/src/update/` unless they say otherwise. Tests are skipped. "plain" means no Ctrl and no Alt.

## Inventory per place

**Global/graph** (`keys.rs` key_event 160-268, graph_key 318-346; predicates in `keys/predicates.rs`)
- `q` quit (55); `Ctrl+c` quit (56) → `quit::asked` keys.rs:320
- `Ctrl+r` refresh (157-158); `F(2)` status/screen (63); `F(3)` tier (82); `+` lanes (74); `F` focus (70)
- `y` copy sha (87); `x` compare (93); `.` context menu (134); `c` commit, `a` amend (125-126)
- `b` branch prompt, `t` tag prompt (106-107); `m` merge, `r` rebase preview (`preview/history.rs:26-27`); `o` take (`preview/take.rs:42`); `R` rewrite editor (115; keys.rs:404)
- `f` fetch, `p` pull, `P` push (25-27)
- `Space` fold (or `edit_stop::space_key` stages first, `keys/edit_stop.rs:29`); `1` first-parent; `H` stash visibility (`keys/view_change.rs:87-89`)
- `j`/`Down`, `k`/`Up`, `PageDown`, `PageUp`, `g`/`Home` top, `G`/`End` bottom (39-44). Not matched with Ctrl or Alt held (35)
- `u` undo, `U` redo, `Ctrl+z` undo (148-150). Skipped while a text field is open (keys.rs:230). With `suspend_on_ctrl_z` set, `Ctrl+z` suspends instead (keys.rs:217-223). `Ctrl+Z` (uppercase) is **not matched**; the comment at 140-142 says so on purpose
- `Esc` cancels a running write (keys.rs:241). Otherwise it dismisses a Failed or Sticky status (keys.rs:249-260). Otherwise `escape_key` (keys.rs:344)
- `/` search (`search.rs:34`). While a search is kept, `n`/`N` step through it (`search.rs:43-44`). In the search prompt: typed chars, `Backspace`, `Esc`, `Enter` (56-61)
- `B` blame, `X` (`commit_graph.rs:89,93`)
- `C` opens the branch picker (`picker.rs:121`); `S` stash (`stash.rs:175`); `J` journal (`journal/keys.rs:29`); `M` map (`map.rs:144`); `O` strip (`strip.rs:20`)
- On the graph, nothing matches `h`, `l`, `Left`, `Right`, `?`, `:`, `Ctrl+p`, `e`, `d`, `s` or `i`. No arm exists for them.

**Detail pane / diff** (`detail/keys.rs`)
- `<` `>` resize while the pane is shown (105). `Esc` closes Explain or clears the compare base first (27). With the pane shown, `Esc` closes it (154)
- `Enter` opens or enters (130/136/140); `Tab` and `BackTab` cycle focus while the pane is shown (158/162)
- In a place sub-mode (84-91): `Esc` show again, `B` blame, `F` file history
- File list (291-315): `j`/`Down`, `k`/`Up`, `PageDown`, `PageUp`, `g`, `G`, `Space` folds a dir. There is **no Home or End**, although the comment at 282 says "home/end"
- Diff scroll (337-342): `j` `k` `Down` `Up` `PageDown` `PageUp` `g` `G`. No Home or End
- Diff options (`diff_option_key` 360-415, plain only): `v` unified/side-by-side (376), `w` whitespace (383), `W` word tint (392), `[` (396), `]` (402), `L` (408)

**Changes pane** (`detail/keys.rs` 210-275, only when the WorkingTree spec is shown)
- `Enter` on a conflict opens the resolver (219)
- `M` mergetool and `e` edit on a CONFLICTS row (230-240). `M` is swallowed on other rows (240)
- `e` opens $EDITOR at the cursor (247)
- `Space` or `s` stages; in the diff it stages a hunk (256)
- `d` discard (262); `Ctrl+a` stage all (268)
- `i` gitignore: **no arm**

**Map** (`map.rs:144-172`): `M` opens it (when the graph has focus) and closes it; `Esc` closes; `Up`/`k`, `Down`/`j`; `Enter` seeks.

**Journal** (`journal/keys.rs:78-110`): `Tab`/`BackTab` passed on (returns false); `J`/`Esc` close; `Up`/`k`, `Down`/`j`; `n`/`N` next/prev lost entry; `Space`; `b` branch here; `Enter` revert.
- Find (`journal/find.rs`): `/` starts a find (172); `n`/`N` step (178); `Esc` clears the find (183). In the find prompt: chars, `Backspace`, `Esc`, `Enter` (215-220).

**Stash** (`stash.rs:159-223`): `Ctrl+c` passed on; `Tab`/`BackTab` passed on; `S`/`Esc` close; `Up`/`k`, `Down`/`j`; `Space` preview; `s` save prompt; `Enter` apply; `p` pop; `d` drop; `b` branch.
- Save prompt (370-407): `Esc`, `Tab`/`BackTab` (383), `Enter` save, chars, `Backspace`, `Delete`, `Left`, `Right`, `Home`, `End`.

**Context menu** (`context_menu.rs:133-161`): `j`/`Down`, `k`/`Up`, `Enter`; `Esc` or `.` closes.

**Ghost Preview** (`preview/keys.rs`): `Ctrl+c` passed on (31); `R` (47); `Left`; `Right`/`Tab`/`BackTab` move the cursor (60-61); `Enter` (67); `Esc` closes (68); `y` (79).

**Confirm box** (`confirm.rs:170-191`): `Ctrl+c` passed on; `Left`, `Right`; `Tab`/`BackTab` toggle; `Enter`; `y` only when NONE modifiers and a yes button exists (188); `Esc`.

**Resolver** (`resolver.rs:323-379`, plain unless marked)
- `Ctrl+c` passed on (323); `Esc` (332/363)
- `j`/`Down`, `k`/`Up`; `B` base; `n`/`N` blocks; `Enter` mark resolved; `e` edit; `d` start over; `M` mergetool; `c` finish; `u`/`U` undo answer (379)
- `o` `t` `b` take (`resolver/take.rs:256`); `Ctrl+o`/`Ctrl+t` take all (257-258, `bulk` = Ctrl without Alt, 254); `O`/`T` settle file (259-260)
- `PageDown`, `PageUp`, `g`/`Home`, `G`/`End` (`resolver/scroll.rs:139-146`)
- What still answers after resolved or done: `resolver/resolved.rs:261-264`, `resolver/done.rs:78-80`

**Banner** (`banner/focus_keys.rs`)
- `Tab` (plain) and `BackTab` take focus (26-27); `Esc` drops a wait or clears Sticky (58, 65); `Tab`/`BackTab` move detail focus (71-72)
- While the banner has focus (102-166): `Ctrl+c` passed on; `Tab`/`BackTab`/`Esc` hand the keyboard back; `c` continue; `s` skip; `a` abort; `M` mergetool; `Enter` resolve; `e` edit; `u`/`U`; `q` quit

**Finish summary** (`finish/keys.rs`)
- `Ctrl+c` passed on (19). Ctrl chords go to `control_key`: `Ctrl+Enter` commit (71), `Ctrl+e` edit outside (72), `Ctrl+Left`/`Ctrl+Right` move a word (74-79)
- Alt is swallowed. `Esc` (42); `Shift+Tab` and `BackTab` go back (45/48); `Tab` (51)
- Files: `y` commit, `Enter`/`Space` diff, `j`/`Down`, `k`/`Up`, `PageDown`, `PageUp` (98-103)
- Subject: `Enter` commits (125), plus text-edit keys with `Home`/`End` (132-138). Body: `Enter` newline, arrows, `Home`/`End`, `Delete` (149-158)
- `q` is swallowed: there is no arm

**Rewrite editor** (`rewrite_editor/keys.rs`)
- `Ctrl+c` (43); other Ctrl chords swallowed (158); `Alt+Up`/`Alt+Down` move a row (163-164)
- `Esc` closes; `y` starts; `j`/`Down`, `k`/`Up`; `p` `s` `f` `d` `e` set the verb; `r` opens the field; `Enter` shows changes; `Tab` only when narrow (170-182)
- Field (`keys/field.rs`): `Ctrl+e` $EDITOR (45); `Esc`; `Enter`; text-edit keys with `Home`/`End`/`Delete` (59-84)

**Branch picker** (`picker.rs:104-152`): `Ctrl+c` passed on; `Esc`; `Up`; `Down`; `Backspace`; plain chars filter (147); `Enter` switches; `Tab` shows it on the graph. **No `BackTab`.**

**Blame place** (`place/keys.rs`)
- Live globals passed to the app: `?`, `q`, `Ctrl+c`, `Ctrl+r` (27)
- `j`/`Down`, `k`/`Up`, `PageDown`, `PageUp`, `g`, `G`, `Space` heatmap, `/` find, `n`/`N`, `w` whitespace, `i` ignore-revs (only when an ignore file exists), `Enter`, `^`, `L` line history, `Esc` (53-77)
- Find prompt (94-99): chars, `Backspace`, `Esc`, `Enter`

**History place** (`place/history_keys.rs:29-52`): `j`/`Down`, `k`/`Up`, `PageDown`, `PageUp`, `Tab`, `n`/`N`, `v`, `w`, `L` load all, `B` blame here, `Enter`, `Esc`. **No `/`.**

**Commit editor** (`commit_editor/keys.rs`)
- `Ctrl+c` (17); `Esc` (24/45); `Tab`/`BackTab` (58/62)
- Ctrl chords: `Ctrl+Enter` commit (90), `Ctrl+s` sign-off (91), `Ctrl+Char('C')` co-author (95), `Ctrl+e` edit outside (101), `Ctrl+Left`/`Ctrl+Right` (102/106)
- Subject (209-230): `Enter`, `Down` to body, text keys, `Home`/`End`
- Body (239-254): `Enter` newline, arrows, `Home`/`End`, `Delete`
- Options (263-279): `Space`/`Enter`, `Left`/`Right`, `Up`/`Down`. Button: `Enter`/`Space` (292)

**Credential prompt** (`credential.rs:318-382`): `Ctrl+c` passed on; `Ctrl+u` clears; other Ctrl chords swallowed; Alt passed on (330); `Enter`; `Esc`; chars, `Backspace`, `Delete`, `Left`, `Right`, `Home`, `End`.

**Name prompt** (`name_prompt.rs:255-286`): `Ctrl+c` passed on; `Ctrl+u` clears; `Enter` create; `Esc`; `Tab`/`BackTab` toggle; text keys with `Home`/`End`/`Delete`.

**Drag** (`drag.rs:139`): `Esc` cancels.

**Outside a repo** (`../empty_state.rs`)
- `Ctrl+c` (68); a confirm with `y` or `n`/`Esc` (74-75)
- `q`; `o`; `i` (unless missing); `Enter`; `j`/`Down`; `k`/`Up` (87-117)
- Field (127-149): `Esc`, `Enter`, `Tab`, text keys, `Home`/`End`/`Delete`

## (A) Matched in code but not in BOUND_ACTIONS (`keys/bound.rs:38-170`, checked by key)

- `F(2)`, `F(3)`, `+`, `F`, `y`, `x` are listed. `B` is listed (resolver base).
- **Not listed:**
  - `Home`/`End` (motion_for:43-44, resolver scroll, text fields)
  - `Ctrl+Z` (is_undo_key:150; only the `u`/`U` rows exist)
  - `R` (rewrite and preview) and `X` (commit_graph.rs:93)
  - `w`, `W`, `v`, `[`, `]`, `L` (detail diff options, places)
  - `^` and `i` (blame); `/` and `n`/`N` inside places and the journal find
  - `e` in the rewrite editor, plus its verbs `p` `s` `f` `d` `r`
  - `Alt+Up`/`Alt+Down`; `Ctrl+e`, `Ctrl+s`, `Ctrl+C`, `Ctrl+u`, `Ctrl+Left`/`Ctrl+Right`
  - `s` as a stage alias (detail 256; only `space stage` is listed)
  - `M`/`e` on a CONFLICTS row (only the banner's `M` is listed)
  - `Space` in stash, journal, blame (heatmap), finish
  - `n`/`N` on the journal's lost entries
  - `Tab` in history and outside a repo; `Shift+Tab`/`BackTab` anywhere
  - `h`/`l` and `Left`/`Right` are not in the code at all, so they are not in the list either

## (B) Doc rows: does code answer them?

| Row | Answer |
|---|---|
| §2.2 `?` | No handler. A place passes it on (place/keys.rs:27) and nothing catches it. |
| §2.2 `Ctrl+P`, `:` | No. |
| §2.2 `Tab`/`Shift+Tab` next/prev pane | Only when the detail pane is shown (detail/keys.rs:158,162) or a banner is live (banner/focus_keys.rs:26-27). |
| §2.2 `h`/`l`, `←`/`→` | No (not in motion_for). |
| §2.2 `Ctrl+Shift+Z` | No (predicates.rs:140-150). |
| §2.3 `i` gitignore | No. |
| §2.3 `e` $EDITOR | Only in the changes pane (detail/keys.rs:247). |
| §2.3 `d`/`s` on graph | No. They work only in the changes pane (256, 262). |
| §2.4 `s` alias | Yes (detail/keys.rs:256). |
| §2.4 `i` | No. |
| §2.4 `M` on a CONFLICTS row | Yes (detail/keys.rs:230). |
| §2.5 `w` `W` `v` `[` `]` `L` | Yes (detail/keys.rs:376-408). |
| §2.6 `e`, `M` | Yes (resolver.rs:375,377). |
| §2.6 `Ctrl+O`/`Ctrl+T` | Yes (take.rs:257-258). |
| §2.6 `PgUp`/`PgDn`, `g`/`G`, `Home`/`End` | Yes (scroll.rs:139-146). |
| §2.6 `Esc` | Yes (resolver.rs:363). |
| §2.6a `q` | No: the finish summary swallows it. |
| §2.6a `e` | Plain `e` has no arm; `Ctrl+e` exists (finish/keys.rs:72). |
| §2.6a `Tab` | Yes (finish/keys.rs:51). |
| §2.6b `Ctrl+Enter`, `Ctrl+E` | Yes (finish/keys.rs:71-72). |
| §2.6b `Shift+Tab` | Yes (45/48). |
| §2.6b `PgUp`/`PgDn` | Yes (102-103). |
| §2.7 `Alt+↑/↓` | Yes (rewrite_editor/keys.rs:163-164). |
| §2.7 `Tab` | Only when narrow (182). |
| §2.8 `Space` `i` `w` | Yes (place/keys.rs:59,67,63). |
| §2.9 `s` save, `b` branch, `Space` preview, `d` | Yes (stash.rs:217,223,216,220). |
| §2.10 `Ctrl+Enter`, `Enter`, `Tab`, `Ctrl+E`, `Ctrl+S`, `Esc` | Yes (commit_editor/keys.rs:90, 209, 58, 101, 91, 45). |
| §2.10 `Ctrl+Shift+C` | Matches `Ctrl+Char('C')` (95). It works only under kitty; see the decoder line below. |
| §2.11 `b`, `n`/`N`, `u`/`U`, `J` | Yes (journal/keys.rs:109, 97-98, 29). `u`/`U` reach the global undo (keys.rs:230). |
| §2.11 `/` | Yes, as the journal find (find.rs:172). |
| §2.13 `o` `i` `j`/`k` `q` | Yes (empty_state.rs:87-117). |
| §2.13 `c` `t` `?` | No. |
| §2.14 `Ctrl+U` | Yes in the credential and name prompts (credential.rs:324, name_prompt.rs:258). Not in the other fields. |
| §2.14 `Home`/`End`, `Delete` | Yes in every text field. |
| §2.16 `Tab` | Yes (picker.rs:152). |
| §2.17 `w` `v` `L` `B` `n`/`N` | Yes (history_keys.rs:48,43,49,50,41-42). |

## Decoder: how Ctrl+Shift+letter arrives

- **Legacy bytes:** `0x01..=0x1a` becomes `Char(lowercase)` with `control` set (`crates/graft-term/src/input/decode/mod.rs:170`). Shift is lost, so `Ctrl+Shift+C` arrives as `Char('c')`+ctrl, which is the quit chord.
- **Kitty CSI u:** `Char(uppercase)` with `control` and `shift` set (`crates/graft-term/src/input/decode/csi.rs:141-142`).
- So the commit editor's `Char('C')` (commit_editor/keys.rs:95) and any `Ctrl+Z` fire only under kitty.
- Kitty is opt-in: `crates/graft/src/terminal.rs:159`, with the test "stays off unless asked for" at :596.
