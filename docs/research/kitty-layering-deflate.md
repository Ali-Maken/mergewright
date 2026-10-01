# Kitty graphics: z-layering, deflate, re-placement (for tickets 44, 45, ADR-0053)

Researcher, 2026-10-01. Ghostty source read at shallow clone `d542774` (github.com/ghostty-org/ghostty, main). Paths below are relative to that clone.

## 1. Alpha compositing across z-index

- **Kitty (spec): confirmed.** "Images placed in the same location with different z-index values will be blended if they are semi-transparent." Equal z: lower image id is lower. https://sw.kovidgoyal.net/kitty/graphics-protocol/ (z-index section). Not tested in a running kitty.
- **Ghostty: confirmed in source, not observed.** Placements sorted by `z`, then image id (`src/renderer/image.zig:503-504`); the image pipeline has `blending_enabled = true` (`src/renderer/metal/shaders.zig:32-37`). 1.2.0 release notes list a kitty z-index fix: https://ghostty.org/docs/install/release-notes/1-2-0. OpenGL path not read (question).

## 2. Deflate (`o=z`)

- **Kitty: confirmed.** "only RFC 1950 ZLIB based deflate compression is supported, which is specified using `o=z`." (spec, compression section).
- **Ghostty: confirmed in source.** Parses `z` → `.zlib_deflate` (`src/terminal/kitty/graphics_command.zig:620-621`); decompresses via wuffs zlib (`src/terminal/kitty/graphics_image.zig:629-668`); test at `graphics_image.zig:1098`.
- **Unsupported terminal:** spec says a terminal without the protocol gives *no reply* to `a=q`; detect by sending `a=q` followed by a DA1 query: DA1 answered with no graphics reply = unsupported (spec, "Querying support"). The error reply a protocol-capable terminal sends for an unknown `o=` value: not confirmed (question).
- Caveat: libghostty-vt embedders may drop compressed payloads, e.g. https://github.com/m96-chan/iOSSH/issues/18 (third party, not Ghostty itself).

## 3. Re-placing (`a=p`) without retransmit

- **Kitty: confirmed.** Transmit once with `i=`, then display with `a=p,i=`. "If you send two placements with the same image id and placement id the second one will replace the first. This can be used to resize or move placements around the screen, without flicker." (spec, display section). So a small image can be moved each frame while a large one, with another id, stays put. Replacing a small image's *pixels* each frame still means re-transmitting that small image (follows from the spec; the cost was not measured).
- **Ghostty: confirmed in source.** Placements keyed by (image id, placement id), `getOrPut` replaces (`src/terminal/kitty/graphics_storage.zig:385-400`).

## 4. Known Ghostty issues (not exhaustive; tracker search refused nothing, only a web search was done)

- Classic placements do not scroll with CSI scroll: https://github.com/ghostty-org/ghostty/issues/4323 (open/closed state not checked: question).
- Kitty z-index fix in 1.2.0 (link above): versions before 1.2 may layer wrongly.
- macOS fractional-pixel artifacts with PNG: https://github.com/ghostty-org/ghostty/discussions/7350.
- Umbrella thread: https://github.com/ghostty-org/ghostty/discussions/5218 (not read).
- Kitty itself: first image replacement inside a synchronized update blanks graphics for one frame (kitty 0.48.2, macOS): https://github.com/kovidgoyal/kitty/issues/10538.

## 5. VS Code and Zed terminals

- **VS Code: no kitty graphics.** xterm.js image addon supports **Sixel and iTerm2 inline images** only, off by default, setting `terminal.integrated.enableImages` (since 1.80). https://code.visualstudio.com/docs/terminal/advanced, https://github.com/microsoft/vscode/issues/183563. Neither protocol has z-index layering or placement ids (guess from protocol design; not checked in xterm.js source).
- **Zed: no graphics at all**, as far as found. "Graphics support in the integrated terminal" closed as not planned: https://github.com/zed-industries/zed/issues/20860. Current Zed source not checked (question).

**Implication (guess, for the conductor):** the kitty layer in tickets 44/45 does nothing in the terminals the user lives in (memory: unicode-tier-is-the-daily-view); only the Unicode path serves him daily.
