---
name: vault-transcribe
description: _sources/recordings/의 녹음을 전사·요약해 transcript 노트로 저장한다
---

# 녹음 전사

볼트의 `_sources/recordings/`에 있는 오디오를 `openai-whisper` CLI로 전사하고, 요약·관련 노트 연결까지 덧붙여 `_sources/transcripts/`에 노트로 저장한다.

"/vault-transcribe", "전사", "녹음 정리", "녹음 전사" 등의 키워드에 반응.

## 전제

- `openai-whisper`가 로컬에 설치되어 있어야 함: `pip install -U openai-whisper`
- 모델 다운로드 공간 (base ≈ 145MB, 첫 실행 시 자동 다운로드)
- 대상 포맷: `.m4a`, `.webm`, `.ogg`, `.wav`, `.mp3`

## 설정 상수 (필요하면 이 파일을 수정)

- `MODEL`: `base` — tiny/base/small/medium/large 중 선택. 속도와 정확도 사이 균형
- `LANGUAGE`: `ko` — 자동 감지하려면 빈 문자열
- `RECORDING_DIR`: `_sources/recordings`
- `TRANSCRIPT_DIR`: `_sources/transcripts`

## 절차

1. **미전사 파일 수집**
   - `_sources/recordings/`의 오디오 파일 나열
   - 각 파일 `<stem>` 기준 `_sources/transcripts/<stem>.md`가 없으면 대상
2. **사용자 확인**: 대상 목록 제시 → 전체/선택/취소
3. **각 파일 전사 (순차)**
   - 명령: `whisper "<path>" --model base --language ko --output_format txt --output_dir /tmp/vault-transcribe`
   - 실패하면 skip하고 에러를 기록한 뒤 다음으로
4. **요약·메타 생성**
   - 전사 원문 3~5문장 요약
   - 주요 토픽 키워드 3~5개 추출
   - 기존 볼트 노트에서 관련 내용 검색 → wikilink 후보
5. **transcript 노트 작성**: 경로 `_sources/transcripts/<stem>.md`
   - frontmatter:
     ```yaml
     type: transcript
     created: <오늘>
     source: _sources/recordings/<원본파일>
     model: base
     tags:
       - transcript
       - <추출 키워드>
     status: seedling
     ```
   - 본문 구성:
     - `## 요약`
     - `## 주요 토픽` (불릿)
     - `## 관련 노트` (wikilink 리스트)
     - `## 전사 원문` (whisper 출력 그대로)
6. **정리 보고**: 생성된 노트 경로 나열 + 후속 제안 (`vault-link`, `vault-mature` 등)

## 규칙

- 원본 오디오 파일은 **삭제하지 않는다** (원천 보존)
- transcript 노트의 stem은 원본 파일 확장자를 제거한 이름 그대로
- whisper 임시 출력(`/tmp/vault-transcribe`)은 작업 후 정리해도 되지만 필수는 아님
- 여러 파일을 묶어 전사할 때도 각 파일마다 별도 transcript 노트를 만든다

## 관련

- 녹음 캡처는 집현 앱의 `/transcribe` 페이지에서 수행
- SessionStart 훅 `check-pending-recordings.sh`가 세션 시작 시 미전사 녹음 개수를 알린다
- 연결·요약을 더 다듬으려면 `vault-link`, `vault-mature`를 이어서 호출
