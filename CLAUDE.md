# Jiphyeon

Tauri v2 기반 마크다운 볼트 대시보드 앱. 볼트 통계 시각화, 노트 탐색, 링크 그래프, 웹 클리핑을 제공한다.

## 기술 스택

- **Backend**: Rust (Tauri v2)
- **Frontend**: Svelte 5 (runes) + SvelteKit (adapter-static, SPA) + Tailwind CSS 4
- **차트**: layercake
- **그래프**: d3-force
- **상태관리**: Svelte $state runes (외부 라이브러리 없음)

## 주요 Rust 크레이트

| 크레이트 | 버전 | 용도 |
|---------|------|------|
| tauri | 2 | 앱 프레임워크 |
| serde_yaml_ng | 0.9 | YAML frontmatter 파싱 |
| pulldown-cmark | 0.13 | 마크다운 → HTML |
| notify | 8 | 파일 시스템 감시 |
| thiserror | 2 | 에러 타입 |

## 프로젝트 구조

```
src/              Svelte 프론트엔드
src-tauri/src/    Rust 백엔드
  models.rs       데이터 모델
  config.rs       설정
  error.rs        에러 타입
  vault/
    parser.rs     볼트 파서
    indexer.rs    볼트 인덱서
  commands/       Tauri IPC 커맨드
  watcher/        파일 감시
docs/             PRD, 아키텍처 문서
  specs/           SDD 명세
```

## 볼트 연결

MVP에서는 단일 로컬 볼트를 지원한다 (`config.rs`의 `vault_path`).

향후 멀티 워크스페이스 확장 예정:
- 개인 볼트 (로컬 경로 또는 GitHub 개인 레포)
- 팀 볼트 (GitHub 팀 레포)
- 워크스페이스 셀렉터로 전환

## 개발 명령

```bash
# Rust 테스트
cargo test --manifest-path src-tauri/Cargo.toml --lib

# 개발 서버
npm run tauri dev

# 빌드
npm run tauri build
```

## Claude Code 작업 규칙

- SDD → TDD 사이클 준수
- 한국어로 작성 (기술 용어는 영어 허용)
