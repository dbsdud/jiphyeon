# Spec: theme-switcher (v1.0 Epic 1.5 라이트/다크 테마 스위칭)

## 전제

- Slice 1.1 토큰은 **다크 팔레트 전용** — 색상 변수가 모두 `@theme` 블록에 하드코딩됨
- Slice 1.3에서 `html[data-theme]` 분기 훅을 심어둠:
  - `src/lib/markdown/mermaid.ts`의 `currentTheme()` 함수가 `document.documentElement.dataset.theme === "dark" ? "dark" : "default"` 반환
  - `src/app.css`의 주석(`"테마 전환 슬라이스에서 html[data-theme]에 따라 light와 분기 예정"`) — 토큰은 아직 분기되지 않음
- Slice 1.2 density 패턴(AppConfig 필드 + `<html data-*>` 속성 + serde default)을 **테마에도 재사용**
- 라이트/다크 토큰 두 팔레트를 런타임에 전환 → **유틸리티 재생성 불필요** (CSS 변수 cascade로 자동 반영)
- OS 다크 모드 연동(`System` 옵션) 요구 — `matchMedia("(prefers-color-scheme)")` 구독

## Public Interface

### Rust (backend)

```rust
/// 사용자 테마 선호.
/// - Light/Dark: 명시적 고정
/// - System: OS 다크 모드 설정에 연동 (prefers-color-scheme)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

/// AppConfig에 추가
pub struct AppConfig {
    // ... 기존 필드
    /// 테마 선호. 구 config에 없으면 System.
    #[serde(default)]
    pub theme: ThemePreference,
}

/// AppConfigPatch에 추가
pub struct AppConfigPatch {
    // ... 기존 필드
    pub theme: Option<ThemePreference>,
}
```

### TypeScript (frontend)

```ts
// src/lib/types.ts
export type ThemePreference = "light" | "dark" | "system";
/** resolved(실제 적용) 테마 — system은 OS 설정으로 계산된 값. */
export type ResolvedTheme = "light" | "dark";

export interface AppConfig {
  // ...
  theme: ThemePreference;
}

export interface AppConfigPatch {
  // ...
  theme?: ThemePreference;
}
```

### 테마 store (신규 — `src/lib/stores/theme.svelte.ts`)

```ts
/** 설정된 선호 + OS 다크모드 여부로 resolved 테마를 계산. */
export function resolveTheme(pref: ThemePreference): ResolvedTheme {
  if (pref === "light") return "light";
  if (pref === "dark") return "dark";
  // system: prefers-color-scheme
  if (typeof window === "undefined") return "dark"; // SSR 대비, SPA에서 실사용 없음
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/** `<html data-theme>` 속성을 세팅. */
export function applyTheme(resolved: ResolvedTheme): void {
  document.documentElement.dataset.theme = resolved;
}

/** System 선호 시 OS 변경 이벤트 구독. 반환된 unsubscribe를 onDestroy에서 호출. */
export function watchSystemTheme(onChange: (r: ResolvedTheme) => void): () => void {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (e: MediaQueryListEvent) =>
    onChange(e.matches ? "dark" : "light");
  mq.addEventListener("change", handler);
  return () => mq.removeEventListener("change", handler);
}
```

### CSS 구조 재편 (`src/app.css`)

**원칙**: 컬러 변수는 `@theme` 블록에 **기본값**만 선언하고(Tailwind 유틸 생성용), `[data-theme]` 블록에서 런타임 오버라이드.

