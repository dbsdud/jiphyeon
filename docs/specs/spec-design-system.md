# Spec: design-system (v1.0 Epic 1.1 디자인 토큰)

## 전제

- Tailwind CSS 4의 `@theme` 지시자를 통해 CSS 변수를 Tailwind 유틸리티 클래스로 연결한다
- 모든 토큰은 **CSS 변수**로 선언되며, 컴팩트 모드(Slice 1.2)가 런타임에 일부 변수를 오버라이드할 수 있어야 한다
- 색상은 다크 테마 기준. 라이트 테마는 v1.0 비범위
- 토큰은 **의미적 이름**(semantic name) 우선, 값 기반 이름(예: `gray-900`) 지양
- 현재 17개 Svelte 파일에서 403개의 Tailwind 유틸리티와 60개의 하드코딩 색상이 사용 중 — Slice 1.1 완료 시점에 모두 토큰 경유로 전환

## Token Interface

### Color

Tailwind 4는 `--color-*` 네임스페이스에서 변수 이름 뒤 부분을 그대로 유틸리티 이름으로 사용한다(`--color-surface-1` → `bg-surface-1`). 따라서 **중첩된 네임스페이스 방지**를 위해 `text`/`content` 접두사 대신 **`fg`(foreground)** 를 사용한다.

```css
/* Surface — 배경 레이어링 */
--color-surface-0: #0e0e0e;   /* base (앱 바탕) */
--color-surface-1: #171717;   /* nav, card */
--color-surface-2: #1f1f1f;   /* elevated card, input */
--color-surface-3: #2a2a2a;   /* hover state */

/* Foreground — 텍스트 색상 → text-fg, text-fg-muted, ... */
--color-fg: #f5f5f5;          /* 기본 텍스트 */
--color-fg-muted: #8a8a8a;    /* 보조 텍스트 */
--color-fg-dim: #555555;      /* disabled */

/* Border */
--color-border: #2e2e2e;         /* 기본 → border-border */
--color-border-strong: #3f3f3f;

/* Accent */
--color-accent: #3b82f6;
--color-accent-hover: #2563eb;
--color-accent-dim: #1e3a5f;

/* Semantic */
--color-success: #22c55e;
--color-warning: #eab308;
--color-danger: #ef4444;
--color-info: #06b6d4;
```

**생성되는 유틸리티**:
- `bg-surface-0/1/2/3`
- `text-fg`, `text-fg-muted`, `text-fg-dim`
- `border-border`, `border-border-strong`
- `text-accent`, `bg-accent`, `bg-accent-hover`, `bg-accent-dim`
- `text-success/warning/danger/info`, `bg-*`, `border-*`

### Typography

```css
/* Font family — Tailwind: font-sans / font-mono */
--font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans KR", system-ui, sans-serif;
--font-mono: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, "Cascadia Code", monospace;

/* Font size — Tailwind: text-xs ~ text-3xl (density-aware) */
--text-xs: 11px;
--text-sm: 12px;
--text-base: 13px;     /* 앱 기본 */
--text-md: 14px;
--text-lg: 16px;       /* 마크다운 본문 기본 */
--text-xl: 18px;
--text-2xl: 22px;
--text-3xl: 28px;

/* Line height (text-*에 페어링) */
--text-xs--line-height: 1.4;
--text-sm--line-height: 1.4;
--text-base--line-height: 1.5;
--text-md--line-height: 1.5;
--text-lg--line-height: 1.6;
--text-xl--line-height: 1.5;
--text-2xl--line-height: 1.4;
--text-3xl--line-height: 1.3;

/* Font weight — Tailwind: font-normal/medium/semibold/bold */
--font-weight-normal: 400;
--font-weight-medium: 500;
--font-weight-semibold: 600;
--font-weight-bold: 700;
```

### Spacing (density-aware)

**Tailwind 4 권장 방식**: 단일 base 변수(`--spacing`) 정의 → 모든 `p-*`/`m-*`/`gap-*` 유틸리티가 자동으로 배수 생성. 컴팩트 모드에서 이 값 하나만 오버라이드하면 전체 간격이 동시 축소된다.

```css
/* Base spacing unit — 컴팩트 모드에서 축소됨 (Slice 1.2) */
--spacing: 4px;       /* Regular: p-1=4px, p-4=16px, p-6=24px */
/* Compact (Slice 1.2):
   --spacing: 3px;    p-1=3px, p-4=12px, p-6=18px (약 75%) */
```

