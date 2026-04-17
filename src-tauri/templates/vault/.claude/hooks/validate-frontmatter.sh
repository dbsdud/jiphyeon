#!/usr/bin/env bash
# PostToolUse hook: .md 파일의 frontmatter 유효성 검증
# 경고만 출력하고 차단하지 않음 (exit 0)

set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

# stdin에서 hook 데이터 읽기
HOOK_DATA=$(cat)

# tool_input에서 file_path 추출
FILE_PATH=$(echo "$HOOK_DATA" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)

# file_path가 없으면 종료
[ -z "$FILE_PATH" ] && exit 0

# .md 파일이 아니면 종료
[[ "$FILE_PATH" != *.md ]] && exit 0

# 시스템 폴더 내 파일이면 종료
case "$FILE_PATH" in
  */_templates/*|*/dashboard/*|*/.claude/*|*/_maintenance/*)
    exit 0
    ;;
esac

# 파일 존재 확인
[ ! -f "$FILE_PATH" ] && exit 0

# frontmatter 존재 확인 (첫 줄이 ---)
FIRST_LINE=$(head -1 "$FILE_PATH")
if [ "$FIRST_LINE" != "---" ]; then
  echo "⚠ frontmatter 없음: $FILE_PATH"
  notify_append warn "frontmatter 없음 — $(basename "$FILE_PATH")" "validate-frontmatter"
  exit 0
fi

# frontmatter 추출 (두 번째 --- 까지)
FRONTMATTER=$(awk 'BEGIN{c=0} /^---$/{c++; if(c==2) exit; next} c==1{print}' "$FILE_PATH")

# type 필드 확인
if ! echo "$FRONTMATTER" | grep -q '^type:'; then
  echo "⚠ frontmatter에 'type' 필드 누락: $FILE_PATH"
  notify_append warn "frontmatter type 누락 — $(basename "$FILE_PATH")" "validate-frontmatter"
fi

# created 필드 확인
if ! echo "$FRONTMATTER" | grep -q '^created:'; then
  echo "⚠ frontmatter에 'created' 필드 누락: $FILE_PATH"
  notify_append warn "frontmatter created 누락 — $(basename "$FILE_PATH")" "validate-frontmatter"
fi

exit 0