```css
@theme {
  /* 컬러 — 기본값(fallback). dark 값과 동일하게 둔다. */
  --color-surface-0: #0e0e0e;
  --color-surface-1: #171717;
  /* ... 기타 색상 토큰 */

  /* 구조 토큰 — 테마 영향 없음 */
  --font-sans: ...;
  --spacing: 4px;
  --radius-md: 6px;
  /* ... */
}

/* 라이트 팔레트 오버라이드
   디자인 테제: "햇살 들어오는 도서관" — 웜오프화이트 기반, 순백 회피.
   모든 수치 WCAG AA 검증 완료 (fg: 15.2:1, fg-muted: 4.9:1, border-strong UI: 3.05:1,
   accent text: 6.4:1, semantic ≥ 4.5:1). */
[data-theme="light"] {
  /* Surface — N↑ = 베이스에서 더 멀어지는 톤 (다크와 의미 일관성) */
  --color-surface-0: #fafaf7;  /* 앱 기본 배경 (웜오프화이트) */
  --color-surface-1: #f3f2ed;  /* 사이드바, 살짝 차별화 */
  --color-surface-2: #e8e6df;  /* 카드/코드블록 배경 */
  --color-surface-3: #d9d7ce;  /* 최강조 (hover/active) */

  /* Foreground — 순흑(#000) 회피, 장문 읽기 피로감 완화 */
  --color-fg:       #1c1c1c;
  --color-fg-muted: #6b6b6b;
  --color-fg-dim:   #9a9a9a;

  /* Border — strong은 AA UI 3:1 확보 */
  --color-border:        #e2e0d7;
  --color-border-strong: #b8b5a9;

  /* Accent — blue-500(dark) ↔ blue-600(light) 한 톤 진하게 */
  --color-accent:       #2563eb;
  --color-accent-hover: #1d4ed8;
  --color-accent-dim:   #dbeafe;  /* wikilink underline, focus ring bg */
  --color-accent-fg:    #ffffff;  /* accent 배경 위 텍스트 (양쪽 테마 동일) */

  /* Semantic — 라이트에선 텍스트 읽힘 위해 톤 진하게 (Tailwind 600~700) */
  --color-success: #15803d;
  --color-warning: #a16207;
  --color-danger:  #b91c1c;
  --color-info:    #0e7490;

  /* Legacy alias 재바인딩 (Slice 1.1 alias 유지) */
  --color-surface: var(--color-surface-0);
  --color-muted:   var(--color-fg-muted);
}

/* 다크 팔레트 — @theme 기본값과 중복이지만 명시적 분기 */
[data-theme="dark"] {
  --color-surface-0: #0e0e0e;
  /* ... 기존 값 그대로 */
}
```

> 라이트 팔레트는 `/design-consultation` 컨설팅(2026-04-18)으로 확정됨. 디자인 테제 "햇살 들어오는 도서관", WCAG AA 전 항목 검증 완료.

### 마크다운 본문 스타일 라이브러리 — github-markdown-css → @tailwindcss/typography

**결정 (2026-04-18 구현 중 변경)**: `github-markdown-css`를 제거하고 `@tailwindcss/typography` 도입.

**배경**:
- 초기 계획은 dark/light 두 CSS를 모두 번들하고 `.markdown-body` 변수 오버라이드로 테마 전환하는 방식
- 실제 검증 결과 `github-markdown-css` 파일은 **하드코딩 hex 71개 + CSS 변수 0개** 구조 → 우리 변수 오버라이드는 무효였고, 두 파일이 공존하면 후순위(light)가 다크 모드 텍스트를 덮어써 테이블/코드/Mermaid SVG가 배경에 묻힘

**신 전략**:
- `@plugin "@tailwindcss/typography"` (Tailwind 4 네이티브 CSS 플러그인 로딩)
- `.prose` 클래스가 `--tw-prose-*` CSS 변수 16개를 사용하는 구조 → 각 변수를 우리 디자인 토큰에 매핑하면 `[data-theme]` 전환이 자동 반영
- `.prose-invert` 변수도 동일 토큰 사용 → 다크/라이트 전환 시 prose 클래스 하나로 일관
- 기존 `.markdown-body`는 **우리 앱 고유 확장만** 담당 (wikilink, `.diagram`, Prism 토큰, KaTeX)
- 마크업에는 `class="markdown-body prose prose-sm max-w-none"` 둘 다 적용

### Mermaid 테마 동기화

- Mermaid는 `mermaid.initialize({ theme })`로 전역 테마를 설정. 이미 렌더된 SVG는 자동 갱신되지 않음
- **전략**: 테마 전환 시 **현재 뷰어의 기존 렌더 마커를 초기화하고 전체 파이프라인 재실행**
  - `data-mermaid-rendered` / `data-math-rendered` / 유사 마커 제거
  - 재치환된 `<pre><code class="language-mermaid">` 원본이 남아 있지 않은 경우가 있으므로, **뷰어에서 HTML 재삽입**(원본 HTML 재-set → pipeline 재실행)으로 처리
