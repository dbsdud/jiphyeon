#!/usr/bin/env bash
# SessionStart hook: 마지막 감사/갭 리포트 이후 7일 경과 시 리마인더
set -euo pipefail

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

VAULT_ROOT="$(pwd)"
REPORTS_DIR="$VAULT_ROOT/_maintenance/reports"

[ ! -d "$REPORTS_DIR" ] && exit 0

# 가장 최근 audit 리포트 날짜 (없으면 빈 문자열)
LAST_AUDIT=$(ls -1 "$REPORTS_DIR"/audit-*.md 2>/dev/null | sort -r | head -1 || true)
LAST_GAP=$(ls -1 "$REPORTS_DIR"/gap-*.md 2>/dev/null | sort -r | head -1 || true)

TODAY=$(date +%s)
REMIND=""

if [ -n "$LAST_AUDIT" ]; then
  # 파일명에서 날짜 추출: audit-YYYY-MM-DD.md
  AUDIT_DATE=$(basename "$LAST_AUDIT" .md | sed 's/audit-//')
  AUDIT_TS=$(date -j -f "%Y-%m-%d" "$AUDIT_DATE" +%s 2>/dev/null || echo 0)
  DAYS_AGO=$(( (TODAY - AUDIT_TS) / 86400 ))
  if [ "$DAYS_AGO" -ge 7 ]; then
    REMIND="$REMIND  - vault-audit: ${DAYS_AGO}일 전 마지막 실행\n"
  fi
else
  REMIND="$REMIND  - vault-audit: 아직 실행한 적 없음\n"
fi

if [ -n "$LAST_GAP" ]; then
  GAP_DATE=$(basename "$LAST_GAP" .md | sed 's/gap-//')
  GAP_TS=$(date -j -f "%Y-%m-%d" "$GAP_DATE" +%s 2>/dev/null || echo 0)
  DAYS_AGO=$(( (TODAY - GAP_TS) / 86400 ))
  if [ "$DAYS_AGO" -ge 7 ]; then
    REMIND="$REMIND  - vault-gap: ${DAYS_AGO}일 전 마지막 실행\n"
  fi
else
  REMIND="$REMIND  - vault-gap: 아직 실행한 적 없음\n"
fi

if [ -n "$REMIND" ]; then
  echo "🔔 볼트 리뷰 권장:"
  echo -e "$REMIND"
  notify_append info "볼트 리뷰 권장 (vault-audit / vault-gap 확인)" "review-reminder"
fi

exit 0
