#!/usr/bin/env python3
"""Thăm dò API đăng nhập Vrain (bỏ đi sau khi viết xong crate vrain).

Đọc VRAIN_USER, VRAIN_PASS từ .env ở thư mục dự án (hoặc biến môi trường).
Chỉ in: mã trạng thái, TÊN khóa của phản hồi, TÊN header/cookie. Không in giá trị.
Bước 1 gửi body rỗng để tìm tiền tố đường dẫn đúng (không dùng mật khẩu).
Bước 2 chỉ thử các tên trường trên tiền tố đó, dừng ở lần thành công đầu tiên.
"""
import json
import os
import sys
import subprocess

BASE = "https://www.vrain.vn"
PREFIXES = ["/api", "/api/vrain", "/api/kttv", ""]
PATHS = ["/public/v2/login", "/public/v1/login"]
FIELD_SETS = [
    ("username", "password"),
    ("userName", "password"),
    ("email", "password"),
    ("account", "password"),
    ("user", "password"),
]


def load_env():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    path = os.path.join(root, ".env")
    if os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            line = line.strip()
            if line and not line.startswith("#") and "=" in line:
                k, v = line.split("=", 1)
                os.environ.setdefault(k.strip(), v.strip().strip('"').strip("'"))


def post(url, body, extra=None):
    """Gọi bằng curl hệ thống (có sẵn chứng chỉ); body qua stdin để không lộ mật khẩu."""
    cmd = [
        "curl", "-sS", "-m", "15", "-X", "POST", url,
        "-H", "Content-Type: application/json", "-H", "Accept: application/json",
        "-d", "@-", "-D", "-", "-o", "-", "-w", "\n__STATUS__%{http_code}",
    ]
    for k, v in (extra or {}).items():
        cmd += ["-H", f"{k}: {v}"]
    try:
        r = subprocess.run(cmd, input=json.dumps(body).encode(), capture_output=True, timeout=30)
    except Exception as e:
        print("  lỗi gọi curl:", e)
        return None, b"", {}
    if r.returncode != 0:
        print("  lỗi mạng:", r.stderr.decode(errors="replace").strip()[:200])
        return None, b"", {}
    out = r.stdout
    out, _, status = out.rpartition(b"\n__STATUS__")
    head, _, raw = out.rpartition(b"\r\n\r\n")
    headers = {}
    for line in head.decode(errors="replace").splitlines()[1:]:
        if ":" in line:
            k, v = line.split(":", 1)
            if k.lower() == "set-cookie":
                headers.setdefault("set-cookie", v.strip())
            else:
                headers[k.strip()] = v.strip()
    return int(status), raw, headers


def get(url):
    r = subprocess.run(
        ["curl", "-sS", "-m", "15", url, "-H", "Accept: application/json",
         "-D", "-", "-o", "-", "-w", "\n__STATUS__%{http_code}"],
        capture_output=True, timeout=30)
    if r.returncode != 0:
        print("  lỗi mạng:", r.stderr.decode(errors="replace").strip()[:200])
        return None, b"", {}
    out, _, status = r.stdout.rpartition(b"\n__STATUS__")
    head, _, raw = out.rpartition(b"\r\n\r\n")
    headers = {}
    for line in head.decode(errors="replace").splitlines()[1:]:
        if ":" in line:
            a, b = line.split(":", 1)
            headers[a.strip()] = b.strip()
    return int(status), raw, headers


def shape(o, depth=0):
    """Chỉ trả về tên khóa và kiểu, không trả giá trị."""
    if isinstance(o, dict):
        return {k: shape(v, depth + 1) if depth < 3 else "..." for k, v in o.items()}
    if isinstance(o, list):
        return [shape(o[0], depth + 1)] if o else []
    return type(o).__name__


def describe(status, raw, headers):
    print(f"  HTTP {status}")
    try:
        print("  khóa phản hồi:", json.dumps(shape(json.loads(raw)), ensure_ascii=False))
    except Exception:
        print(f"  phản hồi không phải JSON ({len(raw)} byte)")
    names = sorted({k.lower() for k in headers.keys()}) if headers else []
    print("  tên header:", ", ".join(names))
    cookies = [v.split("=", 1)[0] for k, v in headers.items() if k.lower() == "set-cookie"] if headers else []
    if cookies:
        print("  tên cookie đặt:", ", ".join(cookies))


STATIONS = {
    "841697470576": "Hương Nguyên",
    "84396786852": "A Đớt",
    "84846302387": "Tà Lương",
}


def curl_json(method, url, headers, jar, body=None):
    """curl có cookie jar; trả (status, đối tượng JSON hoặc None)."""
    cmd = ["curl", "-sS", "-m", "30", "-X", method, url, "-b", jar, "-c", jar,
           "-H", "Accept: application/json", "-w", "\n__STATUS__%{http_code}"]
    for k, v in headers.items():
        cmd += ["-H", f"{k}: {v}"]
    data = None
    if body is not None:
        cmd += ["-H", "Content-Type: application/json", "-d", "@-"]
        data = json.dumps(body).encode()
    r = subprocess.run(cmd, input=data, capture_output=True, timeout=60)
    if r.returncode != 0:
        print("  lỗi mạng:", r.stderr.decode(errors="replace").strip()[:200])
        return None, None
    out, _, status = r.stdout.rpartition(b"\n__STATUS__")
    try:
        return int(status), json.loads(out)
    except Exception:
        return int(status), None


