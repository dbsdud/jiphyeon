# Spec: rescaffold (v0.6 볼트 업데이트 — 재스캐폴드)

## 전제

- **배경**: 앱 릴리즈마다 `src-tauri/templates/vault/.claude/` 하위의 훅/스킬/settings가 업데이트된다. 기존 볼트는 이를 자동 반영할 방법이 없어, 사용자가 수동으로 파일을 옮기거나 볼트를 재생성해야 했다.
- **원칙**: 사용자 자산은 보존, 시스템 자산만 업데이트 가능.
  - **사용자 자산**: 루트 파일(`CLAUDE.md`, `.gitignore`, `.gitattributes`), `_moc/*`, `_templates/*`, 노트 전체
  - **시스템 자산**: `.claude/` 하위 (훅/스킬/settings.json)
- **두 모드**:
  - `add-missing` (기본, 안전): 누락된 파일만 채운다. 기존 파일은 절대 건드리지 않는다. (현행 `scaffold_vault` 동작과 동일)
  - `force-claude` (명시적 선택): 위 + `.claude/` 하위 **템플릿과 내용이 다른 파일을 덮어쓴다**. 덮어쓰는 대상은 `modified_by_user` 리스트로 보고해 UI가 경고한다.
- **Dry-run 지원**: 동일 함수에 `dry_run: true` → 파일 변경 없이 리포트만 생성. UI는 preview → confirm → apply 흐름을 구성한다.
- `force-claude`의 덮어쓰기 대상은 **템플릿에 존재하는 파일**로 한정한다. 사용자가 추가한 `.claude/skills/custom/*` 등은 건드리지 않는다.

## Public Interface

```rust
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RescaffoldMode {
    AddMissing,
    ForceClaude,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RescaffoldReport {
    /// 새로 생성된 파일 경로 (볼트 기준 상대경로)
    pub created: Vec<String>,
    /// 덮어쓴 `.claude/` 파일 (force-claude 모드에서만 채워짐)
    pub overwritten: Vec<String>,
    /// 덮어쓰기 전, 사용자가 수정한 것으로 감지된 `.claude/` 파일
    /// (force-claude: 덮어쓰기 대상이자 경고 대상. add-missing: 항상 빈 Vec)
    pub modified_by_user: Vec<String>,
    /// 건드리지 않은 파일 수 (템플릿 기준, 디렉토리 제외)
    pub unchanged: usize,
    /// dry_run 여부 (UI 확인용)
    pub dry_run: bool,
}

/// 볼트 재스캐폴드 실행 (또는 dry-run).
/// - root가 존재하지 않으면 `VaultNotFound`
/// - dry_run = true 이면 파일 변경 없음, 계산된 Report만 반환
pub fn rescaffold_vault(
    root: &Path,
    mode: RescaffoldMode,
    dry_run: bool,
) -> Result<RescaffoldReport, AppError>

/// IPC: 현재 활성 볼트에 대해 재스캐폴드 실행.
/// config_state에서 vault_path를 읽어 `rescaffold_vault`에 위임.
#[tauri::command]
pub fn rescaffold_active_vault(
    config_state: State<'_, ConfigState>,
    mode: RescaffoldMode,
    dry_run: bool,
) -> Result<RescaffoldReport, AppError>
```

## 내부 (테스트 가능 헬퍼)

```rust
/// 템플릿 파일의 절대경로와 디스크 파일의 내용이 바이트 단위로 동일한지.
/// - 디스크 파일 없음 → false
/// - 읽기 실패 → false (보수적으로 "다르다"로 간주 → force 모드에서 덮어쓰기)
fn matches_template(disk_path: &Path, template_content: &[u8]) -> bool

/// 한 파일에 대해 (모드/상태에 따라) 수행할 액션 결정.
/// 순수 함수 — I/O 없음, 상태 플래그만 받음.
pub fn plan_action(
    rel_path: &str,
    mode: RescaffoldMode,
    disk_exists: bool,
    disk_matches_template: bool,
) -> FileAction

#[derive(Debug, PartialEq, Eq)]
pub enum FileAction {
    Create,                  // 파일 없음 → 생성
    Overwrite { user_modified: bool }, // .claude/ 하위 + force 모드 + 디스크와 다름
    Skip,                    // 건드리지 않음
}
```

