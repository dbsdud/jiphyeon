# Spec: sidebar-layout (v1.0 Epic 1.4 사이드바 재구성)

## 전제

- 앱 이름 통일(package/Cargo/tauri.conf)은 Slice 1.2에서 완료됨 — 본 Slice는 **네비게이션 구조 재편**만 다룸
- 사이드바 상단 표기는 한글 `집현` 유지 (영문 식별자와 분리 — UI 친화성 우선)
- 현재 네비게이션은 `Dashboard / Explore / Graph / Claude / Settings` 5개 플랫 구조
- `+ Clip` 버튼이 사이드바 최상단 오른쪽에 고정되어 시각적 무게가 큼
- Svelte 컴포넌트 테스트 프레임워크 미도입 → **수동 검증** 기반

## Public Interface

본 Slice는 UI 레이아웃 변경 전용 — Rust 공개 API 및 TypeScript 타입 변경 없음.

### 변경 대상 파일

- `src/routes/+layout.svelte` — 네비게이션 그룹화 + Clip 메뉴화 + Transcribe placeholder + 버전 표기
- 그 외 Rust/TS 변경 없음

### 네비게이션 데이터 구조 (layout 내부)

```ts
type NavItem = {
  href: string;
  label: string;
  icon: string;
  disabled?: boolean;
  disabledReason?: string;  // tooltip
  action?: "clip";           // 특정 핸들러 트리거 (href 대신)
};

type NavGroup = {
  title: string;             // "탐색" | "작업" | "설정"
  items: NavItem[];
};

const navGroups: NavGroup[] = [
  {
    title: "탐색",
    items: [
      { href: "/",        label: "Dashboard", icon: "📊" },
      { href: "/explore", label: "Explore",   icon: "📁" },
      { href: "/graph",   label: "Graph",     icon: "🔗" },
    ],
  },
  {
    title: "작업",
    items: [
      { href: "#", label: "Clip",       icon: "✂️", action: "clip" },
      { href: "#", label: "Transcribe", icon: "🎙️",
        disabled: true, disabledReason: "Epic 3에서 제공 예정" },
    ],
  },
  {
    title: "설정",
    items: [
      { href: "/claude",   label: "Claude",   icon: "🤖" },
      { href: "/settings", label: "Settings", icon: "⚙️" },
    ],
  },
];
```

### 레이아웃 구조

```
[집현]                               # 로고 영역 — + Clip 버튼 제거
──────────────────
📓 볼트 (기존 그대로)
──────────────────
탐색                                  # 그룹 라벨: text-xs uppercase muted
  📊 Dashboard
  📁 Explore
  🔗 Graph
──────────────────
작업
  ✂️ Clip                             # 클릭 시 clipOpen = true
  🎙️ Transcribe                       # disabled, tooltip
──────────────────
설정
  🤖 Claude
  ⚙️ Settings
──────────────────
                          v0.6.0     # footer 버전 표기 업데이트
```

## Invariants

- 볼트 섹션은 구조/동작 모두 **불변** (기존 코드 그대로 유지)
- 그룹 순서는 `탐색 → 작업 → 설정` 고정 (재구성은 이 순서 기준 사용자 학습)
- 그룹 라벨 스타일은 볼트 섹션의 `📓 볼트` 라벨과 동일 클래스 재사용 (`text-xs font-semibold text-fg-muted uppercase tracking-wide`)
- Clip은 페이지가 아닌 **모달 액션** — href를 가짜값(`#`)으로 두지 않고 `<button>`으로 렌더
- Transcribe는 **disabled 상태** — 클릭 불가, 시각적으로 muted, `title` 속성으로 안내 문구
- 활성 페이지 표시 스타일은 기존과 동일(`text-fg bg-surface-2`) — 각 그룹 내 `href` 아이템에만 적용
- `+ Clip` 버튼은 사이드바 헤더에서 **완전 제거** — 작업 그룹 메뉴가 단일 진입점
- 버전 표기는 `v0.6.0` (Slice 1.5까지 유지 후 v0.6 릴리스 시점에 확정)

## Behavior Contract — 레이아웃 (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 앱 시작, 볼트 연결됨 | 사이드바 렌더 | 상단: 집현 로고만(+Clip 버튼 없음), 볼트 섹션, 그 아래 3그룹(탐색/작업/설정), 하단 v0.6.0 |
| 2 | 사이드바 | 탐색 그룹 라벨 시각 확인 | `text-xs uppercase muted tracking-wide` 스타일로 "탐색"/"작업"/"설정" 표기 |
| 3 | `/` 경로에서 사이드바 렌더 | Dashboard 항목 스타일 확인 | 활성 스타일(`text-fg bg-surface-2`) 적용 |
| 4 | `/explore` 경로 | Explore 항목 | 활성 스타일 적용, 다른 탐색 항목은 muted |
| 5 | `/claude` 경로 | Claude 항목(설정 그룹) | 활성 스타일 적용 |
| 6 | 사이드바의 Clip 메뉴 클릭 | 클릭 | `WebClipDialog` 오픈(`clipOpen = true`) — 페이지 이동 없음 |
| 7 | 사이드바의 Transcribe 항목 호버 | hover | tooltip으로 "Epic 3에서 제공 예정" 노출, cursor not-allowed, 클릭해도 반응 없음 |
| 8 | 컴팩트 모드 활성 | 사이드바 렌더 | 그룹 라벨/항목 간격이 Regular 대비 축소 (Slice 1.2 토큰 자동 반영) |
| 9 | 다수 볼트 목록 | 사이드바 스크롤 | 볼트 섹션이 지나치게 길어져도 그룹 섹션은 고정 위치를 유지 (flex layout) |

