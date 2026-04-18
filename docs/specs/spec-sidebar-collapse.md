# Spec: sidebar-collapse (v1.0 Epic 1.6 사이드바 접기/펴기)

## 전제

- Slice 1.4에서 사이드바가 `w-52` 고정 + 3그룹(탐색/작업/설정) 구조로 정착
- Slice 1.5의 `[data-theme]` 패턴을 재사용해 `[data-sidebar]` 속성 기반 CSS 분기
- AppConfig persist (density/theme와 동일 패턴)
- flash-of-wrong-state 회피를 위해 **localStorage + app.html 인라인 스크립트**로 첫 페인트 전 선반영
- collapsed 상태에서도 사이드바가 **완전히 숨겨지지는 않고**, 아이콘 레일(`w-12`)로 축소되어 최소한의 네비게이션을 유지

## Public Interface

### Rust (backend)

```rust
/// AppConfig에 추가
pub struct AppConfig {
    // ... 기존 필드
    /// 사이드바 접힘 여부. 구 config에 없으면 false(펼침).
    #[serde(default)]
    pub sidebar_collapsed: bool,
}

/// AppConfigPatch에 추가
pub struct AppConfigPatch {
    // ... 기존 필드
    pub sidebar_collapsed: Option<bool>,
}
```

`merged_with`는 기존 패턴과 동일하게 `Some`일 때만 덮어쓴다.

### TypeScript (frontend)

```ts
// src/lib/types.ts
export interface AppConfig {
  // ...
  sidebar_collapsed: boolean;
}

export interface AppConfigPatch {
  // ...
  sidebar_collapsed?: boolean;
}
```

### CSS (src/app.css)

```css
/* 기본(펼침): w-52 고정 너비, 모든 그룹 라벨/텍스트 보임 */
/* collapsed: w-12 아이콘 레일, 라벨/버전/볼트 이름 숨김 */
[data-sidebar="collapsed"] .sidebar-label,
[data-sidebar="collapsed"] .sidebar-version,
[data-sidebar="collapsed"] .sidebar-vault-name,
[data-sidebar="collapsed"] .sidebar-group-title {
  display: none;
}
[data-sidebar="collapsed"] .sidebar-root {
  width: 3rem;  /* w-12 = 48px */
}
```

Transition 150ms ease — width/padding 부드러운 전환 (prefers-reduced-motion 존중).

### 토글 store (신규)

기존 `themePref` 패턴 재사용하여 `src/lib/stores/sidebar.svelte.ts` 또는 `theme.svelte.ts`에 `sidebarCollapsed` 추가:

```ts
class SidebarStore {
  collapsed = $state(false);
  set(v: boolean): void { this.collapsed = v; }
  toggle(): void { this.collapsed = !this.collapsed; }
}
export const sidebarStore = new SidebarStore();
```

### Flash 방지 인라인 스크립트 (app.html 추가)

```html
<script>
  (function () {
    try {
      var v = localStorage.getItem("sidebar-collapsed");
      if (v === "true") {
        document.documentElement.dataset.sidebar = "collapsed";
      }
    } catch (e) { /* fallback: 기본 펼침 */ }
  })();
</script>
```

### 토글 버튼 위치

**헤더 우측** (집현 로고 옆). 현재 헤더는 로고만 있어 우측 여백이 비어 있음. `Slice 1.4`에서 제거한 `+ Clip` 자리와 같은 위치를 재활용.

collapsed 상태에서는 헤더의 로고("집현")를 숨기고 **토글 버튼만** 아이콘(`‹`/`›`)으로 보이게.

### 키보드 단축키

`Cmd/Ctrl + B` — VS Code와 동일한 관용 단축키. 전역(`+layout.svelte`의 `keydown` 리스너)에서 처리.

### 레이아웃 구조 (collapsed)

```
┌─────┐
│  ›  │  ← 토글(펴기)
├─────┤
│  ●  │  ← 활성 볼트 표시 (이름 생략, 점만)
├─────┤
│  📊 │  ← Dashboard
│  📁 │  ← Explore
│  🔗 │  ← Graph
├─────┤
│  ✂️  │  ← Clip
│  🎙️ │  ← Transcribe (disabled)
├─────┤
│  🤖 │  ← Claude
│  ⚙️ │  ← Settings
└─────┘
```

각 아이콘은 `title` 속성으로 라벨 tooltip 제공.

## Invariants

