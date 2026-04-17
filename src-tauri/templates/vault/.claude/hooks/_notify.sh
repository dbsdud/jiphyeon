#!/usr/bin/env bash
# notifications.jsonl append 헬퍼.
# 훅 스크립트가 source해서 `notify_append LEVEL MESSAGE [SOURCE]`로 사용한다.
#
# 사용 예:
#   . "$(dirname "${BASH_SOURCE[0]}")/_notify.sh"
#   notify_append warn "깨진 링크 3개" "check-broken-links"
#
# 안전성:
# - 파일 I/O/디렉토리 생성 실패가 훅 전체를 깨뜨리지 않도록 || true 방어
# - JSON escape는 backslash / quote / CR / LF / tab만 최소 처리

_notify_json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

# 볼트 루트 추정: CLAUDE_PROJECT_DIR → cwd에서 CLAUDE.md 상위 → cwd
_notify_find_root() {
  if [ -n "${CLAUDE_PROJECT_DIR:-}" ]; then
    printf '%s' "$CLAUDE_PROJECT_DIR"
    return 0
  fi
  local dir
  dir="$(pwd)"
  while [ "$dir" != "/" ] && [ ! -f "$dir/CLAUDE.md" ]; do
    dir="$(dirname "$dir")"
  done
  if [ -f "$dir/CLAUDE.md" ]; then
    printf '%s' "$dir"
  else
    printf '%s' "$(pwd)"
  fi
}

notify_append() {
  local level="${1:-info}"
  local message="${2:-}"
  local source="${3:-}"
  [ -z "$message" ] && return 0

  local root dir file ts
  root="$(_notify_find_root)"
  dir="$root/.claude/state"
  file="$dir/notifications.jsonl"
  ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

  mkdir -p "$dir" 2>/dev/null || return 0

  local esc_msg esc_src
  esc_msg="$(_notify_json_escape "$message")"

  if [ -n "$source" ]; then
    esc_src="$(_notify_json_escape "$source")"
    printf '{"level":"%s","message":"%s","source":"%s","ts":"%s"}\n' \
      "$level" "$esc_msg" "$esc_src" "$ts" >> "$file" 2>/dev/null || true
  else
    printf '{"level":"%s","message":"%s","ts":"%s"}\n' \
      "$level" "$esc_msg" "$ts" >> "$file" 2>/dev/null || true
  fi
}