def main():
    import datetime
    import tempfile

    load_env()
    user, pw = os.environ.get("VRAIN_USER"), os.environ.get("VRAIN_PASS")
    if not user or not pw:
        sys.exit("Thiếu VRAIN_USER / VRAIN_PASS trong .env")
    org = os.environ.get("VRAIN_ORG_UUID", "d50c9e31-74b5-4f44-b2c9-ba76c073340a")
    ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36"
    h = {"Origin": "https://www.vrain.vn", "Referer": "https://www.vrain.vn/landing",
         "User-Agent": ua, "x-org-uuid": org, "x-vrain-user-agent": ua}

    fd, jar = tempfile.mkstemp(prefix="vrain_jar_")
    os.close(fd)
    try:
        st, j = curl_json("POST", BASE + "/api/vrain/public/v1/login", h, jar, {"username": user, "password": pw})
        print("login:", st)
        if st != 200:
            return

        vn = datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=7)))
        today = vn.strftime("%Y-%m-%d")

        # 1) stats/summary
        st, j = curl_json("GET", BASE + "/api/vrain/private/v2/stats/summary", h, jar)
        print("\nstats/summary:", st)
        if j:
            if isinstance(j, dict):
                print("  gốc là object, khóa:", list(j.keys()))
                items = j.get("stats", [])
            else:
                print(f"  gốc là danh sách {len(j)} phần tử")
                items = j
            if items:
                print("  khóa mỗi phần tử:", sorted(items[0].keys()) if isinstance(items[0], dict) else type(items[0]).__name__)
            for x in items:
                if str(x.get("stationID")) in STATIONS:
                    print("  ", STATIONS[str(x["stationID"])], {k: x.get(k) for k in ("sumDepth", "from", "to", "level")})

        # 2) organizations/details, thử có và không có header x-org-uuid
        for label, hh in (("có x-org-uuid", h), ("không x-org-uuid", {k: v for k, v in h.items() if k != "x-org-uuid"})):
            for iv in ("_1h", "_10m"):
                url = f"{BASE}/api/kttv/private/v1/organizations/details?from={today}&to={today}&i={iv}"
                st, j = curl_json("GET", url, hh, jar)
                print(f"\ndetails {iv} [{label}]:", st)
                if not j or "stats" not in j:
                    if j:
                        print("  khóa:", list(j.keys()))
                    continue
                stats = j["stats"]
                tps = [x.get("timePoint") for x in stats]
                print(f"  {len(stats)} mốc; đầu: {tps[:3]}; cuối: {tps[-3:]}")
                print("  khóa mỗi mốc:", sorted(stats[0].keys()))
                for sid, name in STATIONS.items():
                    vals = []
                    for x in stats[-4:]:
                        for s_ in x.get("stations", []):
                            if str(s_.get("id")) == sid:
                                vals.append((x.get("timePoint"), s_.get("depth")))
                    print(f"  {name}: {vals}")
                break
            else:
                continue
            break

        # ---- J0: ba điều còn mở (kế hoạch Vrain, Task 0) ----
        ids = set(STATIONS)

        def mine(j):
            return [(x.get("timePoint"), s_.get("depth")) for x in j.get("stats", []) for s_ in x.get("stations", []) if str(s_.get("id")) in ids]

        print("\n== J0(a) nhiều ngày: from = hôm kia ==")
        two_days_ago = (vn - datetime.timedelta(days=2)).strftime("%Y-%m-%d")
        st, j = curl_json("GET", f"{BASE}/api/kttv/private/v1/organizations/details?from={two_days_ago}&to={today}&i=_1h", h, jar)
        print("  HTTP", st)
        if j and "stats" in j:
            tps = [x.get("timePoint") for x in j["stats"]]
            print(f"  {len(tps)} mốc; đầu: {tps[:3]}; cuối: {tps[-3:]}")
            days = sorted({t.split()[-1] for t in tps if t and " " in t})
            print("  các ngày xuất hiện:", days)

        print("\n== J0(b) không có header x-org-uuid ==")
        h2 = {k: v for k, v in h.items() if k != "x-org-uuid"}
        st, j = curl_json("GET", f"{BASE}/api/kttv/private/v1/organizations/details?from={today}&to={today}&i=_1h", h2, jar)
        print("  HTTP", st, "(200 = không bắt buộc)")

        print("\n== J0(c) mốc giờ hiện tại ==")
        print("  giờ VN lúc gọi:", vn.strftime("%H:%M %d/%m"))
        st, j = curl_json("GET", f"{BASE}/api/kttv/private/v1/organizations/details?from={today}&to={today}&i=_1h", h, jar)
        if j and "stats" in j:
            print("  4 mốc cuối của 3 trạm:", mine({"stats": j["stats"][-4:]}))
            st2, j2 = curl_json("GET", f"{BASE}/api/kttv/private/v1/organizations/details?from={today}&to={today}&i=_10m", h, jar)
            if j2 and "stats" in j2:
                tps = [x.get("timePoint") for x in j2["stats"]]
                print("  _10m: mốc cuối:", tps[-3:], "→ so với giờ VN để biết mốc giờ gần nhất có trọn không")
    finally:
        try:
            os.remove(jar)
        except OSError:
            pass


if __name__ == "__main__":
    main()
