# macOS and Linux only; Windows is a non-goal for v1

Windows support is not free: it means a third CI matrix leg, path and
line-ending special cases, a different PTY story, and a terminal-graphics
landscape that barely overlaps with the one the pixel graph targets. The user
runs macOS and Linux, so v1 promises exactly those two and pays for nothing
else — no Windows CI, no Windows terminals in the manual checklist.

## Consequences

Platform-specific code still lives behind the module boundaries in PROMPT.md
section 6, so Windows stays possible later; it just is not claimed, tested or
blocked on now.