- `AppConfig::default().sidebar_collapsed == false` (기본 펼침)
- `serde(default)` → 구 config(필드 없음) 로드 시 false
- `AppConfigPatch { sidebar_collapsed: None }` → 기존 값 유지
- `html[data-sidebar]` 속성은 `"collapsed"` 또는 미설정 둘 중 하나 (펼침 = 속성 없음)
  - 명시성을 위해 `"expanded"` 대신 **속성 미설정**으로 펼침 상태 표현 (CSS 룰 적은 쪽이 기본)
- Transition 시간은 150ms 고정 — `prefers-reduced-motion: reduce`에서는 즉시 전환
- collapsed에서도 **활성 볼트 점(●)은 보임** — 현재 작업 컨텍스트 유지
- 그룹 구분선(border-b)은 collapsed에서도 유지 (그룹 분리감)
- 글로벌 단축키 Cmd/Ctrl+B는 `<input>`/`<textarea>` 포커스 중에는 미발동

## Behavior Contract — Rust AppConfig

| # | Given | When | Then |
|---|-------|------|------|
| 1 | `AppConfig::default()` | `.sidebar_collapsed` 접근 | `false` |
| 2 | 구 config.json (sidebar_collapsed 필드 없음) | `load_config` | `cfg.sidebar_collapsed == false` |
| 3 | `AppConfig { sidebar_collapsed: true, ..Default }` 저장 후 | `load_config` | `cfg.sidebar_collapsed == true` (roundtrip) |
| 4 | `base.sidebar_collapsed = false`, `patch.sidebar_collapsed = Some(true)` | `merged_with` | `next.sidebar_collapsed == true` |
| 5 | `base.sidebar_collapsed = true`, `patch.sidebar_collapsed = None` | `merged_with` | `next.sidebar_collapsed == true` (유지) |
| 6 | `base.sidebar_collapsed = true`, `patch.sidebar_collapsed = Some(false)` | `merged_with` | `next.sidebar_collapsed == false` (펼침 전환) |

## Behavior Contract — 프론트 (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 7 | `cfg.sidebar_collapsed = false`, 앱 첫 로드 | layout onMount | 사이드바 `w-52`, 모든 라벨/볼트 이름/버전 보임 |
| 8 | `cfg.sidebar_collapsed = true`, 앱 첫 로드 | 첫 페인트 | 사이드바 `w-12` 아이콘 레일로 선반영 (인라인 스크립트), 깜빡임 없음 |
| 9 | 토글 버튼 클릭 (펼침 → 접힘) | 클릭 즉시 | `html[data-sidebar="collapsed"]` 세팅, CSS transition 150ms, localStorage 동기화, `updateConfig({ sidebar_collapsed: true })` 호출 |
| 10 | 토글 버튼 클릭 (접힘 → 펼침) | 클릭 즉시 | `html[data-sidebar]` 제거, transition, localStorage/config 동기화 |
| 11 | Cmd/Ctrl + B 입력 (일반 상태) | keydown | 토글 버튼 클릭과 동일 동작 |
| 12 | 텍스트 input 포커스 중 Cmd/Ctrl + B | keydown | 토글 무시 (기본 브라우저 단축키 보존) |
| 13 | collapsed 상태에서 메뉴 아이템 호버 | hover | 아이콘만 보이되 `title` tooltip으로 라벨 노출 |
| 14 | collapsed 상태에서 Clip 아이콘 클릭 | 클릭 | WebClipDialog 오픈 (펼침과 동일) |
| 15 | collapsed 상태에서 Dashboard 아이콘 클릭 | 클릭 | `/` 경로 이동, 활성 하이라이트 유지 |
| 16 | collapsed × Compact 동시 적용 | 렌더 | 충돌 없이 각각 적용 (아이콘 레일 너비는 고정, 밀도 토큰만 축소) |
| 17 | collapsed × Light/Dark 각각 | 렌더 | 아이콘 레일이 각 테마의 surface/fg 색상으로 반영 |
| 18 | 앱 재시작 후 | 로드 | 직전 collapsed 상태 유지 (config + localStorage) |
| 19 | `prefers-reduced-motion: reduce` OS 설정 | 토글 | transition 없이 즉시 전환 |

## Settings UI

`/settings`의 디스플레이 섹션에 별도 토글은 **추가하지 않음**. 사이드바 자체의 버튼이 주 진입점.

이유: 밀도/테마는 초기화나 재확인이 필요한 설정이지만, collapsed는 한 번의 클릭으로 바로 되돌릴 수 있는 빈번한 토글이라 설정 페이지에 둘 당위성이 낮음.

## 이벤트 채널

