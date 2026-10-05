#!/usr/bin/env python3
"""Generate every TokenBurn icon asset from one drawing (needs Pillow).

    python3 scripts/gen_icons.py

Outputs (all committed, so CI never needs Pillow):
  assets/icons/tokenburn.svg            vector version of the same drawing
  assets/icons/tokenburn.png            256x256
  assets/icons/tokenburn-<N>.png        16..1024 (Linux hicolor sizes + macOS iconset)
  packaging/windows/tokenburn.ico       16/24/32/48, classic BMP entries (NSIS-safe)
  packaging/macos/tokenburn.icns        via `iconutil` (macOS only; skipped elsewhere)
  crates/tokenburn-gui/assets/icon-64.rgba   raw RGBA for the window icon
"""
import math, os, shutil, struct, subprocess, sys, tempfile
from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
S = 2048  # supersampled canvas

def bezier(p0, p1, p2, p3, n=64):
    out = []
    for i in range(n + 1):
        t = i / n
        u = 1 - t
        out.append((
            u**3*p0[0] + 3*u*u*t*p1[0] + 3*u*t*t*p2[0] + t**3*p3[0],
            u**3*p0[1] + 3*u*u*t*p1[1] + 3*u*t*t*p2[1] + t**3*p3[1],
        ))
    return out

def flame(scale, ox, oy):
    """Flame outline in a 0..1 box, scaled/offset into the canvas."""
    pts = []
    # start at the bottom centre, go up the right side to the tip, down the left
    pts += bezier((0.50, 0.97), (0.86, 0.97), (0.95, 0.62), (0.72, 0.38))   # right belly
    pts += bezier((0.72, 0.38), (0.64, 0.28), (0.66, 0.14), (0.58, 0.03))   # right to tip
    pts += bezier((0.58, 0.03), (0.40, 0.16), (0.40, 0.30), (0.36, 0.40))   # tip down left
    pts += bezier((0.36, 0.40), (0.30, 0.34), (0.28, 0.28), (0.29, 0.20))   # small left lick
    pts += bezier((0.29, 0.20), (0.10, 0.36), (0.05, 0.60), (0.18, 0.78))   # left belly
    pts += bezier((0.18, 0.78), (0.26, 0.92), (0.38, 0.97), (0.50, 0.97))   # bottom
    return [(ox + x*scale, oy + y*scale) for x, y in pts]

def svg():
    """Same drawing as an SVG (viewBox 0 0 1024 1024)."""
    def path(scale, ox, oy):
        segs = [
            ((0.50, 0.97), (0.86, 0.97), (0.95, 0.62), (0.72, 0.38)),
            ((0.72, 0.38), (0.64, 0.28), (0.66, 0.14), (0.58, 0.03)),
            ((0.58, 0.03), (0.40, 0.16), (0.40, 0.30), (0.36, 0.40)),
            ((0.36, 0.40), (0.30, 0.34), (0.28, 0.28), (0.29, 0.20)),
            ((0.29, 0.20), (0.10, 0.36), (0.05, 0.60), (0.18, 0.78)),
            ((0.18, 0.78), (0.26, 0.92), (0.38, 0.97), (0.50, 0.97)),
        ]
        f = lambda p: f"{ox + p[0]*scale:.1f} {oy + p[1]*scale:.1f}"
        d = "M " + f(segs[0][0])
        for a, b, c, e in segs:
            d += f" C {f(b)}, {f(c)}, {f(e)}"
        return d + " Z"
    k = 1024 / S
    box = S*0.64
    ibox = box*0.52
    outer = path(box*k, (S-box)/2*k, S*0.17*k)
    inner = path(ibox*k, ((S-ibox)/2 + S*0.012)*k, (S*0.17 + box*0.47)*k)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="256" height="256">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1e2232"/><stop offset="1" stop-color="#0e1018"/></linearGradient>
    <linearGradient id="fo" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffc440"/><stop offset="1" stop-color="#e84028"/></linearGradient>
    <linearGradient id="fi" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff096"/><stop offset="1" stop-color="#ffb03c"/></linearGradient>
  </defs>
  <rect width="1024" height="1024" rx="230" fill="url(#bg)"/>
  <path d="{outer}" fill="url(#fo)"/>
  <path d="{inner}" fill="url(#fi)"/>