## Behavior Contract — 상호작용

| # | Given | When | Then |
|---|-------|------|------|
| 10 | Clip 메뉴 아이템 | 키보드 포커스 후 Enter | `clipOpen = true` 동작 (마우스 클릭과 동일) |
| 11 | Transcribe 메뉴 아이템 | 키보드 포커스 | focus 시 disabled 상태 표시, 포커스는 다음 항목으로 건너뛰지 않음(접근성) |
| 12 | 볼트 전환 이벤트 | 사이드바 | 3그룹 구조는 유지, 활성 페이지 하이라이트만 보존 |
| 13 | 페이지 이동(`<a>` 클릭) | 내부 라우팅 | 기존 `navigate(href)` + SvelteKit 라우팅 동작 그대로 |

## Edge Cases

- **Transcribe disabled 시 접근성**: `<button disabled>` 또는 `aria-disabled="true"` + `tabindex="-1"` 중 선택 → **`aria-disabled` 패턴** 채택(버튼 대신 `<div role="menuitem">`처럼 다루지 않고, 기존 `<a>` 스타일 톤을 유지하되 포인터 이벤트 차단). 간단히 `<button disabled>`로 구현하고 시각 스타일만 opacity/cursor로 표현
- **컴팩트 모드 교차**: 그룹 구분선 여백이 컴팩트에서 너무 빡빡할 수 있음 → `border-t + py-2` 기본값으로 Slice 1.2와 동일 비례 축소. 필요 시 Slice 1.4 수동 검증에서 조정
- **긴 볼트 목록**: 볼트 섹션이 화면을 넘어가면 3그룹이 아래로 밀려 접근성이 떨어질 가능성 → 기존 코드에 스크롤 처리 없음. 본 Slice 범위 외(별도 이슈)
- **v0.5.0 표기 → v0.6.0**: package.json/Cargo.toml 버전은 **별도 태스크**(`#12 chore: v0.6.0 버전 표기`)에서 처리. 사이드바 문자열만 본 Slice에서 선반영 (문자열 상수이므로 UI 일관성 우선)

## Dependencies

- 기존 `+layout.svelte` 상태(`clipOpen`, `currentPath`, `navigate` 등) 그대로 활용
- Tailwind 4 토큰 (Slice 1.1/1.2 산출물) — 추가 토큰 불필요
- 아이콘: 이모지 그대로 (Slice 1.1에서 아이콘 시스템 미도입 결정)
- `WebClipDialog` 컴포넌트 — 기존 그대로

## Mock Boundary

- 본 Slice는 Rust 변경 없음 → Rust 테스트 추가 없음
- 프론트 수동 검증 스크립트:
  1. `npm run tauri dev` 실행
  2. 볼트 연결된 상태에서 사이드바 확인 (그룹 3개, 각 항목 아이콘/라벨)
  3. 각 페이지 이동 후 활성 상태 확인
  4. Clip 메뉴 클릭 → 모달 오픈 확인
  5. Transcribe 호버 → disabled + tooltip 확인
  6. 컴팩트 모드 토글 → 사이드바 간격 축소 확인
  7. 볼트 추가/전환 → 기존 동작 그대로 확인

## Out of Scope (Slice 1.4)

- 사이드바 접기/펴기 토글
- 볼트 섹션 정렬/필터
- Transcribe 실제 연결 (Epic 3)
- 키보드 단축키(Cmd+K 팔레트 등)
- 사이드바 커스터마이즈(항목 숨기기/순서 변경)
- package.json/Cargo.toml 버전 bump (별도 태스크 #12)
- 아이콘 시스템(이모지 → SVG) 교체
- 그룹 축소(collapsible section)

## 열린 결정 (진입 전 확인 필요)

- **로고 라벨**: `집현`(현재) vs `Jiphyeon`(플랜 문서 예시) → **`집현` 유지** 제안 (영문 식별자와 분리, 한글 브랜딩). 변경 원하면 알려 주세요.
- **Transcribe 아이콘**: `🎙️` vs `🎧` vs 비활성 표시용 `⏳` → **`🎙️` 유지** 제안 (기능 의미 명확).
- **버전 문자열 위치 확정**: footer 유지 vs 로고 옆 옮기기 → **footer 유지** 제안 (현재 UX와의 일관성).
