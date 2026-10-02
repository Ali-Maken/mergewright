# Prior art: keybinding help and remapping (M8, pipeline step 2)

Researched 2026-10-02. Every fact carries a link. **Guess** / **Question** = not confirmed from a primary source.
github.com web pages returned HTTP 503 to the fetcher; GitHub content was read through `gh api` (same files, raw).

## Graft today (for comparison)

- `?` = "Context help for the focused pane"; `Ctrl+P`/`:` = command palette — `docs/design/02-keymap.md:176-177`.
- Palette fuzzy-filters "actions, branches, tags, files and commits" — `docs/design/02-keymap.md:480-487`.
- "Every binding is remappable in `[keys]` in `config.toml`; Graft detects and warns about remap collisions at startup rather than picking silently" — `docs/design/02-keymap.md:168-169`. Whether this is built is a **Question** (not checked in code; scout's job).
- Standing reference already recommends: hint bar + palette that teaches the shortcut — `docs/research/ux-principles.md:82-90`.

## Per tool

### lazygit
- Help: always-visible bottom line "that contains keybinding info" (`showBottomLine: true`) — [Config.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md). `?` "Open keybindings menu" — [Keybindings_en.md](https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md). Menu is grouped local / global / navigation — [#3001](https://github.com/jesseduffield/lazygit/issues/3001). Whether the `?` menu can be filtered by typing: **Question**.
- Remap: YAML `keybinding:` with per-context sections (`universal:`, …), e.g. `optionMenu: '?'`; disable with `<disabled>` — [Config.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md). Whether bottom line / `?` menu show the remapped key: **Guess** yes (they render from the binding table), unconfirmed.
- Complaints:
  - `?` didn't tell a user how to continue a rebase; asks for "a fuzzy-searchable modal view that is contextual to the current mode" — [#3141](https://github.com/jesseduffield/lazygit/issues/3141).
  - Request for a which-key style interface — [#1219](https://github.com/jesseduffield/lazygit/issues/1219).
  - Hidden default `-alt` bindings silently override a user's remap ("invisibly overwritten") — [#5763](https://github.com/jesseduffield/lazygit/issues/5763).
  - Validation error for a binding that does not exist — [#5697](https://github.com/jesseduffield/lazygit/issues/5697).

### gitui
- Remap: `key_bindings.ron` (RON, crossterm `KeyCode` + modifiers), shipped vim-style file to copy; help key is itself remappable (`open_help: F(1)`); key *symbols* shown in UI are configurable separately — [KEY_CONFIG.md](https://github.com/gitui-org/gitui/blob/master/KEY_CONFIG.md).
- Complaint: official vim config remaps `Shift+U` to reset, but default `undo_commit` keeps `Shift+U`; same key does different things in two panes, "easy to … accidentally undo a commit". No conflict detection — [#2704](https://github.com/gitui-org/gitui/issues/2704). Config broke on upgrade — [#2059](https://github.com/gitui-org/gitui/issues/2059).

### tig
- Remap: `bind <keymap> <key> <action>` in `.tigrc`; keymaps per view (`main`, `diff`, `status`, …) plus `generic`; unbind with `none` — [tigrc(5)](https://jonas.github.io/tig/doc/tigrc.5.html). Help view `h` lists bindings; whether it includes user binds: **Question**.
- Complaint: a view-specific `none` beats a `generic` bind, so you can't truly clear a key — [#816](https://github.com/jonas/tig/issues/816); "how to rebind refresh action?" — [#1434](https://github.com/jonas/tig/issues/1434).

### Magit / transient
- Help: every command family opens a popup ("menu") listing keys with descriptions — [Transient manual](https://docs.magit.vc/transient/). `C-h` then a key shows help for that suffix (§2.6, same link). Remapping is Lisp (§3 "Modifying Existing Transients", §4.3) — not end-user config. Whether popups reflect user-rebound suffix keys: **Guess** yes (popup is drawn from the definition).

### which-key (Emacs; Neovim folke/which-key similar — not fetched)
- Popup "displays the key bindings following your currently entered incomplete command (a prefix)"; default delay 1 s (`which-key-idle-delay`); paging with `C-h`; reads live keymaps so remaps "display instantly" — [emacs-which-key](https://github.com/justbur/emacs-which-key).

### helix
- Remap: `~/.config/helix/config.toml`, `[keys.normal]`/`[keys.insert]`/`[keys.select]`; minor modes as nested tables (`[keys.normal.g]`); "one-way key remapping" — [remapping](https://docs.helix-editor.com/remapping.html). Info boxes toggled by `auto-info` — [editor](https://docs.helix-editor.com/editor.html). Whether remaps appear in the info box: **Question**. Request to configure info-box position — [#11572](https://github.com/helix-editor/helix/issues/11572).

### zellij
- Remap: KDL, keybinds "divided into modes"; several keys may bind one action — [keybindings](https://zellij.dev/documentation/keybindings.html). Startup tips, `mouse_hover_tips` "contextual help tips" — [options](https://zellij.dev/documentation/options.html). Status bar shows per-mode hints — **Question** whether it reflects remapped keys (not in fetched pages).

### VS Code
- Keyboard Shortcuts editor: search by command name *or by the key itself*; right-click "Show Same Keybindings" to see conflicts; `keybindings.json` entries `key` / `command` / `when` (context); remove a default with `"-command"`; troubleshooting log shows which key was detected and which command ran — [keybindings](https://code.visualstudio.com/docs/configure/keybindings). Command Palette `⇧⌘P` — [UI](https://code.visualstudio.com/docs/getstarted/userinterface); whether each palette row shows its shortcut: **Question** (fetched text doesn't say; widely seen, unconfirmed here).

### Sublime Merge
- `.sublime-keymap` JSON array, user file `Packages/User/Default.sublime-keymap`, `"context"` conditions (`merge_mode`, `has_staged`, …); "Currently there is no compiled list of all built-in commands" — [key_bindings](https://www.sublimemerge.com/docs/key_bindings). Palette showing keys: **Question**.

### JetBrains
- Find Action by Shortcut: press the combo to find the action; assigning shows "a warning if it conflicts with existing shortcuts" and offers to remove it; right-click "Reset Shortcuts"; editing a predefined keymap makes a copy; Keymap page searchable by action name — [IntelliJ keymap](https://www.jetbrains.com/help/idea/configuring-keyboard-and-mouse-shortcuts.html).

## Journeys (step-counted, per ux-principles §1)

"Find the key for X" (X = an action the user knows by name):
| Tool | Steps | Source |
|---|---|---|
| lazygit | `?` → scan grouped list (→ filter?) → read key: 2–3 | links above; filter is a Question |
| VS Code / JetBrains | open palette/Find Action → type name → read shortcut: 3 | palette shortcut display is a Question for VS Code |
| which-key / transient | press prefix → wait/read popup: 2 (only if you know the prefix) | which-key, transient |
| Graft now | `?` → scan, or `Ctrl+P` → type → (key shown? **Question**): 2–3 | 02-keymap.md:176-177 |

"Rebind X to Y":
| Tool | Steps | Source |
|---|---|---|
| JetBrains | settings → Keymap → search X → right-click Add → press Y → resolve conflict dialog → OK: ~7, conflict caught at the moment | JetBrains link |
| VS Code | open editor → search X → double-click → press Y → Enter: 5; conflicts only if you ask ("Show Same Keybindings") | VS Code link |
| lazygit / gitui / tig / helix / zellij / Sublime Merge | find the action's *config name* in docs → open file → write line → restart/reload → test: ~5, plus lookup; no conflict check found | links above |
| Graft (designed) | edit `[keys]` → restart → read collision warning: ~4 + name lookup | 02-keymap.md:168-169 |

## What hurts, across tools
- Silent conflicts: remap leaves an old default live elsewhere — gitui #2704, lazygit #5763.
- Can't fully clear a key — tig #816.
- Help that lists keys but not "what next" in the current mode — lazygit #3141.
- Finding the config name of an action is a docs hunt — Sublime Merge "no compiled list"; gitui points at `key_list.rs` source.

## Take / reject / invent candidates (for the planner)
- **Take** search-by-name *and* search-by-key in one help view (VS Code, JetBrains Find by Shortcut).
- **Take** `?` filtered to the focused pane's live keys, grouped (lazygit) — answers #3141 if it covers mode state (operation in progress).
- **Take** hints/help rendered from the live binding table so remaps propagate (which-key "display instantly"). Must be a test: remap → hint shows new key.
- **Take** conflict check at load naming both actions and panes (JetBrains warns at assign time; Graft already specifies startup warning, 02-keymap.md:168-169). Must include defaults still bound (gitui #2704, lazygit #5763).
- **Take** a way to unbind that wins everywhere (tig #816 counter-example).
- **Take** palette rows showing their key (ux-principles §3.4) — confirm VS Code behaviour before citing it.
- **Reject** GUI keymap editor with per-key record dialog (JetBrains/VS Code) — heavy for a TUI; **Question** for user whether in-app rebinding is wanted at all vs file.
- **Reject** Lisp/code-level remapping (transient).
- **Invent?** `?` shows each action's config name next to its key so "rebind X" needs no docs lookup — nothing found does this; **Guess** it cuts the rebind journey by the lookup step.
- **Invent?** which-key-style delay popup is a poor fit: Graft has single-key verbs, few prefixes (02-keymap.md:12-16) — reject unless prefixes are added.