## Invariants

- `rescaffold_vault`는 **디렉토리는 항상 create_dir_all** (idempotent, 사용자 자산 영향 없음).
- **루트/사용자 자산** (`.gitignore`, `.gitattributes`, `CLAUDE.md`, `_moc/*`, `_templates/*`) — 모드와 무관하게 **없을 때만 생성**, **절대 덮어쓰지 않는다**.
- **`.claude/` 하위 템플릿 파일**:
  - `AddMissing` 모드: 없을 때만 생성, 있으면 skip.
  - `ForceClaude` 모드: 없으면 생성, 있으면 **내용 비교**:
    - 동일 → skip
    - 다름 → 덮어쓰고 `modified_by_user`에 기록 + `overwritten`에 기록
- **dry_run = true** 시: `fs::write` / `fs::create_dir_all` 어느 것도 호출하지 않음 (read-only).
- **훅 스크립트 재생성/덮어쓰기 시**: 기존 `scaffold_vault`와 동일하게 `.claude/hooks/*.sh` 실행 권한(0o755) 재부여.
- **사용자 추가 파일**은 절대 삭제/이동하지 않는다 (템플릿 외 파일은 무관심).
- `rescaffold_vault`가 **VaultNotFound** 외 에러로 실패 시 부분 진행된 상태일 수 있다 (partial 허용, 재실행으로 복구).
- Report의 `created` + `overwritten` + `unchanged` = 템플릿 파일 총 개수 (dry_run 여부와 무관, 모드 무관).

## Behavior Contract — plan_action (순수)

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 루트 `CLAUDE.md`, disk_exists=false, 어떤 모드든 | plan_action | `Create` |
| 2 | 루트 `CLAUDE.md`, disk_exists=true, 어떤 모드든 | plan_action | `Skip` |
| 3 | `_moc/Home.md`, disk_exists=true, `ForceClaude` | plan_action | `Skip` (사용자 자산은 force 대상 아님) |
| 4 | `.claude/settings.json`, disk_exists=false, `AddMissing` | plan_action | `Create` |
| 5 | `.claude/settings.json`, disk_exists=false, `ForceClaude` | plan_action | `Create` |
| 6 | `.claude/settings.json`, disk_exists=true, matches=true, `AddMissing` | plan_action | `Skip` |
| 7 | `.claude/settings.json`, disk_exists=true, matches=false, `AddMissing` | plan_action | `Skip` (add-missing은 절대 덮어쓰지 않음) |
| 8 | `.claude/settings.json`, disk_exists=true, matches=true, `ForceClaude` | plan_action | `Skip` (이미 최신과 동일) |
| 9 | `.claude/settings.json`, disk_exists=true, matches=false, `ForceClaude` | plan_action | `Overwrite { user_modified: true }` |

## Behavior Contract — matches_template

| # | Given | When | Then |
|---|-------|------|------|
| 10 | 디스크에 템플릿과 바이트 동일 파일 | matches_template | `true` |
| 11 | 디스크에 다른 내용의 파일 | matches_template | `false` |
| 12 | 디스크 파일 없음 | matches_template | `false` |
| 13 | 개행만 다른 파일 (LF vs CRLF) | matches_template | `false` (바이트 동일성 기준 — 의도적 엄격) |

## Behavior Contract — rescaffold_vault (통합)

