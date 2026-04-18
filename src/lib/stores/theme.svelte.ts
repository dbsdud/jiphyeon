/**
 * Slice 1.5 — 테마 스위칭 store + 유틸.
 * spec: docs/specs/spec-theme-switcher.md
 *
 * - `resolveTheme(pref)`: pref("light"|"dark"|"system") → resolved("light"|"dark")
 * - `applyTheme(resolved)`: `<html data-theme>` 세팅 + localStorage 동기화
 * - `watchSystemTheme(cb)`: prefers-color-scheme change 구독
 * - `themeRefresh`: 테마 변경 시 bump → view 페이지의 $effect가 트래킹하여 재렌더
 */
import type { ThemePreference, ResolvedTheme } from "$lib/types";

const STORAGE_KEY = "theme-pref";

export function resolveTheme(pref: ThemePreference): ResolvedTheme {
  if (pref === "light") return "light";
  if (pref === "dark") return "dark";
  if (typeof window === "undefined") return "dark";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

export function applyTheme(pref: ThemePreference): ResolvedTheme {
  const resolved = resolveTheme(pref);
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = resolved;
  }
  if (typeof localStorage !== "undefined") {
    try {
      localStorage.setItem(STORAGE_KEY, pref);
    } catch {
      /* 비활성 환경 무시 */
    }
  }
  return resolved;
}

export function watchSystemTheme(
  onChange: (resolved: ResolvedTheme) => void,
): () => void {
  if (typeof window === "undefined") return () => {};
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = (e: MediaQueryListEvent) =>
    onChange(e.matches ? "dark" : "light");
  mq.addEventListener("change", handler);
  return () => mq.removeEventListener("change", handler);
}

class ThemeRefreshStore {
  version = $state(0);

  bump(): void {
    this.version += 1;
  }
}

export const themeRefresh = new ThemeRefreshStore();

/** 현재 테마 선호(Light/Dark/System). 여러 컴포넌트가 공유하기 위해 모듈 스코프에 둔다. */
class ThemePrefStore {
  value = $state<ThemePreference>("system");

  set(pref: ThemePreference): void {
    this.value = pref;
  }
}

export const themePref = new ThemePrefStore();