`themeRefresh` 패턴 재사용 — 단, 사이드바 레이아웃 변경은 마크다운 뷰어 재렌더 같은 후속 작업이 필요없으므로 **별도 store 없이** `sidebarStore.collapsed` 상태만 바뀌면 `<html data-sidebar>` 업데이트 + CSS가 처리.

## Dependencies

- `src-tauri/src/config.rs` — `AppConfig.sidebar_collapsed` + `AppConfigPatch.sidebar_collapsed`
- `src/lib/types.ts` — 타입 추가
- `src/app.css` — `[data-sidebar="collapsed"]` 셀렉터 규칙
- `src/app.html` — flash 방지 인라인 스크립트
- `src/lib/stores/sidebar.svelte.ts` — 신규 (`sidebarStore`)
  - 또는 `theme.svelte.ts`에 추가 (스토어 모듈 과분화 방지 차원)
- `src/routes/+layout.svelte` — 토글 버튼, 단축키 리스너, CSS 클래스 마커(`sidebar-root`, `sidebar-label`, `sidebar-vault-name`, `sidebar-group-title`, `sidebar-version`)

## Mock Boundary

- Rust: 순수 config 테스트 (기존 패턴 연장)
- 프론트: 단위 테스트 프레임워크 미도입 → 수동 E2E
  - `npm run tauri dev` → 토글 버튼, 단축키, 재시작 유지, compact/theme 교차 검증

## 테스트 목록 (Rust 유닛)

1. `app_config_default_sidebar_collapsed_is_false`
2. `load_config_missing_sidebar_collapsed_field_uses_default`
3. `save_then_load_preserves_sidebar_collapsed_true`
4. `merged_with_applies_some_sidebar_collapsed`
5. `merged_with_none_sidebar_collapsed_keeps_original`
6. `merged_with_can_toggle_sidebar_collapsed`

## Edge Cases

- **인라인 스크립트 실패(CSP/disabled)**: try/catch → 기본 펼침으로 페이드
- **localStorage vs config 불일치**: Rust config가 ground truth. onMount에서 config 값으로 덮어씀. 초기 프레임에 localStorage 값이 보일 수 있지만 UX 영향 미미
- **모바일/좁은 창(Tauri는 데스크톱 전용이라 일반적 문제 아님)**: 반응형 auto-collapse는 **Out of Scope**
- **키보드 단축키 충돌**: Cmd/Ctrl+B는 브라우저에서 글자 굵게 등이 아닌 "북마크 바 표시"(Chrome) — Tauri 웹뷰에서는 문제 없음. 추후 충돌 보고 시 재검토
- **사이드바 내부 스크롤**: collapsed에서도 flex 레이아웃 유지, 아이콘이 넘치면 overflow-y: auto
- **토글 버튼 자체의 아이콘 색**: surface-1 위에서 fg-muted hover:text-fg 패턴 유지

## Out of Scope (Slice 1.6)

- 반응형 auto-collapse (창 너비 기준)
- 사이드바 너비 drag resize
- 볼트 목록 다수 시 스크롤
- 아이콘 레일에서 active volume 이름 tooltip (title 속성 기본값만)
- 사이드바 2-level fold (그룹별 접기)
- 설정 페이지에서 별도 toggle UI
- 키보드 단축키 커스터마이즈

## 열린 결정 (Slice 진입 전 확정)

- **토글 버튼 위치**: 헤더 우측(현 제안) vs footer vs 사이드바 바깥 가장자리 → **헤더 우측** 권장 (발견성 + 제거된 +Clip 자리 재활용)
- **collapsed에서 볼트 섹션 표현**: 활성 볼트 점(●)만(현 제안) vs 섹션 전체 숨김 vs 📓 아이콘만 → **활성 점(●)** 권장 (컨텍스트 유지 + 시각 노이즈 최소)
- **키보드 단축키 포함**: Cmd/Ctrl+B(현 제안) vs 없음 → **포함** 권장 (파워 유저 기대치)
- **flash 방지**: localStorage + 인라인 스크립트(현 제안) vs 없음 → **포함** 권장 (theme와 일관)
- **collapsed 아이콘**: 유니코드 화살표 `‹ / ›`(현 제안) vs `☰ / ✕` vs SVG → **유니코드** 권장 (기존 이모지 스타일과 일관)
- **Store 분리**: `sidebar.svelte.ts` 신규 파일 vs `theme.svelte.ts`에 추가 → **theme.svelte.ts에 추가** 권장 (store 과분화 방지, UI layout 관련 상태 묶기)