| # | Given | When | Then |
|---|-------|------|------|
| 14 | 존재하지 않는 경로 | rescaffold_vault | `AppError::VaultNotFound` |
| 15 | 빈 디렉토리 + `AddMissing` + `dry_run=false` | rescaffold_vault | 모든 템플릿 파일 생성, `created.len() == 템플릿 파일 수`, `overwritten/modified_by_user`는 빈 Vec |
| 16 | 이미 스캐폴드된 볼트 (모든 파일 템플릿과 동일) + `AddMissing` | rescaffold_vault | `created == []`, `overwritten == []`, `unchanged == 템플릿 파일 수` |
| 17 | 이미 스캐폴드된 볼트 (`.claude/settings.json`만 사용자 수정) + `AddMissing` | rescaffold_vault | `created == []`, `overwritten == []`, `modified_by_user == []` (add-missing은 경고도 안함), `unchanged == 템플릿 파일 수` |
| 18 | 이미 스캐폴드된 볼트 (모든 파일 템플릿과 동일) + `ForceClaude` | rescaffold_vault | `overwritten == []`, `modified_by_user == []`, `unchanged == 템플릿 파일 수` |
| 19 | `.claude/settings.json`만 사용자 수정 + `ForceClaude` | rescaffold_vault | `overwritten == ["...settings.json"]`, `modified_by_user == ["...settings.json"]`, 나머지는 unchanged 또는 skip |
| 20 | `.claude/hooks/foo.sh` 삭제됨 + `AddMissing` | rescaffold_vault | 재생성됨 (`created`에 포함) + 실행 권한 0o755 복구 |
| 21 | `.claude/hooks/foo.sh` 사용자 수정 + `ForceClaude` | rescaffold_vault | 덮어쓰고 `modified_by_user`에 포함 + 실행 권한 재부여 |
| 22 | `.claude/skills/my-custom/SKILL.md` (사용자 추가, 템플릿 외) | rescaffold_vault | **건드리지 않음** — Report에 등장하지 않음 |
| 23 | 사용자가 `.claude/settings.json` 수정, `dry_run = true` + `ForceClaude` | rescaffold_vault | Report는 BC #19와 동일 (modified_by_user 포함), **실제 파일은 변경 없음** |
| 24 | dry_run = true, 누락된 파일 있음 + `AddMissing` | rescaffold_vault | `created` 리스트 채워짐, **디스크에는 파일 생성 안 됨** |

## Behavior Contract — rescaffold_active_vault (IPC)

| # | Given | When | Then |
|---|-------|------|------|
| 25 | `vault_path == None` | IPC | `AppError::VaultNotConfigured` |
| 26 | 유효한 활성 볼트 + `mode = "add-missing"` + `dry_run = true` | IPC | `RescaffoldReport` 반환, 파일 변경 없음 |
| 27 | 유효한 활성 볼트 + `mode = "force-claude"` + `dry_run = false` | IPC | 실제 적용 후 Report 반환 |

## Edge Cases

- **원자성 없음**: 파일 단위로 순차 실행. 중간 실패 시 partial 상태. 재실행(idempotent)으로 복구 유도 — 별도 롤백 없음.
- **대용량 파일 비교**: 템플릿이 모두 작은 텍스트 파일이므로 바이트 전체 비교 비용 무시 가능.
- **개행 스타일**: Windows에서 CRLF로 변환된 파일은 `matches_template`에서 "다름"으로 판정 → force 모드에서 LF로 덮어쓰기. 템플릿은 `include_str!` 기준(소스트리의 실제 바이트).
- **권한 에러** (읽기/쓰기 거부): `AppError::Io`로 전파. Report는 이미 처리된 항목까지만 채워짐.
- **동시성**: 재스캐폴드 중 watcher가 파일 변경을 감지 → `vault-changed` 이벤트가 다수 발생 가능. 프론트가 재스캐폴드 완료까지 debounce 처리하거나, 완료 후 전체 인덱스 재구축으로 정합성 확보.
- **볼트 활성 전환 중**: `rescaffold_active_vault` 호출 시점의 `vault_path` 스냅샷으로 실행 (RwLock read).
- **진행률 피드백**: 템플릿 파일 수가 많지 않아 (<50) 단일 blocking 호출로 충분. 스트리밍/진행률 이벤트는 범위 외.

