"""Chuyển tinhtoan.xlsx (cột A cao trình, B dung tích triệu m3) sang CSV tăng dần."""
import csv
import sys

import openpyxl

src, dst = sys.argv[1], sys.argv[2]
ws = openpyxl.load_workbook(src)["Sheet1"]
rows = []
for a, b, *_ in ws.iter_rows(min_row=1, values_only=True):
    rows.append((round(float(a), 2), float(b)))
rows.sort()
for (z0, v0), (z1, v1) in zip(rows, rows[1:]):
    assert z1 > z0, f"cao trình không tăng: {z0} -> {z1}"
    assert v1 > v0, f"dung tích không tăng: {z0} -> {z1}"
with open(dst, "w", newline="") as f:
    w = csv.writer(f)
    w.writerow(["cao_trinh_m", "dung_tich_trieu_m3"])
    for z, v in rows:
        w.writerow([f"{z:.2f}", repr(v)])
print(len(rows), "dòng;", rows[0][0], "->", rows[-1][0])
