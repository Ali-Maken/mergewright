#!/usr/bin/env python3
"""m8-polish-and-ship/22: a committable digest of a terminal escape stream.

    python3 scripts/esc-digest.py <stream.esc> > <digest.txt>

Raw terminal streams are never committed (CLAUDE.md "Gates"). What the M1
kitty and iTerm2 checks needed from theirs was *which image commands went out,
in what order, with what payload*: this prints that, one line per command, in
stream order:

    kitty  <n>  <header fields>  payload=<bytes>  sha256=<hex>
    iterm2 <n>  <header fields>  payload=<bytes>  sha256=<hex>

`kitty` is an APC graphics command (ESC _ G <header> ; <payload> ESC \\),
`iterm2` an OSC 1337 File= command (ESC ] 1337 ; File=<header> : <payload> BEL
or ESC \\). The payload is hashed as sent (base64 text), so a digest of the
same stream is byte-identical run to run. Bytes outside image commands are
summarised by count and hash on the last line.
"""
import hashlib
import re
import sys

KITTY = re.compile(rb"\x1b_G([^;\x1b]*)(?:;([^\x1b]*))?\x1b\\", re.S)
ITERM = re.compile(rb"\x1b\]1337;File=([^:\x07\x1b]*):([^\x07\x1b]*)(?:\x07|\x1b\\)", re.S)


def main(path):
    data = open(path, "rb").read()
    found = []
    for kind, pattern in (("kitty", KITTY), ("iterm2", ITERM)):
        for m in pattern.finditer(data):
            found.append((m.start(), m.end(), kind, m.group(1), m.group(2) or b""))
    found.sort()
    rest = bytearray()
    at = 0
    print(f"# digest of {path.rsplit('/', 1)[-1]}: {len(data)} bytes, {len(found)} image commands")
    for n, (start, end, kind, header, payload) in enumerate(found, 1):
        rest += data[at:start]
        at = end
        fields = header.decode("ascii", "replace")
        digest = hashlib.sha256(payload).hexdigest()
        print(f"{kind:6} {n:3}  {fields}  payload={len(payload)}  sha256={digest}")
    rest += data[at:]
    print(f"# other bytes: {len(rest)}  sha256={hashlib.sha256(rest).hexdigest()}")


if __name__ == "__main__":
    main(sys.argv[1])