## UI (Settings 페이지)

- Settings 하단에 "**볼트 업데이트**" 섹션 추가:
  - 모드 선택 라디오:
    - `[ ] 누락된 파일만 채우기 (안전)` — 기본 선택
    - `[ ] .claude/ 강제 업데이트` — 경고 아이콘 + 툴팁
  - **`[미리보기]` 버튼** → `rescaffold_active_vault(mode, dry_run=true)`
  - 리포트 카드 표시:
    - "새로 생성될 파일 N개" (접이식 목록)
    - "덮어쓸 파일 M개" (force-claude일 때만, 경고 색상, 각 경로에 ⚠️ 아이콘)
    - "변경 없음 K개"
  - **`[적용]` 버튼** — 미리보기 이후에만 활성. `dry_run=false`로 재호출. 성공 시 Toast (info 또는 success) 발화.
- 적용 후 통계 변경을 반영하기 위해 **인덱스 재구축** 트리거 (force-claude 모드에서 `.claude/` 변경은 인덱스와 무관하지만, 미래 확장성 위해 rescan 호출 고려 — 비필수).
- `.claude/` 변경은 기존 `vault-changed` 이벤트가 발생하므로 Claude 도구 뷰(`/claude`)가 자동 갱신된다.

## Dependencies

- 신규: `src-tauri/src/commands/rescaffold.rs` (또는 기존 `commands/onboarding.rs`에 공존)
- 재사용: `commands/onboarding.rs`의 `vault_files()`, `VAULT_DIRECTORIES`, `is_executable_script`, `set_executable`
  - 현재 `vault_files`는 `pub` 아님 → 모듈 내부 재배치 또는 `pub(crate)` 승격
- 수정: `lib.rs` — `rescaffold_active_vault` 핸들러 등록
- 신규: `src/lib/api.ts` — `rescaffoldActiveVault(mode, dryRun)` 래퍼
- 수정: `src/routes/settings/+page.svelte` — 볼트 업데이트 섹션 + 미리보기/적용 UI

## Mock boundary

- `plan_action` — 순수 enum 분기, 9개 조합 전수 테스트.
- `matches_template` — TempDir + 파일 작성 후 호출.
- `rescaffold_vault` — TempDir로 볼트 시뮬레이션. 다음 시나리오로 통합 테스트:
  1. 빈 볼트 → 전체 생성
  2. 완전히 일치하는 볼트 + 각 모드 → Report 형태 확인
  3. `.claude/settings.json` 사용자 수정 + 각 모드 → 분기 확인
  4. 사용자 추가 파일이 있어도 건드리지 않음 확인
  5. dry_run 동작 (디스크 변경 0 확인)
- `rescaffold_active_vault` — ConfigState 기반 smoke (유닛 1~2개 정도).

## 릴리즈 파이프라인 (Spec 범위 외 메모)

1. 외부 레포 `co-vault-storage-example` 훅/스킬 수정 완료
2. `src-tauri/templates/vault/.claude/` 에 해당 내용 재동기화 (수작업 복사 또는 스크립트화)
3. 앱 버전 bump + 릴리즈 노트에 명시: **"기존 볼트는 Settings > 볼트 업데이트 > .claude/ 강제 업데이트 실행"**

## 향후 (이 Spec의 범위 외)

- Claude Code 버전 추적: `.claude/settings.json`에 템플릿 버전 메타데이터 심어 "이 볼트의 템플릿 버전" 표시
- 선택적 덮어쓰기: 수정된 파일별 check/uncheck UI (전부 덮어쓰기 vs 일부만)
- 백업: 덮어쓰기 전 `.bak` 생성
- 사용자 자산 업데이트 옵션 (`_templates/*` 템플릿이 업데이트되었을 때) — 현재 spec 범위 밖
