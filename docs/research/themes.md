# Prior art: themes (M8, pipeline step 2)

Researched 2026-10-02 for m8-polish-and-ship. Facts carry a link; **Guess** / **Question** marks what I did not confirm. GitHub READMEs were read via `gh api` (WebFetch got HTTP 503 from github.com). gnu.org refused the connection, so Magit/Emacs is unconfirmed.

## Need (from the repo)

- Spec: PROMPT.md:374–379 — tokens `bg … lane-1…12`; ship dark, light, high-contrast; auto-pick via OSC 11 with manual override; OSC 11 is a light/dark hint only; lanes independent of theme; `NO_COLOR`; truecolor → 256 → 16 with perceptual mapping; conflict tokens get fixed 256 entries; tints dropped at 16 colours / high contrast / `NO_COLOR`.
- User: dark theme; Zed / VS Code terminals, Ghostty (CLAUDE.md memory). docs/field-notes.md:44,48,50 — the cursor row was unreadable in dark (`selection` vs `bg` 1.21:1 → fixed to 3.04:1). Lesson recorded at :50: ask theme and terminal.

## Per tool

### delta
- Chosen by: `--dark` / `--light` flags; `--syntax-theme` (defaults to `BAT_THEME`). https://dandavison.github.io/delta/full---help-output.html
- Auto: `--detect-dark-light auto|always|never`, which queries the terminal (OSC 11). The docs warn it needs exclusive terminal access, which conflicts with pagers. Same page.
- Depth: `--true-color auto` emits 24-bit only if `COLORTERM` is `truecolor`/`24bit`. Same page.
- One colour: per-style options, e.g. `--minus-style`, `--plus-style` (default `normal auto` / `syntax auto`). Same page.
- Complaints: auto-detect fails when run inside lazygit: https://github.com/dandavison/delta/issues/1968 (open), https://github.com/jesseduffield/lazygit/issues/4550. Users ask for both light and dark set at once: https://github.com/dandavison/delta/issues/1678, /1976; docs on per-mode settings: /2024; a 16-colour theme: /2068 (titles only read).

### bat
- Chosen by: `--theme`, `BAT_THEME`, `--list-themes`; custom `.tmTheme` files in `$(bat --config-dir)/themes`. https://github.com/sharkdp/bat/blob/master/README.md ("Highlighting theme", "Adding new themes")
- Auto: picks a theme from the terminal background. `--theme-dark` / `--theme-light` and `BAT_THEME_DARK` / `BAT_THEME_LIGHT` choose the pair; `--theme auto:system` follows the OS. Same README.
- Palette inheritance: the `ansi`, `base16` and `base16-256` themes use only terminal palette colours. Their stated wins: maximum compatibility, they "adapt to terminal theme changes. Even for already printed output", and they blend with other terminal software. Same README, "8-bit themes".
- Depth: truecolor is strongly recommended. It is detected from `COLORTERM`. Same README.

### lazygit
- Chosen by: a `gui.theme` block in `config.yml`. Each key takes a list of attributes, with at most one colour. Defaults use ANSI names (`activeBorderColor: [green, bold]`, `selectedLineBgColor: [blue]`, `defaultFgColor: [default]`). https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md ("Color Attributes", "Highlighting the selected line")
- No light/dark themes and no auto-detection. **Question:** does any release do it? Request #4366 is closed, but I did not read how it was resolved: https://github.com/jesseduffield/lazygit/issues/4366
- One colour: set the key, e.g. `selectedLineBgColor: [default]` (bold only) or `reverse`. Config.md as above.
- Complaints:
  - Unreadable selected item on light terminals: https://github.com/jesseduffield/lazygit/issues/4306 (open).
  - Default colours hard to read on dark-on-light: /508.
  - "Use foreground instead of white": /2255; default text colour: /1639 (titles only).
  - This is the same failure the user hit in Graft (field-notes:48).

### gitui (ratatui, the closest stack)
- Chosen by: `theme.ron`, a partial override (since 0.23 only the changed keys are needed); `-t <file>` picks a named theme file; Catppuccin presets are external. https://github.com/gitui-org/gitui/blob/master/THEMES.md
- Docs note that named colours (`yellow`) take whatever the terminal palette defines, and that RGB may be unsupported. Same file.
- Auto light/dark: none documented. **Guess:** none exists.
- Complaints:
  - UI invisible on Solarized (text same colour as background): https://github.com/gitui-org/gitui/issues/1360
  - Light-theme contrast of the selected branch: /922
  - Selected tab hard-coded white, can't be themed: /719

### helix
- Chosen by: `theme = "<name>"` in config, or `:theme <name>` live. `inherits = "<base>"` overrides single keys; a `[palette]` holds named colours, merged over the 16 terminal defaults. https://docs.helix-editor.com/themes.html
- Auto light/dark and truecolor fallback: not on that page. **Question.**

### zellij
- Chosen by: `theme <name>`; `theme_dark` / `theme_light` (both are required for auto-switching). https://github.com/zellij-org/zellij-org.github.io/blob/main/docs/src/themes.md
- Auto: uses the terminal's colour-scheme report (`DSR 997` replies, `CSI ? 2031` change notifications), not OSC 11. It switches live. Terminals that don't report never switch. `explicit_theme_hue "dark"` pins the mode, for terminals that report wrongly or not at all. Manual `ToggleTheme` action. Same file.
- **Question:** do Ghostty, Zed and VS Code terminals answer `CSI ? 996 n` / 2031? Not checked.