- `src/routes/view/+page.svelte` 측에서 테마 변경 이벤트를 구독하고 현재 노트를 강제 재렌더

### KaTeX / Prism

- KaTeX: 색상은 상속(`currentColor` 기반) → 토큰 변경만으로 자동 반영. 별도 동기화 불필요
- Prism: `app.css`의 `.markdown-body pre code .token.*` 규칙이 이미 토큰 변수 기반 → 자동 반영

### Flash of Wrong Theme 방지

첫 페인트 전에 테마를 결정해야 함. `src/app.html`에 **인라인 스크립트** 삽입:

```html
<!-- src/app.html <head> 최상단 -->
<script>
  (function() {
    try {
      var pref = localStorage.getItem("theme-pref") || "system";
      var resolved = pref === "system"
        ? (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
        : pref;
      document.documentElement.dataset.theme = resolved;
    } catch (e) {
      document.documentElement.dataset.theme = "dark";
    }
  })();
</script>
```

- `+layout.svelte`의 `onMount`에서 config 로드 후 실제 값으로 한 번 더 덮어씀 (config와 localStorage 불일치 시 config 우선)
- localStorage 동기화는 save 시점에 추가 (`localStorage.setItem("theme-pref", pref)`)

## Invariants

- `ThemePreference::default() == System`
- serde `"light"` / `"dark"` / `"system"` 세 값만 허용
- 구 config.json (theme 필드 없음) → `System`으로 로드
- `AppConfigPatch { theme: None }` → 기존 값 유지
- `<html data-theme>` 속성 값은 항상 `"light"` 또는 `"dark"` (resolved, system은 존재하지 않음)
- 다크 팔레트는 **Slice 1.1 현재 값과 픽셀 완전 일치** (회귀 방지)
- 라이트 팔레트 전환 시 **WCAG AA 대비** 유지 (텍스트 4.5:1, UI 컴포넌트 3:1)
- Mermaid/KaTeX/github-markdown 모두 `data-theme` 바뀌면 일관된 테마로 보임
- 토큰 외 하드코딩 hex(`#[0-9a-f]{6}`)는 Slice 1.1 이후 잔존하지 않아야 함 (본 Slice 진입 전 grep 재검증)

## Behavior Contract — ThemePreference serde

| # | Given | When | Then |
|---|-------|------|------|
| 1 | `ThemePreference::Light` | `serde_json::to_string` | `"\"light\""` |
| 2 | `ThemePreference::Dark` | `serde_json::to_string` | `"\"dark\""` |
| 3 | `ThemePreference::System` | `serde_json::to_string` | `"\"system\""` |
| 4 | `"\"light\"" / "\"dark\"" / "\"system\""` | `from_str::<ThemePreference>` | 대응 variant |
| 5 | `"\"invalid\""` | `from_str::<ThemePreference>` | Err |
| 6 | `ThemePreference::default()` | — | `System` |

## Behavior Contract — AppConfig migration

| # | Given | When | Then |
|---|-------|------|------|
| 7 | 구 config.json (theme 필드 없음) | `load_config` | `cfg.theme == System` |
| 8 | `AppConfig { theme: Light, ..Default }` 저장 후 | `load_config` | `cfg.theme == Light` (roundtrip) |
| 9 | 기본 `AppConfig` | `.theme` 접근 | `System` |

## Behavior Contract — AppConfigPatch merge

| # | Given | When | Then |
|---|-------|------|------|
| 10 | `base.theme = System`, `patch.theme = Some(Light)` | `merged_with` | `next.theme == Light` |
| 11 | `base.theme = Dark`, `patch.theme = None` | `merged_with` | `next.theme == Dark` (유지) |
| 12 | `base.theme = Light`, `patch.theme = Some(System)` | `merged_with` | `next.theme == System` |
| 13 | 빈 patch(`AppConfigPatch::default()`) | `merged_with` | theme 포함 모든 필드 유지 |

