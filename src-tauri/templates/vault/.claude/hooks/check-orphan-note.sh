#!/usr/bin/env bash
# PostToolUse hook: 새 노트에 [[wikilink]]가 하나도 없으면 경고
set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

HOOK_DATA=$(cat)
TOOL_NAME=$(echo "$HOOK_DATA" | grep -o '"tool_name":"[^"]*"' | head -1 | cut -d'"' -f4)
FILE_PATH=$(echo "$HOOK_DATA" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)

# Write(새 파일 생성)일 때만 검사
[ "$TOOL_NAME" != "Write" ] && exit 0
[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.md ]] && exit 0
[ ! -f "$FILE_PATH" ] && exit 0

case "$FILE_PATH" in
  */_templates/*|*/.claude/*|*/_maintenance/*|*/_moc/*) exit 0 ;;
esac

if ! grep -q '\[\[.*\]\]' "$FILE_PATH" 2>/dev/null; then
  echo "⚠ 링크 없는 새 노트: $FILE_PATH"
  echo "  → 최소 1개 기존 노트와 [[wikilink]] 연결 필요"
  notify_append warn "링크 없이 저장됨 — $(basename "$FILE_PATH")" "check-orphan-note"
fi

exit 0
