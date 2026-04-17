#!/usr/bin/env bash
# PostToolUse hook: 새로 추가된 태그가 기존 유사 태그와 중복되는지 경고
set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

HOOK_DATA=$(cat)
FILE_PATH=$(echo "$HOOK_DATA" | grep -o '"file_path":"[^"]*"' | head -1 | cut -d'"' -f4)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.md ]] && exit 0
[ ! -f "$FILE_PATH" ] && exit 0

case "$FILE_PATH" in
  */_templates/*|*/.claude/*|*/_maintenance/*) exit 0 ;;
esac

# 볼트 루트 추정
VAULT_ROOT=$(dirname "$FILE_PATH")
while [ "$VAULT_ROOT" != "/" ] && [ ! -f "$VAULT_ROOT/CLAUDE.md" ]; do
  VAULT_ROOT=$(dirname "$VAULT_ROOT")
done
[ "$VAULT_ROOT" = "/" ] && exit 0

# 현재 파일의 태그 추출
CURRENT_TAGS=$(awk '
  BEGIN{in_fm=0; in_tags=0}
  /^---$/{in_fm++; if(in_fm==2) exit; next}
  in_fm==1 && /^tags:/{in_tags=1; next}
  in_fm==1 && in_tags && /^  - /{gsub(/^  - /,""); print; next}
  in_fm==1 && in_tags && !/^  /{in_tags=0}
' "$FILE_PATH")

[ -z "$CURRENT_TAGS" ] && exit 0

# 기존 전체 태그 수집 (현재 파일 제외)
EXISTING_TAGS=$(find "$VAULT_ROOT" -name '*.md' \
  -not -path '*/_templates/*' \
  -not -path '*/.claude/*' \
  -not -path '*/_maintenance/*' \
  -not -path "$FILE_PATH" \
  -exec awk '
    BEGIN{in_fm=0; in_tags=0}
    /^---$/{in_fm++; if(in_fm==2) exit; next}
    in_fm==1 && /^tags:/{in_tags=1; next}
    in_fm==1 && in_tags && /^  - /{gsub(/^  - /,""); print; next}
    in_fm==1 && in_tags && !/^  /{in_tags=0}
  ' {} \; 2>/dev/null | sort -u)

[ -z "$EXISTING_TAGS" ] && exit 0

WARNINGS=""
while IFS= read -r tag; do
  [ -z "$tag" ] && continue
  TAG_BASE=$(basename "$tag" | tr '/' '\n' | tail -1)

  while IFS= read -r existing; do
    [ -z "$existing" ] && continue
    [ "$tag" = "$existing" ] && continue

    # 계층 없는 태그가 계층 태그의 하위와 동일한 경우
    # 예: "backend" vs "domain/backend"
    EXISTING_BASE=$(echo "$existing" | tr '/' '\n' | tail -1)
    if [ "$TAG_BASE" = "$EXISTING_BASE" ] && [ "$tag" != "$existing" ]; then
      WARNINGS="$WARNINGS  - '$tag' ↔ '$existing' (유사 태그 존재)\n"
    fi
  done <<< "$EXISTING_TAGS"
done <<< "$CURRENT_TAGS"

if [ -n "$WARNINGS" ]; then
  echo "⚠ 태그 중복 의심:"
  echo -e "$WARNINGS"
  notify_append warn "유사 태그 존재 — $(basename "$FILE_PATH")" "check-tag-duplication"
fi

exit 0
