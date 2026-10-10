"""m1-the-graph/40 capture: drive target/release/graft in a pty forced to the
pixel tier (Kitty protocol), decode the images it transmits and composite the
placements on screen at the end into one PNG, at a 17x37 cell on a dark
backdrop. Writes the file only; never prints image contents.

usage: python3 docs/shots/40-capture.py <repo> <cols> <rows> <keys> <out.png>
keys: sent one at a time, 0.4 s apart; 'q' is appended to quit.
Text is not drawn: the PNG shows the graph images where the terminal would
place them, which is what the pixel tier's look is judged on.
"""
import base64, fcntl, os, pty, re, select, struct, sys, tempfile, termios, time, zlib
# Every Graft launch goes through sandbox_env (m8/22b): temp XDG_CONFIG_HOME, checked.
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "scripts"))
from trib_sandbox import sandbox_env  # noqa: E402

repo, cols, rows, keys, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4], sys.argv[5]
CELL_W, CELL_H = 17, 37
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
binary = os.path.join(root, "target", "release", "graft")

# Forced to the pixel tier by a temp config folder (was target/40/cfg, which
# lived outside temp and was never checked in).
config_home = tempfile.mkdtemp(prefix="graft-40-")
os.makedirs(os.path.join(config_home, "graft"))
with open(os.path.join(config_home, "graft", "config.toml"), "w") as f:
    f.write('[graph]\nrenderer = "pixel"\n')
pid, fd = pty.fork()
if pid == 0:
    os.execve(binary, [binary, repo], sandbox_env(config_home, TERM="xterm-kitty", COLORTERM="truecolor"))
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, cols * CELL_W, rows * CELL_H))

buf = bytearray()
def pump(seconds):
    end = time.time() + seconds
    while time.time() < end:
        ready, _, _ = select.select([fd], [], [], 0.05)
        if ready:
            try:
                data = os.read(fd, 1 << 16)
            except OSError:
                return
            if not data:
                return
            buf.extend(data)

pump(6.0)
for key in keys:
    os.write(fd, key.encode()); pump(0.4)
pump(2.0)
screen_end = len(buf)  # quitting deletes every image; composite the frame before it
os.write(fd, b"q"); pump(1.0)

token = re.compile(rb"\x1b_G([^;\x1b]*)(?:;([^\x1b]*))?\x1b\\|\x1b\[(\d*)(?:;(\d*))?H")
images, pending, places = {}, {}, {}
cursor = (0, 0)
for match in token.finditer(bytes(buf[:screen_end])):
    if match.group(1) is None:
        cursor = (int(match.group(3) or 1) - 1, int(match.group(4) or 1) - 1)
        continue
    keys_ = dict(kv.split(b"=", 1) for kv in match.group(1).split(b",") if b"=" in kv)
    action = keys_.get(b"a", b"t")
    image = keys_.get(b"i")
    if action in (b"t", b"T") or (image is None and pending):
        if image is not None:
            pending["id"], pending["s"], pending["v"], pending["data"] = image, int(keys_[b"s"]), int(keys_[b"v"]), bytearray()
        pending["data"].extend(match.group(2) or b"")
        if keys_.get(b"m", b"0") == b"0":
            images[pending["id"]] = (pending["s"], pending["v"], base64.b64decode(bytes(pending["data"])))
            pending = {}
    elif action == b"p":
        crop = tuple(int(keys_.get(k, b"0")) for k in (b"x", b"y", b"w", b"h"))
        places[(image, keys_.get(b"p"))] = (image, cursor, crop)
    elif action == b"d":
        for key in [key for key in places if image is None or key[0] == image]:
            del places[key]

width, height = cols * CELL_W, rows * CELL_H
canvas = bytearray(bytes((0x1E, 0x1E, 0x2E)) * (width * height))
for image, (row, col), (sx, sy, sw, sh) in places.values():
    if image not in images:
        continue
    s, v, rgba = images[image]
    sw, sh = sw or s - sx, sh or v - sy
    for y in range(sh):
        cy = row * CELL_H + y
        if not 0 <= cy < height or sy + y >= v:
            continue
        for x in range(sw):
            cx = col * CELL_W + x
            i = ((sy + y) * s + sx + x) * 4
            a = rgba[i + 3] if sx + x < s else 0
            if a == 0 or not 0 <= cx < width:
                continue
            o = (cy * width + cx) * 3
            for c in range(3):
                canvas[o + c] = (rgba[i + c] * a + canvas[o + c] * (255 - a)) // 255

def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
raw = b"".join(b"\x00" + bytes(canvas[y * width * 3:(y + 1) * width * 3]) for y in range(height))
with open(out, "wb") as f:
    f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))
drawn = sorted({y // CELL_H for y in range(height) if any(canvas[(y * width + x) * 3] != 0x1E for x in range(0, width, 3))})
print(f"{out}: {width}x{height}, {len(images)} images decoded, {len(places)} placed, screen rows with graph pixels: {len(drawn)} ({drawn[:1]}..{drawn[-1:]})")