## Behavior Contract — 프론트 적용 (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 14 | `cfg.theme = "dark"`, OS 무관 | 앱 첫 로드 | `<html data-theme="dark">`, 배경 #0e0e0e |
| 15 | `cfg.theme = "light"`, OS 무관 | 앱 첫 로드 | `<html data-theme="light">`, 배경 #ffffff |
| 16 | `cfg.theme = "system"`, OS = dark | 앱 첫 로드 | `<html data-theme="dark">` |
| 17 | `cfg.theme = "system"`, OS = light | 앱 첫 로드 | `<html data-theme="light">` |
| 18 | `cfg.theme = "system"` 상태에서 OS 토글 | matchMedia change | `<html data-theme>`이 즉시 전환, 앱 전체 색상 반영 |
| 19 | Settings에서 Light → Dark 저장 | 저장 성공 | `<html data-theme="dark">`, 재로드 불필요 |
| 20 | 테마 전환 시 현재 뷰어의 마크다운 | 재렌더 | github-markdown/Mermaid/KaTeX/Prism이 새 테마로 보임 |
| 21 | 새로고침(F5) | 첫 페인트 | `app.html` 인라인 스크립트가 `<html data-theme>`을 선반영 → 깜빡임 없음 |
| 22 | localStorage `theme-pref` 없음 + config `dark` | 첫 페인트 | 인라인 스크립트는 `system` fallback → config 로드 후 `dark`로 교체. 초기 프레임이 OS에 따라 잠깐 다를 수 있음(아래 Edge Cases 참조) |

## Behavior Contract — Mermaid 재렌더

| # | Given | When | Then |
|---|-------|------|------|
| 23 | 뷰어에 Mermaid SVG 3개 렌더됨 (dark) | 테마 Light로 전환 | 3개 모두 light 테마로 재렌더 (파싱 에러 없음) |
| 24 | 뷰어 진입 중에 테마 전환 | pipeline 실행 중 | 현재 실행은 기존 테마로 완료, 이후 재렌더 트리거 |
| 25 | Mermaid parse 실패한 블록 존재 | 테마 전환 | 여전히 에러 오버레이 유지 (테마가 버그를 고치지 않음) |

## Settings UI

`/settings` 페이지 디스플레이 섹션에 **테마 선택** 신규 추가:

```svelte
<div class="text-xs text-fg-muted mb-2">테마</div>
<div class="space-y-2">
  <label>
    <input type="radio" value="system" bind:group={theme} />
    <div>
      <div>시스템 설정 따르기 (System)</div>
      <div class="text-xs text-fg-muted">OS의 다크 모드 선호에 맞춰 자동 전환</div>
    </div>
  </label>
  <label>
    <input type="radio" value="light" bind:group={theme} />
    <div>밝은 테마 (Light)</div>
  </label>
  <label>
    <input type="radio" value="dark" bind:group={theme} />
    <div>어두운 테마 (Dark)</div>
  </label>
</div>
```

저장 성공 후:
1. `document.documentElement.dataset.theme = resolveTheme(saved.theme)`
2. `localStorage.setItem("theme-pref", saved.theme)` (flash 방지 동기화)
3. 뷰어 재렌더 이벤트 발행 (Mermaid 테마 갱신)

## 이벤트 채널

전역 테마 변경 알림 채널 (뷰어 재렌더 트리거용):

- **옵션 A (권장)**: 커스텀 이벤트 `window.dispatchEvent(new CustomEvent("theme-changed"))` — 기존 `vault-changed` 패턴과 동일
- 옵션 B: 테마 store를 export하여 consumer가 `$effect`로 반응 — 더 svelte스럽지만 `src/lib/stores/vault.svelte.ts`의 bump 패턴과 중복

→ **옵션 B** 채택(신규 `themeRefresh` store), density/theme 설정 저장 시 `themeRefresh.bump()` 호출. 기존 vault store와 동일 패턴.

## Dependencies

