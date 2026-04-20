#!/usr/bin/env bash
# SessionStart hook: 볼트 건강 스냅샷 (경량)
# pipefail 일부러 제외 — grep 매치 0건(exit 1)이 파이프라인 전체를 죽이면 안 됨.
set -eu

# shellcheck source=./_notify.sh
. "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"

VAULT_ROOT="$(pwd)"
[ ! -f "$VAULT_ROOT/CLAUDE.md" ] && exit 0

# 전체 노트 수 (시스템 폴더 제외)
TOTAL=$(find "$VAULT_ROOT" -name '*.md' \
  -not -path '*/_templates/*' \
  -not -path '*/.claude/*' \
  -not -path '*/_maintenance/*' \
  -not -path '*/_moc/*' \
  -not -path '*/archive/*' \
  -not -name 'CLAUDE.md' \
  -not -name 'TASK.md' \
  -not -name 'README.md' \
  -type f 2>/dev/null | wc -l | tr -d ' ')

# 노트가 없으면 출력 생략
[ "$TOTAL" -eq 0 ] && exit 0

# status별 집계
SEEDLING=$(grep -rl '^status: seedling' "$VAULT_ROOT" --include='*.md' 2>/dev/null | grep -v '_templates\|\.claude\|_maintenance' | wc -l | tr -d ' ')
STALE=$(grep -rl '^status: stale' "$VAULT_ROOT" --include='*.md' 2>/dev/null | grep -v '_templates\|\.claude\|_maintenance' | wc -l | tr -d ' ')

# 깨진 링크 수 (빠르게)
ALL_LINKS=$(grep -rohE '\[\[[^]]+\]\]' "$VAULT_ROOT"/*.md "$VAULT_ROOT"/**/*.md 2>/dev/null | sed 's/\[\[//;s/\]\]//' | sort -u)
BROKEN=0
while IFS= read -r link; do
  [ -z "$link" ] && continue
  TARGET="$link"
  [[ "$TARGET" != *.md ]] && TARGET="$TARGET.md"
  if ! find "$VAULT_ROOT" -name "$(basename "$TARGET")" -type f 2>/dev/null | grep -q .; then
    BROKEN=$((BROKEN + 1))
  fi
done <<< "$ALL_LINKS"

# 최근 7일 생성 노트
RECENT=$(find "$VAULT_ROOT" -name '*.md' \
  -not -path '*/_templates/*' \
  -not -path '*/.claude/*' \
  -not -path '*/_maintenance/*' \
  -newer "$VAULT_ROOT" -mtime -7 \
  -type f 2>/dev/null | wc -l | tr -d ' ')

echo "📊 볼트 현황: 노트 ${TOTAL}개 | seedling ${SEEDLING} | stale ${STALE} | 깨진링크 ${BROKEN} | 최근7일 ${RECENT}개"
notify_append info "볼트 현황: 노트 ${TOTAL}개 · seedling ${SEEDLING} · stale ${STALE} · 깨진링크 ${BROKEN}" "vault-health-snapshot"

exit 0
