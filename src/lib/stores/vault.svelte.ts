/**
 * 볼트 변경 신호를 프론트 전역으로 전파하는 reactive store.
 *
 * 사용 흐름:
 * 1. Rust watcher가 파일 변경을 감지 → "vault-changed" 이벤트 emit
 * 2. +layout.svelte가 구독 → `rescan_vault` IPC로 백엔드 인덱스 갱신
 * 3. `vaultRefresh.bump()` 호출 → version 증가
 * 4. 각 페이지(Dashboard/Explore 등)의 $effect가 version을 track하여 자동 재로드
 */
class VaultRefreshStore {
  version = $state(0);

  bump(): void {
    this.version += 1;
  }
}

export const vaultRefresh = new VaultRefreshStore();