- `config.rs` — `ThemePreference` enum + AppConfig/AppConfigPatch 필드 + merge 로직
- `src/lib/types.ts` — 타입 추가
- `src/app.css` — 컬러 변수를 `[data-theme="light"]` / `[data-theme="dark"]` 블록으로 분리, light-github CSS 추가 import
- `src/app.html` — 첫 페인트 전 인라인 스크립트
- `src/lib/stores/theme.svelte.ts` — 신규 (`resolveTheme`, `applyTheme`, `watchSystemTheme`, `themeRefresh`)
- `src/routes/+layout.svelte` — onMount에서 초기 테마 적용 + system 구독
- `src/routes/settings/+page.svelte` — 테마 라디오 추가
- `src/routes/view/+page.svelte` — `themeRefresh`를 $effect로 관찰 → 재렌더
- `src/lib/markdown/mermaid.ts` — 마커 초기화 지원(기존 `data-mermaid-rendered` 리셋 헬퍼)

## Mock Boundary

- Rust: 순수 serde/config 테스트 (기존 패턴 연장)
- 프론트: 단위 테스트 프레임워크 미도입 → 수동 E2E
  - `npm run tauri dev` → 세 옵션 토글, OS 다크모드 토글, 뷰어 재렌더 확인

## 테스트 목록 (Rust 유닛)

1. `theme_pref_serializes_to_lowercase` — 3 variants → JSON 문자열
2. `theme_pref_deserializes_from_lowercase` — JSON 문자열 → variant
3. `theme_pref_rejects_invalid_string` — 잘못된 값 Err
4. `theme_pref_default_is_system`
5. `app_config_default_theme_is_system`
6. `load_config_missing_theme_field_uses_default` — 구 config 로드
7. `save_then_load_preserves_light_theme`
8. `save_then_load_preserves_dark_theme`
9. `merged_with_applies_some_theme`
10. `merged_with_none_theme_keeps_original`
11. `merged_with_can_switch_theme` — 각 조합

## Edge Cases

- **인라인 스크립트 실패(CSP 등)**: try/catch로 감싸고 fallback `dark` — 기존 UX 유지
- **localStorage 비활성 환경**: 인라인 스크립트는 catch 블록에서 기본 `dark`, config 로드 후 실제 값으로 덮어씀
- **config.theme 없고 localStorage.theme-pref 있음**: 인라인 스크립트가 localStorage 우선 → config 로드 후 Rust 기본값(`System`)이 이를 덮어씀 → resolved가 일치할 가능성 높음. 불일치 시 config 우선(Rust가 ground truth)
- **System 모드에서 OS 설정 중간 전환**: `watchSystemTheme`가 구독, 즉시 반영
- **비뷰어 페이지에서 테마 전환**: 재렌더 트리거가 무의미(effect는 뷰어에만 바인딩). 뷰어 재진입 시 자연스럽게 반영
- **동시에 density + theme 저장**: 단일 `updateConfig` patch로 함께 처리되므로 경합 없음
- **WebClipDialog 등 모달 내부 색상**: 모든 UI 컴포넌트가 토큰 기반이어야 함. 하드코딩 잔존 시 Slice 1.5 수동 검증에서 추가 fix 이슈화
- **legacy alias(`--color-surface`, `--color-muted`)**: 각 테마 블록에서 재바인딩. 제거는 별도 이슈(Slice 1.1 말미 정리 미완)

## Out of Scope (Slice 1.5)

- 라이트 테마 전용 이미지/아이콘 자산 (현재 이모지만 사용, SVG 자산 없음)
- 고대비(high-contrast) 모드
- 색약 친화 팔레트 토글
- 다크에서만 적용되는 drop-shadow 톤 보정(필요 시 Slice 1.6 이후)
- Mermaid 테마 커스터마이즈(dark 외 base/forest 등)
- 사이드바 collapse/expand (Slice 1.6)
- 테마별 KaTeX 수식 색상 미세 조정

## 열린 결정 (Slice 진입 전 확정)

모든 열린 결정은 2026-04-18 확정되었음.

- ✅ **라이트 팔레트 수치**: `/design-consultation` 컨설팅으로 확정
- ✅ **기본값**: `System`
- ✅ **Mermaid 기존 SVG 재렌더**: 즉시 재렌더
- ✅ **github-markdown CSS 로딩**: 두 파일 모두 번들
- ✅ **토글 위치**: Settings에만 (사이드바 퀵 토글은 Slice 1.6 collapse와 함께 재검토)
- ✅ **localStorage 키**: `"theme-pref"`
