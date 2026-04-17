#!/usr/bin/env bash
# PreToolUse hook: 노트 Write/Edit 전 frontmatter + 파일명 규칙 차단
# tool_input의 content/file_path를 파싱하여 파일 생성 전에 검증
set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

HOOK_DATA=$(cat)

TOOL_NAME=$(echo "$HOOK_DATA" | grep -o '"tool_name":"[^"]*"' | head -1 | cut -d'"' -f4)
FILE_PATH=$(echo "$HOOK_DATA" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.md ]] && exit 0

# 시스템 폴더 제외
case "$FILE_PATH" in
  */_templates/*|*/dashboard/*|*/.claude/*|*/_maintenance/*|*/_moc/*)
    exit 0
    ;;
esac

# 볼트 루트의 시스템 .md 파일 제외 (CLAUDE.md, TASK.md, README.md 등)
VAULT_ROOT="$FILE_PATH"
while [ "$VAULT_ROOT" != "/" ]; do
  VAULT_ROOT=$(dirname "$VAULT_ROOT")
  [ -f "$VAULT_ROOT/CLAUDE.md" ] && break
done
if [ "$VAULT_ROOT" != "/" ] && [ "$(dirname "$FILE_PATH")" = "$VAULT_ROOT" ]; then
  exit 0
fi

# --- Write: content에서 frontmatter 검증 ---
if [ "$TOOL_NAME" = "Write" ]; then
  CONTENT=$(echo "$HOOK_DATA" | python3 -c "
import sys, json
data = json.load(sys.stdin)
print(data.get('tool_input', {}).get('content', ''))
" 2>/dev/null) || exit 0

  # frontmatter 시작 확인
  FIRST_LINE=$(echo "$CONTENT" | head -1)
  if [ "$FIRST_LINE" != "---" ]; then
    echo "BLOCK: frontmatter 없음 — 노트는 반드시 YAML frontmatter로 시작해야 합니다"
    notify_append error "차단: frontmatter 누락 — $(basename "$FILE_PATH")" "pre-validate-note"
    exit 2
  fi

  # frontmatter 추출
  FM=$(echo "$CONTENT" | awk 'BEGIN{c=0} /^---$/{c++; if(c==2) exit; next} c==1{print}')

  if ! echo "$FM" | grep -q '^type:'; then
    echo "BLOCK: frontmatter에 'type' 필드 누락"
    notify_append error "차단: type 필드 누락 — $(basename "$FILE_PATH")" "pre-validate-note"
    exit 2
  fi
  if ! echo "$FM" | grep -q '^created:'; then
    echo "BLOCK: frontmatter에 'created' 필드 누락"
    notify_append error "차단: created 필드 누락 — $(basename "$FILE_PATH")" "pre-validate-note"
    exit 2
  fi
fi

# --- Write: 파일명 규칙 검증 ---
if [ "$TOOL_NAME" = "Write" ]; then
  FILENAME=$(basename "$FILE_PATH" .md)

  # kebab-case 검증
  if echo "$FILENAME" | grep -qE '[A-Z]'; then
    echo "BLOCK: 파일명에 대문자 — 소문자 kebab-case를 사용하세요 ($FILENAME.md)"
    notify_append error "차단: 파일명 대문자 — $FILENAME.md" "pre-validate-note"
    exit 2
  fi
  if echo "$FILENAME" | grep -qE '_'; then
    echo "BLOCK: 파일명에 언더스코어 — 하이픈(-)을 사용하세요 ($FILENAME.md)"
    notify_append error "차단: 파일명 언더스코어 — $FILENAME.md" "pre-validate-note"
    exit 2
  fi
  if echo "$FILENAME" | grep -qE ' '; then
    echo "BLOCK: 파일명에 공백 — 하이픈(-)을 사용하세요 ($FILENAME.md)"
    notify_append error "차단: 파일명 공백 — $FILENAME.md" "pre-validate-note"
    exit 2
  fi

  # 날짜 접두사 검증 (content에서 type 추출)
  if [ -n "${FM:-}" ]; then
    TYPE=$(echo "$FM" | grep '^type:' | sed 's/type: *//')
    case "$TYPE" in
      til|meeting|decision|clipping)
        if ! echo "$FILENAME" | grep -qE '^[0-9]{4}-[0-9]{2}-[0-9]{2}-'; then
          echo "BLOCK: $TYPE 타입은 날짜 접두사 필요 — YYYY-MM-DD-제목 ($FILENAME.md)"
          notify_append error "차단: $TYPE 타입 날짜 접두사 누락 — $FILENAME.md" "pre-validate-note"
          exit 2
        fi
        ;;
    esac
  fi
fi

exit 0