추가 특수 간격 필요 시 Tailwind의 arbitrary value(`p-[20px]`) 사용 대신 새 커스텀 토큰 선언:
```css
--spacing-gutter: 60px;   /* p-gutter, m-gutter 자동 생성 */
```
단, 현재 스펙 범위에서는 base 변수만 사용한다.

### Radius / Shadow

```css
--radius-sm: 4px;
--radius-md: 6px;
--radius-lg: 10px;
--radius-full: 9999px;

--shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.3);
--shadow-sm: 0 2px 4px rgba(0, 0, 0, 0.4);
--shadow-md: 0 4px 12px rgba(0, 0, 0, 0.5);
```

### Tailwind 연결 (`@theme`)

Tailwind 4는 `@theme` 블록에 선언된 CSS 변수를 **네임스페이스 기반**으로 유틸리티 클래스에 자동 매핑한다:

| CSS 변수 네임스페이스 | 생성되는 유틸리티 |
|-------------------|----------------|
| `--color-*` | `bg-*`, `text-*`, `border-*`, `ring-*`, `fill-*` |
| `--font-*` | `font-*` (family) |
| `--text-*` | `text-*` (size, `--text-*--line-height` 페어링 지원) |
| `--font-weight-*` | `font-*` (weight) |
| `--spacing` | `p-N`, `m-N`, `gap-N`, `w-N`, `h-N` (N × base) |
| `--radius-*` | `rounded-*` |
| `--shadow-*` | `shadow-*` |

`src/app.css`:

```css
@import "tailwindcss";

@theme {
  /* Color */
  --color-surface-0: #0e0e0e;
  --color-surface-1: #171717;
  /* ... */

  /* Typography */
  --font-sans: -apple-system, ...;
  --text-base: 13px;
  --text-base--line-height: 1.5;
  --font-weight-medium: 500;

  /* Spacing (Tailwind 4 base unit 방식) */
  --spacing: 4px;

  /* Radius, Shadow */
  --radius-md: 6px;
  --shadow-sm: 0 2px 4px rgba(0,0,0,0.4);
}
```

사용 예:
- `class="bg-surface-1"` → `background: var(--color-surface-1)`
- `class="text-base"` → `font-size: 13px; line-height: 1.5`
- `class="p-4"` → `padding: calc(4 × var(--spacing)) = 16px` (Regular) / 12px (Compact)
- `class="font-medium"` → `font-weight: 500`
- `class="rounded-md"` → `border-radius: 6px`

## Invariants

- 모든 토큰은 **`@theme` 블록에 선언**되고, 컴팩트 모드는 `[data-density="compact"]`에서 일부 변수만 오버라이드
- Tailwind 4 네임스페이스 규칙 준수: `--color-*`, `--text-*`, `--font-weight-*`, `--spacing`, `--radius-*`, `--shadow-*`
- Svelte 컴포넌트는 **직접 hex 색상을 작성하지 않는다** (`#3b82f6` → `bg-accent`)
- `text-xs` ~ `text-3xl` 클래스는 8단계로 고정 — 이보다 세분화 필요 시 spec 개정
- 컴팩트 모드에서 변화되는 토큰은 `--text-*` (각 사이즈)와 `--spacing` (base 단위)만 (색상/radius는 불변)
- `--spacing` base 방식 사용 — 모든 `p-N`/`m-N`/`gap-N`이 자동으로 스케일링됨

## Behavior Contract — Token Application

