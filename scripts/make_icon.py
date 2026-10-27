"""Vẽ biểu tượng ứng dụng (giọt nước navy + cam theo palette đặc tả 6.3) và xuất icon.png, icon.ico, icon.icns.
Chạy một lần khi cần đổi biểu tượng:  python3 scripts/make_icon.py   (cần Pillow; .icns cần `iconutil` của macOS).
"""
import math
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw

OUT = Path("crates/app/assets")
S = 2048  # vẽ lớn rồi thu nhỏ để mép mịn
NAVY_TOP, NAVY_BOTTOM, ORANGE, WHITE = (0x3E, 0x5C, 0x91), (0x1C, 0x2F, 0x52), (0xFF, 0x8C, 0x32), (255, 255, 255)


def scale(v):
    return v * S / 1024


def background():
    grad = Image.new("RGB", (S, S))
    px = grad.load()
    for y in range(S):
        t = y / (S - 1)
        c = tuple(round(NAVY_TOP[i] + (NAVY_BOTTOM[i] - NAVY_TOP[i]) * t) for i in range(3))
        for x in range(S):
            px[x, y] = c
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=round(S * 0.225), fill=255)
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    img.paste(grad, (0, 0), mask)
    return img


def drop_mask():
    cx, cy, r, apex = scale(512), scale(610), scale(250), scale(150)
    d = cy - apex
    ang = math.acos(r / d)  # góc giữa trục dọc và tiếp điểm
    tl = (cx - r * math.sin(ang), cy - r * math.cos(ang))
    tr = (cx + r * math.sin(ang), cy - r * math.cos(ang))
    m = Image.new("L", (S, S), 0)
    dr = ImageDraw.Draw(m)
    dr.ellipse([cx - r, cy - r, cx + r, cy + r], fill=255)
    dr.polygon([(cx, apex), tr, tl], fill=255)
    return m


def wave_mask():
    m = Image.new("L", (S, S), 0)
    dr = ImageDraw.Draw(m)
    pts = [(x, scale(585) + scale(26) * math.sin((x / scale(1)) / 150 * math.tau)) for x in range(0, S + 1, 4)]
    dr.polygon(pts + [(S, S), (0, S)], fill=255)
    return m


img = background()
drop = drop_mask()
img.paste(Image.new("RGBA", (S, S), WHITE + (255,)), (0, 0), drop)
water = ImageChops.multiply(drop, wave_mask())
img.paste(Image.new("RGBA", (S, S), ORANGE + (255,)), (0, 0), water)
icon = img.resize((1024, 1024), Image.LANCZOS)

OUT.mkdir(parents=True, exist_ok=True)
icon.save(OUT / "icon.png")
icon.save(OUT / "icon.ico", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])

with tempfile.TemporaryDirectory() as tmp:
    iconset = Path(tmp) / "icon.iconset"
    iconset.mkdir()
    for base in (16, 32, 128, 256, 512):
        icon.resize((base, base), Image.LANCZOS).save(iconset / f"icon_{base}x{base}.png")
        icon.resize((base * 2, base * 2), Image.LANCZOS).save(iconset / f"icon_{base}x{base}@2x.png")
    subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(OUT / "icon.icns")], check=True)
print("Đã tạo", ", ".join(str(OUT / n) for n in ("icon.png", "icon.ico", "icon.icns")))
