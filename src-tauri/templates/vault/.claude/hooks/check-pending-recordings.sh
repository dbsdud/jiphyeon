#!/usr/bin/env bash
# SessionStart 훅: _sources/recordings/의 미전사 녹음 개수를 stderr로 알림.
# transcript가 없는 오디오 파일이 있으면 "/vault-transcribe"로 처리하라는 안내 한 줄 출력.

set -u

root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
rec_dir="$root/_sources/recordings"
tx_dir="$root/_sources/transcripts"

[[ ! -d "$rec_dir" ]] && exit 0

pending=0
while IFS= read -r -d '' f; do
  stem="$(basename "${f%.*}")"
  if [[ ! -f "$tx_dir/$stem.md" ]]; then
    pending=$((pending + 1))
  fi
done < <(find "$rec_dir" -maxdepth 1 -type f \( \
  -name "*.m4a" -o -name "*.webm" -o -name "*.ogg" -o -name "*.wav" -o -name "*.mp3" \
\) -print0 2>/dev/null)

if [[ "$pending" -gt 0 ]]; then
  # SessionStart 훅의 stdout은 Claude 컨텍스트에 주입된다.
  # Claude가 첫 응답에서 이 사실을 명시적으로 언급하도록 지시 형태로 작성.
  cat <<EOF
[집현 SessionStart] _sources/recordings/에 미전사 녹음 ${pending}개가 감지되었습니다.
첫 응답 시 사용자에게 이 사실을 알리고, /vault-transcribe 스킬로 전사할지 질문하세요.
대상 파일은 _sources/recordings/ 하위의 오디오 파일 중 _sources/transcripts/<stem>.md가 없는 것들입니다.
EOF
fi

exit 0
