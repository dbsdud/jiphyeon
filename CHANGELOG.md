# Changelog

집현(Jiphyeon)의 모든 주요 변경 사항을 기록합니다. 형식은 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/)를, 버전은 [SemVer](https://semver.org/)를 따릅니다.

## [Unreleased]

v1.0.0 MVP 공개 배포 준비 중.

- LICENSE(MIT), CHANGELOG, 설치 가이드 정비
- 운영 빌드 및 배포 절차 문서화

## [0.9.0] - 2026-04-21

Epic 4 — Explore 개선 + 온보딩 샘플 + README 정비.

### Added
- 시작용 샘플 노트 4개가 scaffold에 포함 (dev/TIL, readings/Reading, decisions/ADR, ideas/Idea) — wikilink 상호 참조로 대시보드 그래프/클러스터가 첫 진입부터 활성화
- 대시보드 "시작하기" 카드 — 노트 1~6개 상태에서만 노출, `cd <볼트> && claude` → `/vault-new` → `/vault-link` 3단계 안내
- Explore `FolderTree`의 접힘 상태 `localStorage` persist
- `_` prefix 시스템 폴더 시각적 구분 (dim)
- README에 집현전 연혁 섹션 (세종대왕 맥북 던짐 이스터에그)

### Changed
- Explore `.` 루트 노드가 "📓 볼트 루트" 라벨로 표시
- 접기/펼치기 chevron을 SVG 14px로 교체, 행 전체 클릭 = 선택 + 자식 토글
- `_moc/Home.md`, `_moc/Topics.md` 확장 — 샘플 노트 목록, 시작하기 안내, 스킬 요약표

## [0.8.0] - 2026-04-21

Epic 3 — Transcribe: 녹음 캡처 + 볼트 스킬 연동.

### Added
- `/transcribe` 페이지 + 사이드바 진입점 활성화
- MediaRecorder 기반 녹음 UI — 포맷 플랫폼 폴백 (`audio/mp4` → `audio/webm` → `audio/ogg`, macOS `.m4a` 우선)
- Canvas + Web Audio `AnalyserNode`로 실시간 파형 표시
- 60분 자동 롤링 (시간 도달 시 파일 자동 분할 + 시퀀스 증가)
- 녹음 파일 전체 목록 — 전사 상태 뱃지(`전사 완료` / `미전사`), 크기/수정일, 개별 삭제
- 오디오 파일 ⌘V 붙여넣기 지원 (Finder 복사 → /transcribe 붙여넣기)
- `vault-transcribe` 스킬 scaffold 템플릿 — `_sources/recordings/` 스캔 → `openai-whisper` CLI → `_sources/transcripts/<stem>.md` (whisper `base` 기본값, 사용자 변경 가능)
- SessionStart 훅 `check-pending-recordings.sh` — 미전사 녹음 감지 시 Claude 컨텍스트에 처리 지시 주입
- Rust 커맨드: `save_recording`, `delete_recording`, `list_recordings` (16 테스트)
- macOS 마이크 권한: `macOSPrivateApi: true`, `Info.plist` `NSMicrophoneUsageDescription`

### Fixed
- `vault-health-snapshot.sh`의 `set -euo pipefail` 조합이 stale 노트 0개 볼트에서 grep exit 1로 SessionStart 훅 전체를 죽이던 이슈 (pipefail 제거)

## [0.7.0] - 2026-04-20

Epic 2 — 대시보드 강화 + 그래프 인텔리전스.

### Added
- **God Node 카드** — 백링크 수 상위 허브 노트 상위 5개 노출, self-reference 제외, broken link 제외, 정렬 desc → path asc
- **클러스터 요약 카드** — 무방향 BFS 기반 연결 컴포넌트, 크기 1(고립) 분리 집계, 대표 노트 = God Node 규칙 재사용
- **그래프 검색/필터** — title 검색 + type/tag 필터 AND, 앵커 + 1-hop 이웃 정상 표시, 나머지 `opacity 0.15` dim, 200ms transition
- `GraphNode.tags` 필드 추가 (프론트 필터링용)
- `indexer::build_link_graph`, `compute_top_god_nodes`, `compute_clusters` (28 테스트)

## [0.6.0] - 2026-04-18

Epic 1 — UI 전면 개선.

### Added
- 디자인 시스템 토큰 (색상/타이포그래피/간격/밀도)
- 컴팩트/기본 밀도 토글 (설정 persist + 전역 적용)
- GitHub 스타일 마크다운 + GFM + 코드 하이라이팅 + KaTeX + Mermaid
- 사이드바 재구성 (탐색/작업/설정 3그룹)
- 라이트/다크/System 테마 스위칭 (flash 방지 인라인 스크립트)
- 사이드바 collapse/expand (`Cmd/Ctrl+B` 단축키, `AppConfig.sidebar_collapsed` persist)

### Changed
- 마크다운 본문 스타일을 `@tailwindcss/typography`로 전환 (테마 변수 연동)
- `bg-accent text-fg` 패턴 금지, `--color-accent-fg` 토큰 신설

## [0.5.0] - 이전

MVP 이전 기반 — 에디터 연동 + Claude 도구 뷰 + 멀티 볼트. 이 CHANGELOG는 0.6.0부터 추적.

[Unreleased]: https://github.com/dbsdud/jiphyeon/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/dbsdud/jiphyeon/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/dbsdud/jiphyeon/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/dbsdud/jiphyeon/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/dbsdud/jiphyeon/releases/tag/v0.6.0
