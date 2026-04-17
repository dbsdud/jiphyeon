#!/usr/bin/env bash
# Stop hook: 세션 중 생성/수정된 노트를 활동 로그에 기록
set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

VAULT_ROOT="$(pwd)"
[ ! -f "$VAULT_ROOT/CLAUDE.md" ] && exit 0

LOG_FILE="$VAULT_ROOT/_maintenance/session-log.md"
TODAY=$(date +%Y-%m-%d)
NOW=$(date +%H:%M)

# git으로 이번 세션에서 변경된 .md 파일 확인
CHANGED=$(git diff --name-only HEAD 2>/dev/null | grep '\.md$' | grep -v '_templates\|\.claude\|_maintenance' || true)
UNTRACKED=$(git ls-files --others --exclude-standard 2>/dev/null | grep '\.md$' | grep -v '_templates\|\.claude\|_maintenance' || true)

# 변경 사항이 없으면 종료
[ -z "$CHANGED" ] && [ -z "$UNTRACKED" ] && exit 0

# 로그 파일이 없으면 헤더 생성
if [ ! -f "$LOG_FILE" ]; then
  mkdir -p "$(dirname "$LOG_FILE")"
  echo "# 세션 활동 로그" > "$LOG_FILE"
  echo "" >> "$LOG_FILE"
fi

# 오늘 날짜 섹션이 없으면 추가
if ! grep -q "^## $TODAY" "$LOG_FILE" 2>/dev/null; then
  echo "" >> "$LOG_FILE"
  echo "## $TODAY" >> "$LOG_FILE"
fi

# 엔트리 추가
echo "" >> "$LOG_FILE"
echo "### $NOW" >> "$LOG_FILE"

if [ -n "$UNTRACKED" ]; then
  echo "생성:" >> "$LOG_FILE"
  echo "$UNTRACKED" | while read -r f; do echo "- $f" >> "$LOG_FILE"; done
fi

if [ -n "$CHANGED" ]; then
  echo "수정:" >> "$LOG_FILE"
  echo "$CHANGED" | while read -r f; do echo "- $f" >> "$LOG_FILE"; done
fi

CREATED_COUNT=$([ -n "$UNTRACKED" ] && printf '%s\n' "$UNTRACKED" | grep -c . || echo 0)
MODIFIED_COUNT=$([ -n "$CHANGED" ] && printf '%s\n' "$CHANGED" | grep -c . || echo 0)
notify_append success "세션 활동 기록: 생성 ${CREATED_COUNT}개 / 수정 ${MODIFIED_COUNT}개" "session-activity-log"

exit 0