### btop
- Chosen by: theme files in system and user `themes/` dirs, or `--themes-dir`. https://github.com/aristocratos/btop/blob/main/README.md ("Themes")
- Depth: 24-bit by default. `truecolor = false` or `-l/--low-color` converts to 256 colours (6×6×6 cube). A real TTY gets 16 colours, forced with `-t/--tty`. Same README.

### VS Code (GUI, plus the user's terminal host)
- Picker: `⌘K ⌘T`, with live preview. `window.autoDetectColorScheme` follows the OS. The preferred dark / light / high-contrast themes are settings, and `window.autoDetectHighContrast` exists. One colour: `workbench.colorCustomizations`, scoped per theme. https://code.visualstudio.com/docs/configure/themes
- **Terminal host fact that matters to Graft:** `terminal.integrated.minimumContrastRatio` defaults to **4.5** and raises or lowers a foreground colour's luminance until it meets 4.5:1. ANSI colours come from the VS Code theme. https://code.visualstudio.com/docs/terminal/appearance
  - **Guess:** in VS Code, Graft's foregrounds (lanes, `text-muted`) are repainted by the host. Backgrounds such as `selection` are not.

### Zed (the user's other terminal host)
- `theme: { mode: system|light|dark, dark, light }`; the terminal's `minimum_contrast` defaults to `1.0` (off). https://zed.dev/docs/reference/all-settings
- So the same Graft theme renders differently in Zed and VS Code.

### JetBrains
- Settings → Appearance → Theme, or the Quick Switch popup (Ctrl+`). "Sync with OS" picks dark or light. A High Contrast theme exists, "designed for users with color vision deficiency". https://www.jetbrains.com/help/idea/user-interface-themes.html

### Sublime Merge
- Themes are `.sublime-theme` JSON; to override, put a same-named file in `Packages/User/`. Colour schemes for diffs are separate from the UI theme. https://www.sublimemerge.com/docs/themes
- **Question:** is there an auto light/dark setting? Not on that page; `command_line` docs silent.

### Magit
- **Question:** not confirmed (gnu.org refused the connection). **Guess:** faces are set via `M-x customize-face`; light vs dark comes from Emacs `frame-background-mode`.

## Journeys (steps, from "using the app" to done)

The counts come from the docs above. Editing a file counts as 1 step, restarting as 1.

| Tool | Switch to light | Fix one unreadable colour |
|---|---|---|
| VS Code | 0 if auto is on; otherwise `⌘K⌘T`, pick, ⏎ = 3 | find the key name (1), edit `colorCustomizations` (1) = 2+ (finding the name is the hard part) |
| JetBrains | 0 with Sync; else Ctrl+`, pick = 2–3 | **Question** |
| zellij | 0 if the terminal reports; else `ToggleTheme` = 1 | edit the KDL theme (1); live = 1–2 |
| bat | 0 (auto); `--theme` = 1 | not per colour: write a `.tmTheme` (many) |
| delta | 0 if detection works, else edit `--light` in gitconfig = 1–2 | edit one `*-style` (1) |
| helix | `:theme x` = 1 | `inherits` + 1 key, then `:theme` = 2–3 |
| lazygit | not possible as a theme; edit about 4–10 keys by hand + restart | edit 1 key + restart = 2 |
| gitui | write or download a `theme.ron` + `-t` restart = 2–3 | edit 1 key in `theme.ron` + restart = 2 |
| btop | change theme in the options menu: **Question** on the count | edit theme file + reload: **Question** |

Best: 0 steps (auto) with a 1-step live toggle (zellij, VS Code). The worst case, lazygit, is the source of its top readability complaints.

## Praise and complaints (patterns)

- **Selected line unreadable on the other background** (lazygit #4306, #508; gitui #922). This is Graft's own field-notes:48 bug.
- **Hard-coded white or default fg invisible** (lazygit #2255, gitui #719, #1360).
- **Auto-detect breaks when nested or piped** (delta #1968, lazygit #4550). zellij documents the "terminal doesn't report" case and gives a pin.
- **Palette-inheriting themes are praised** for matching the terminal and surviving theme switches (bat README). They give up control of contrast.

## Take / reject / invent candidates (for the planner)

- **Take:**
  - Auto-pick with a dark/light pair plus a pin (zellij `theme_dark` / `theme_light` / `explicit_theme_hue`; bat `--theme-dark` / `--theme-light`).
  - One live toggle key (zellij `ToggleTheme`, helix `:theme`). Key must come from 02-keymap.md.
  - Partial override of named tokens (gitui 0.23 partial `theme.ron`, helix `inherits`, VS Code `colorCustomizations`). Graft's tokens are already the names.
  - Depth from `COLORTERM` (delta, bat), with an explicit override (btop `-l`, `-t`).
- **Take, check first:** listen for `CSI ? 2031` change notifications as well as OSC 11. **Question:** do the user's three terminals support it?
- **Reject:**
  - Attribute-list config per widget (lazygit). It produced the light-terminal complaints.
  - Theme-only-by-file with no built-in light theme (gitui).
  - Full-palette inheritance as the default: it loses the contrast floor Graft needs for `selection`. Keep it, if at all, as an "ansi" option for 16 colours.
- **Invent:**
  - A contrast check in tests for every theme, including user overrides at load time: `selection` vs `bg` ≥ 3:1, text ≥ 4.5:1. Warn by name on a failing override. No tool found does this. Graft has already shipped the bug once (field-notes:48–50).
  - **Question for the design:** does VS Code's default 4.5 minimum-contrast setting shift Graft's lane hues? Test it in the user's VS Code before tuning values.
