#!/bin/bash
cmd=$(python3 -c 'import sys,json;print(json.load(sys.stdin).get("tool_input",{}).get("command",""))')
if echo "$cmd" | grep -Eq 'rm -rf (/|~|\*)|git push.*(--force| -f)|git reset --hard|chmod -R 777|mkfs|dd if=|cat .*\.env'; then
  echo "Bị chặn: lệnh nguy hiểm. Hãy hỏi người dùng trước." >&2
  exit 2
fi
exit 0
