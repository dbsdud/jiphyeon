#!/usr/bin/env bash
# PreToolUse hook: _templates/ 편집 차단, archive/ 편집 경고
set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

HOOK_DATA=$(cat)
FILE_PATH=$(echo "$HOOK_DATA" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.md ]] && exit 0

# _templates/ 편집 차단
case "$FILE_PATH" in
  */_templates/*)
    echo "BLOCK: _templates/ 파일은 직접 편집할 수 없습니다. 템플릿 수정이 필요하면 사용자에게 확인하세요."
    notify_append error "차단: _templates/ 편집 시도 — $(basename "$FILE_PATH")" "protect-system-files"
    exit 2
    ;;
esac

# archive/ 편집 경고
case "$FILE_PATH" in
  */archive/*)
    echo "⚠ 아카이브된 노트입니다: $FILE_PATH"
    echo "  → 복원(/vault-archive) 후 편집을 권장합니다"
    notify_append warn "아카이브 노트 편집 — $(basename "$FILE_PATH")" "protect-system-files"
    exit 0
    ;;
esac

exit 0