| # | Given | When | Then |
|---|-------|------|------|
| 1 | `src/app.css`에 토큰 정의됨 | `class="bg-surface-1"` 사용 | `background-color: var(--color-surface-1)` 적용 |
| 2 | `<html>` 속성 미지정 | 모든 `--text-*` 값 확인 | 기본(Regular) 모드 값 반환 |
| 3 | `<html data-density="compact">` | `--text-base` 확인 | 컴팩트 값(12px) 반환 (Slice 1.2에서 구현) |
| 4 | 컴포넌트에서 `text-primary` | 렌더링 시 | `color: var(--color-text-primary)` (#f5f5f5) |
| 5 | 컴포넌트에서 `rounded-md` | 렌더링 시 | `border-radius: var(--radius-md)` (6px) |

## Behavior Contract — Migration

| # | Given | When | Then |
|---|-------|------|------|
| 6 | 기존 컴포넌트에 `text-white` | 토큰 전환 | `text-primary`로 치환 |
| 7 | 기존 컴포넌트에 `text-[#888]` | 토큰 전환 | `text-muted`로 치환 |
| 8 | 기존 컴포넌트에 `bg-[#1a1a1a]` | 토큰 전환 | `bg-surface-1`로 치환 |
| 9 | 기존 토큰 이름 (`color-surface`, `color-muted`) | 신규 체계로 전환 | 시각 회귀 없이 호환 (매핑 테이블 적용) |
| 10 | `text-sm` (12px, Tailwind 기본) | 토큰 적용 후 | `text-sm` 동일 유지 — Tailwind 기본 스케일과 충돌 없음 |

### 기존 → 신규 토큰 매핑

| 기존 클래스 | 신규 클래스 | 비고 |
|------|------|------|
| `text-white` | `text-fg` | 기본 텍스트 |
| `text-muted` | `text-fg-muted` | 보조 텍스트 (#888 → #8a8a8a 미세 조정) |
| `bg-surface` | `bg-surface-0` | 앱 바탕 |
| `bg-surface-1/2/3` | `bg-surface-1/2/3` | 동일 |
| `border-border` | `border-border` | 동일 (변수명만 `default` 제거) |
| `text-accent`, `bg-accent` | 동일 | - |
| `text-success/warning/danger` | 동일 | - |
| **신규** | `text-fg-dim`, `border-border-strong`, `bg-accent-hover`, `text-info` 등 | - |

기존 legacy 토큰(`--color-surface`, `--color-muted`, `--color-border`)은 마이그레이션 기간 동안 alias로 유지한 후 Slice 1.1 완료 시점에 제거.

## Edge Cases

- **Tailwind 기본 스케일 덮어쓰기**: `@theme` 안의 `--text-sm` 재정의는 Tailwind 기본값을 완전 대체 → 의도된 동작. 단, `--text-md` 같이 Tailwind에 없는 키를 추가하면 새 유틸리티(`text-md`) 생성
- **Arbitrary value 금지**: `p-[16px]` 형태 대신 항상 `p-4` 사용 (컴팩트 모드에서 자동 스케일)
- **CSS 변수 fallback**: `bg-[var(--color-surface-1,_#171717)]` 스타일 fallback은 사용하지 않음 (토큰은 항상 정의됨)
- **다이내믹 스타일**: 링크 그래프(d3-force)는 SVG 내부에서 CSS 변수를 직접 읽을 수 있으므로 `getComputedStyle(document.documentElement).getPropertyValue('--color-accent')` 패턴 사용
- **시각 회귀**: 색상 수치가 미세 조정되므로 전체 페이지 스크린샷 비교는 의미 없음 — 수동으로 각 페이지 육안 확인

## Dependencies

- `src/app.css` — 토큰 선언, `@theme` 블록
- 기존 17개 Svelte 파일 — 하드코딩 제거 대상
- Tailwind 4.2+ (이미 설치됨)
- 자동화 없음 — 치환은 grep 기반 수동 (스펙 12번 참조)

## Mock Boundary

- 디자인 시스템은 **순수 CSS** — 유닛 테스트 범위 외
- 검증: 각 Svelte 페이지를 수동으로 돌면서 시각 확인 (`npm run tauri dev`)
- 회귀 체크리스트는 Slice 1.1 구현 시 함께 정리

## 마이그레이션 절차

1. `src/app.css`에 새 토큰 전체 추가 (`@theme` 블록)
2. 기존 토큰과 신규 토큰 **둘 다 유지** (호환 기간)
3. 각 Svelte 파일에서 하드코딩 값을 grep으로 찾아 토큰 클래스로 치환
   - `#[0-9a-fA-F]{3,8}` → 매핑 테이블 기준 치환
   - `text-white`, `text-[#xxx]` → `text-primary/secondary/muted`
4. 17개 파일 모두 확인 후 기존 토큰 제거
5. 시각 확인: Dashboard, Explore, View, Graph, Claude, Settings, Capture 각 페이지 순회

## Out of Scope (Slice 1.1)

- 컴팩트 모드 실제 전환 로직 — Slice 1.2
- 라이트 테마
- 마크다운 뷰어 스타일(`.markdown-body`) — Slice 1.3
- 사이드바 레이아웃 변경 — Slice 1.4