</svg>
"""

def vgrad(size, top, bottom):
    img = Image.new("RGBA", size)
    px = img.load()
    for y in range(size[1]):
        t = y / (size[1] - 1)
        c = tuple(int(top[i] + (bottom[i]-top[i])*t) for i in range(4))
        for x in range(size[0]):
            px[x, y] = c
    return img

def draw():
    base = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    # rounded-square background (dark, subtle gradient)
    bg = vgrad((S, S), (30, 34, 50, 255), (14, 16, 24, 255))
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, S-1, S-1), radius=int(S*0.225), fill=255)
    base.paste(bg, (0, 0), mask)

    # outer flame: orange -> red
    box = S * 0.64
    ox, oy = (S - box)/2, S*0.17
    outer = Image.new("L", (S, S), 0)
    ImageDraw.Draw(outer).polygon(flame(box, ox, oy), fill=255)
    grad = vgrad((S, S), (255, 196, 64, 255), (232, 64, 40, 255))  # top light -> bottom red
    # glow
    glow = outer.filter(ImageFilter.GaussianBlur(S*0.03))
    glow_img = Image.new("RGBA", (S, S), (255, 120, 40, 0))
    glow_img.putalpha(glow.point(lambda v: int(v*0.35)))
    base = Image.alpha_composite(base, glow_img)
    base.paste(grad, (0, 0), outer)

    # inner flame: yellow -> pale
    ibox = box * 0.52
    iox, ioy = (S - ibox)/2 + S*0.012, S*0.17 + box*0.47
    inner = Image.new("L", (S, S), 0)
    ImageDraw.Draw(inner).polygon(flame(ibox, iox, ioy), fill=255)
    igrad = vgrad((S, S), (255, 240, 150, 255), (255, 176, 60, 255))
    base.paste(igrad, (0, 0), inner)
    return base

def main():
    master = draw()
    out_icons = os.path.join(ROOT, "assets", "icons")
    os.makedirs(out_icons, exist_ok=True)
    sizes = [16, 22, 24, 32, 48, 64, 96, 128, 256, 512, 1024]
    imgs = {}
    for n in sizes:
        imgs[n] = master.resize((n, n), Image.LANCZOS)
        imgs[n].save(os.path.join(out_icons, f"tokenburn-{n}.png"))
    imgs[256].save(os.path.join(out_icons, "tokenburn.png"))
    with open(os.path.join(out_icons, "tokenburn.svg"), "w") as f:
        f.write(svg())

    # Windows .ico. Classic BMP/DIB entries (16..48): NSIS' legacy icon loader
    # rejects PNG-compressed entries, so we avoid those on purpose.
    def dib(img):
        w, h = img.size
        px = img.convert("RGBA").load()
        header = struct.pack("<IiiHHIIiiII", 40, w, h*2, 1, 32, 0, w*h*4, 0, 0, 0, 0)
        rows = b""
        for y in range(h-1, -1, -1):
            for x in range(w):
                r, g, b, a = px[x, y]
                rows += bytes((b, g, r, a))
        mask_row = ((w + 31)//32)*4
        return header + rows + bytes(mask_row*h)
    ico_sizes = [16, 24, 32, 48]
    blobs = [dib(imgs[n]) for n in ico_sizes]
    hdr = struct.pack("<HHH", 0, 1, len(ico_sizes))
    off = 6 + 16*len(ico_sizes)
    entries = b""
    for n, blob in zip(ico_sizes, blobs):
        entries += struct.pack("<BBBBHHII", n, n, 0, 0, 1, 32, len(blob), off)
        off += len(blob)
    with open(os.path.join(ROOT, "packaging", "windows", "tokenburn.ico"), "wb") as f:
        f.write(hdr + entries + b"".join(blobs))

    # raw RGBA for the iced window icon
    with open(os.path.join(ROOT, "crates", "tokenburn-gui", "assets", "icon-64.rgba"), "wb") as f:
        f.write(imgs[64].tobytes())

    # macOS .icns through iconutil
    if shutil.which("iconutil"):
        with tempfile.TemporaryDirectory() as td:
            iconset = os.path.join(td, "tokenburn.iconset")
            os.makedirs(iconset)
            for n in (16, 32, 128, 256, 512):
                imgs[n].save(os.path.join(iconset, f"icon_{n}x{n}.png"))
                imgs[n*2].save(os.path.join(iconset, f"icon_{n}x{n}@2x.png"))
            subprocess.run(["iconutil", "-c", "icns", iconset, "-o",
                            os.path.join(ROOT, "packaging", "macos", "tokenburn.icns")], check=True)
    else:
        print("iconutil not found — skipped tokenburn.icns", file=sys.stderr)
    print("icons written")

if __name__ == "__main__":
    main()
