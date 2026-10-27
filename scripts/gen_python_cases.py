"""Sinh ca đối chiếu bằng đúng logic của giaodien.py (bản Python cũ).

Chỉ sinh ca mà bản cũ tính đúng: cao trình trùng điểm bảng, khác nhau,
không phải dòng 1 (165 m, bản cũ bỏ qua), xả = 0.
"""
import csv
import random
import sys

import openpyxl

src, dst = sys.argv[1], sys.argv[2]
sheet = openpyxl.load_workbook(src)["Sheet1"]


def python_old(z0, z1, hours, h1, h2):
    # Sao nguyên văn vòng lặp của giaodien.py (min_row=2, ngưỡng 0.01, 17.3 gán cứng).
    v_total = h1 * 17.3 * 3600 + h2 * 17.3 * 3600
    v0 = v1 = 0
    for a, b in sheet.iter_rows(min_row=2, max_row=127, min_col=1, max_col=2, values_only=True):
        if abs(a - z0) < 0.01:
            v0 = b * 1000000
        elif abs(a - z1) < 0.01:
            v1 = b * 1000000
        if v0 > 0 and v1 > 0:
            break
    return ((v1 - v0) + v_total) / (hours * 3600)


levels = [round(a, 2) for a, in sheet.iter_rows(min_row=2, max_col=1, values_only=True)]
rng = random.Random(42)
with open(dst, "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["z_start", "z_end", "hours", "h1", "h2", "q_python"])
    for _ in range(50):
        z0, z1 = rng.sample(levels, 2)
        hours = rng.choice([1, 2, 4, 6, 8, 12, 24])
        h1, h2 = (rng.randint(0, hours * 2) / 2 for _ in range(2))
        h1, h2 = min(h1, hours), min(h2, hours)
        w.writerow([z0, z1, hours, h1, h2, repr(python_old(z0, z1, hours, h1, h2))])
