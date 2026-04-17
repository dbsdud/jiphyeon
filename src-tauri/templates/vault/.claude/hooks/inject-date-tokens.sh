#!/usr/bin/env bash
# UserPromptSubmit hook: @today, @yesterday 등 날짜 토큰을 실제 날짜로 치환
set -euo pipefail

HOOK_DATA=$(cat)

# prompt 내용 추출
PROMPT=$(echo "$HOOK_DATA" | grep -o '"prompt":"[^"]*"' | head -1 | cut -d'"' -f4 2>/dev/null || echo "")

[ -z "$PROMPT" ] && exit 0

# 날짜 토큰이 없으면 종료
echo "$PROMPT" | grep -qE '@(today|yesterday|tomorrow|last-week)' || exit 0

TODAY=$(date +%Y-%m-%d)
YESTERDAY=$(date -v-1d +%Y-%m-%d 2>/dev/null || date -d 'yesterday' +%Y-%m-%d)
TOMORROW=$(date -v+1d +%Y-%m-%d 2>/dev/null || date -d 'tomorrow' +%Y-%m-%d)
LAST_WEEK=$(date -v-7d +%Y-%m-%d 2>/dev/null || date -d '7 days ago' +%Y-%m-%d)

OUTPUT=""
[ "$(echo "$PROMPT" | grep -c '@today')" -gt 0 ] && OUTPUT="$OUTPUT @today → $TODAY"
[ "$(echo "$PROMPT" | grep -c '@yesterday')" -gt 0 ] && OUTPUT="$OUTPUT | @yesterday → $YESTERDAY"
[ "$(echo "$PROMPT" | grep -c '@tomorrow')" -gt 0 ] && OUTPUT="$OUTPUT | @tomorrow → $TOMORROW"
[ "$(echo "$PROMPT" | grep -c '@last-week')" -gt 0 ] && OUTPUT="$OUTPUT | @last-week → $LAST_WEEK"

if [ -n "$OUTPUT" ]; then
  echo "📅${OUTPUT}"
fi

exit 0
