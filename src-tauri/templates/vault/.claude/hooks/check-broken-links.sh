#!/usr/bin/env bash
# PostToolUse hook: [[wikilink]] 대상 파일 존재 여부 실시간 확인
set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

HOOK_DATA=$(cat)
FILE_PATH=$(echo "$HOOK_DATA" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.md ]] && exit 0
[ ! -f "$FILE_PATH" ] && exit 0

# 시스템 폴더 제외
case "$FILE_PATH" in
  */_templates/*|*/.claude/*|*/_maintenance/*) exit 0 ;;
esac

# 볼트 루트 추정 (CLAUDE.md가 있는 디렉토리)
VAULT_ROOT=$(dirname "$FILE_PATH")
while [ "$VAULT_ROOT" != "/" ] && [ ! -f "$VAULT_ROOT/CLAUDE.md" ]; do
  VAULT_ROOT=$(dirname "$VAULT_ROOT")
done
[ "$VAULT_ROOT" = "/" ] && exit 0

# [[wikilink]] 추출 (없으면 빈 문자열)
LINKS=$(grep -oE '\[\[[^]]+\]\]' "$FILE_PATH" 2>/dev/null | sed 's/\[\[//;s/\]\]//' | sort -u || true)
[ -z "$LINKS" ] && exit 0

BROKEN=""
while IFS= read -r link; do
  # 파일명으로 검색 (확장자 없으면 .md 추가)
  TARGET="$link"
  [[ "$TARGET" != *.md ]] && TARGET="$TARGET.md"

  if ! find "$VAULT_ROOT" -name "$(basename "$TARGET")" -type f 2>/dev/null | grep -q .; then
    BROKEN="$BROKEN  - [[$link]]\n"
  fi
done <<< "$LINKS"

if [ -n "$BROKEN" ]; then
  echo "⚠ 깨진 링크 발견 ($FILE_PATH):"
  echo -e "$BROKEN"
  BROKEN_COUNT=$(printf '%b' "$BROKEN" | grep -c '\[\[' || true)
  notify_append warn "깨진 링크 ${BROKEN_COUNT}개 — $(basename "$FILE_PATH")" "check-broken-links"
fi

exit 0
