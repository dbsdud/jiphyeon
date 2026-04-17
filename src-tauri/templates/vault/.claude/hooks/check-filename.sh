#!/usr/bin/env bash
# PostToolUse hook: 파일명 규칙 검증 (CLAUDE.md 기준)
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

# 시스템 폴더 제외
case "$FILE_PATH" in
  */_templates/*|*/.claude/*|*/_maintenance/*|*/_moc/*) exit 0 ;;
esac

FILENAME=$(basename "$FILE_PATH" .md)
WARNINGS=""

# MOC 파일이 아닌 경우 kebab-case 검증
if [[ "$FILE_PATH" != */_moc/* ]]; then
  # 대문자 검사
  if echo "$FILENAME" | grep -qE '[A-Z]'; then
    WARNINGS="$WARNINGS  - 파일명에 대문자 사용: 소문자 kebab-case 권장\n"
  fi
  # 언더스코어 검사
  if echo "$FILENAME" | grep -qE '_'; then
    WARNINGS="$WARNINGS  - 파일명에 언더스코어 사용: 하이픈(-) 권장\n"
  fi
  # 공백 검사
  if echo "$FILENAME" | grep -qE ' '; then
    WARNINGS="$WARNINGS  - 파일명에 공백 포함: 하이픈(-) 권장\n"
  fi
fi

# 날짜 접두사 필요 타입 확인 (til, meeting, decision, clipping)
if [ -f "$FILE_PATH" ]; then
  FRONTMATTER=$(awk 'BEGIN{c=0} /^---$/{c++; if(c==2) exit; next} c==1{print}' "$FILE_PATH")
  TYPE=$(echo "$FRONTMATTER" | grep '^type:' | sed 's/type: *//')

  case "$TYPE" in
    til|meeting|decision|clipping)
      if ! echo "$FILENAME" | grep -qE '^[0-9]{4}-[0-9]{2}-[0-9]{2}-'; then
        WARNINGS="$WARNINGS  - $TYPE 타입은 날짜 접두사 필요: YYYY-MM-DD-제목\n"
      fi
      ;;
  esac
fi

if [ -n "$WARNINGS" ]; then
  echo "⚠ 파일명 규칙 ($FILENAME.md):"
  echo -e "$WARNINGS"
  notify_append warn "파일명 규칙 위반 — $FILENAME.md" "check-filename"
fi

exit 0
