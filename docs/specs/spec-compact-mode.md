# Spec: compact-mode (v1.0 Epic 1.2 컴팩트/기본 밀도 토글)

## 전제

- Slice 1.1에서 도입된 디자인 토큰(`--spacing`, `--text-*`)이 **런타임에 오버라이드 가능한 구조**로 선언되어 있다
- 컴팩트 모드는 **글로벌 상태**다 — 볼트별 설정이 아니라 앱 전체 설정
- 저장 즉시 전역 반영되어야 한다 (앱 재시작 불필요)
- 라이트/다크 테마는 v1.0 비범위 → 컴팩트 모드는 색상과 독립

## Public Interface

### Rust (backend)

```rust
/// 앱의 UI 밀도 모드.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Regular,
    Compact,
}

/// AppConfig에 추가
pub struct AppConfig {
    // ... 기존 필드
    /// UI 밀도 모드. 구 config에 없으면 Regular.
    #[serde(default)]
    pub density: Density,
}

/// AppConfigPatch에 추가
pub struct AppConfigPatch {
    // ... 기존 필드
    pub density: Option<Density>,
}
```

`merged_with`는 기존 패턴과 동일하게 `Some`일 때만 덮어쓴다.

### TypeScript (frontend)

```ts
// src/lib/types.ts
export type Density = "regular" | "compact";

export interface AppConfig {
  // ...
  density: Density;
}

export interface AppConfigPatch {
  // ...
  density?: Density;
}
```

### CSS (src/app.css)

```css
@theme {
  /* Regular (기본) — Slice 1.1에서 정의된 값 */
  --spacing: 4px;
  --text-xs: 11px;
  --text-sm: 12px;
  --text-base: 13px;
  --text-md: 14px;
  --text-lg: 16px;
  --text-xl: 18px;
  --text-2xl: 22px;
  --text-3xl: 28px;
}

/* Compact 모드 오버라이드 */
[data-density="compact"] {
  --spacing: 3px;          /* 75% — 모든 p-N/m-N/gap-N 동시 축소 */
  --text-xs: 10px;
  --text-sm: 11px;
  --text-base: 12px;
  --text-md: 13px;
  --text-lg: 14px;
  --text-xl: 16px;
  --text-2xl: 19px;
  --text-3xl: 24px;
}
```

### 프론트 전역 적용 (layout + settings)

```ts
// src/routes/+layout.svelte onMount
const cfg = await getConfig();
document.documentElement.dataset.density = cfg.density;

// src/routes/settings/+page.svelte save 성공 후
document.documentElement.dataset.density = saved.density;
```

## Invariants

- `Density::default() == Density::Regular`
- `serde(rename_all = "lowercase")` → JSON에서 `"regular"` / `"compact"`
- 구 config.json (필드 없음) 로드 → `density == Regular`
- `AppConfigPatch { density: None }` → 기존 density 유지
- `AppConfigPatch { density: Some(v) }` → 새 값으로 덮어쓰기
- 컴팩트 모드는 **spacing(1개 변수) + text-* (8개 변수)만** 오버라이드
  - 색상/radius/shadow/font-family/font-weight/line-height는 불변
- `<html data-density>` 속성 값은 `"regular"` 또는 `"compact"` 둘 중 하나 (JSON 문자열과 일치)

## Behavior Contract — Density serde

| # | Given | When | Then |
|---|-------|------|------|
| 1 | `Density::Regular` | `serde_json::to_string` | `"\"regular\""` |
| 2 | `Density::Compact` | `serde_json::to_string` | `"\"compact\""` |
| 3 | `"\"regular\""` | `serde_json::from_str::<Density>` | `Density::Regular` |
| 4 | `"\"compact\""` | `serde_json::from_str::<Density>` | `Density::Compact` |
| 5 | `"\"invalid\""` | `serde_json::from_str::<Density>` | Err |

## Behavior Contract — AppConfig migration

| # | Given | When | Then |
|---|-------|------|------|
| 6 | 구 config.json (density 필드 없음) | `load_config` | `cfg.density == Regular` |
| 7 | `AppConfig { density: Compact, ..Default }` 저장 후 | `load_config` | `cfg.density == Compact` (roundtrip) |
| 8 | 기본 `AppConfig` | `.density` 접근 | `Regular` |

## Behavior Contract — AppConfigPatch merge

| # | Given | When | Then |
|---|-------|------|------|
| 9 | `base.density = Regular`, `patch.density = Some(Compact)` | `merged_with` | `next.density == Compact` |
| 10 | `base.density = Compact`, `patch.density = None` | `merged_with` | `next.density == Compact` (유지) |
| 11 | `base.density = Compact`, `patch.density = Some(Regular)` | `merged_with` | `next.density == Regular` (전환) |
| 12 | 빈 patch(`AppConfigPatch::default()`) | `merged_with` | density 포함 모든 필드 유지 |

## Behavior Contract — 프론트 전역 적용

이 섹션은 Svelte 수동 검증 대상.

| # | Given | When | Then |
|---|-------|------|------|
| 13 | 앱 첫 로드, `cfg.density = "regular"` | layout `onMount` | `<html data-density="regular">` |
| 14 | 앱 첫 로드, `cfg.density = "compact"` | layout `onMount` | `<html data-density="compact">` |
| 15 | Settings에서 Regular → Compact 저장 | 저장 성공 이후 | `<html>` 속성이 `compact`로 변경, 페이지 전체 간격/폰트 축소 (재로드 불필요) |
| 16 | Settings에서 Compact → Regular 저장 | 저장 성공 이후 | `<html>` 속성이 `regular`로 변경, 페이지 전체 간격/폰트 원복 |

## Settings UI

- `/settings` 페이지에 "디스플레이" 또는 "밀도" 섹션 신규 추가
- 라디오 2개: `Regular (기본)`, `Compact`
- 현재 선택값은 `config.density`에서 초기화
- 저장 시 `updateConfig({ density })` 호출, 성공 후 `document.documentElement.dataset.density = saved.density`

```svelte
<section>
  <h2>밀도</h2>
  <label><input type="radio" value="regular" bind:group={density} /> Regular (기본)</label>
  <label><input type="radio" value="compact" bind:group={density} /> Compact</label>
</section>
```

## Edge Cases

- **onMount 이전 초기 렌더링**: `<html>` 속성 설정 전 렌더되면 잠깐 Regular로 보일 수 있음 → SvelteKit SSR 비활성(SPA)이므로 CSR 시작 시점에 onMount 전 영향은 무시 가능. 필요 시 `app.html`에 인라인 스크립트로 localStorage 기반 초기값 설정 (MVP 범위 외)
- **토글 중 로딩 상태**: 저장 버튼 `saving` 상태 동안 라디오 비활성화 → UX 일관성
- **밀도 변경 시 그래프 재계산**: 링크 그래프(d3-force)는 픽셀 기반 레이아웃 → 밀도 변경 시 자동으로 재적용되지는 않음. 페이지 이동 후 재진입 시 반영. 이 한계는 Slice 1.2 비범위
- **Tailwind arbitrary value 사용 코드**: `p-[16px]`처럼 하드코딩된 픽셀은 컴팩트에서 축소되지 않음 → Slice 1.1 마이그레이션에서 모두 제거되었는지 grep 재검증

## Dependencies

- 기존 `config::{AppConfig, AppConfigPatch, merged_with, save_config, load_config}` — 필드 2개 추가
- 기존 `commands/settings::{get_config, update_config}` — 변경 없음 (patch 구조 확장으로 자동 수용)
- Svelte: `+layout.svelte`의 `onMount`에서 getConfig 호출 — 기존에는 vaultStatus만 로드했는데 config도 한 번 로드 필요
- CSS: `app.css`의 `[data-density="compact"]` 블록 신규 추가

## Mock Boundary

- `Density` serde는 순수 함수 유닛 테스트
- `AppConfig`/`AppConfigPatch` 확장은 기존 config.rs 테스트 패턴 연장
- 프론트 전역 적용은 수동 E2E (`npm run tauri dev` → Settings에서 토글)

## 테스트 목록 (Rust 유닛)

1. `density_serializes_to_lowercase_string` — enum variant → JSON 문자열
2. `density_deserializes_from_lowercase_string` — JSON 문자열 → enum variant
3. `density_default_is_regular`
4. `app_config_default_density_is_regular`
5. `load_config_missing_density_field_uses_default` — 구 config.json 로드
6. `save_then_load_preserves_compact_density`
7. `merged_with_applies_some_density`
8. `merged_with_none_density_keeps_original`
9. `merged_with_can_switch_compact_to_regular`

## Out of Scope (Slice 1.2)

- 다크/라이트 테마 토글
- 폰트 패밀리 선택 UI
- 밀도 3단계(편안/기본/컴팩트)
- 링크 그래프 밀도 반응형 재계산
- 밀도별 프리뷰 이미지
- 시스템 설정과 연동 (macOS Reduce Motion 등)
