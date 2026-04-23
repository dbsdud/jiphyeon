<script lang="ts">
  import "../app.css";
  import { onMount, onDestroy } from "svelte";
  import { page } from "$app/state";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import WebClipDialog from "$lib/components/WebClipDialog.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import VaultOnboarding from "$lib/components/VaultOnboarding.svelte";
  import GitInitModal from "$lib/components/GitInitModal.svelte";
  import AddVaultModal from "$lib/components/AddVaultModal.svelte";
  import {
    getVaultStatus,
    listVaults,
    switchVault,
    removeVault,
    getConfig,
    updateConfig,
    rescanVault,
    type VaultStatus,
  } from "$lib/api";
  import type {
    VaultEntry,
    NotificationEvent,
    NotificationLevel,
  } from "$lib/types";
  import { vaultRefresh } from "$lib/stores/vault.svelte";
  import {
    applyTheme,
    watchSystemTheme,
    themeRefresh,
    themePref,
    sidebarStore,
  } from "$lib/stores/theme.svelte";

  const { children } = $props();

  let isCapture = $derived((page.url.pathname as string) === "/capture");

  type NavItem = {
    label: string;
    icon: string;
    href?: string;
    action?: "clip";
    disabled?: boolean;
    disabledReason?: string;
  };

  type NavGroup = {
    title: string;
    items: NavItem[];
  };

  const navGroups: NavGroup[] = [
    {
      title: "탐색",
      items: [
        { href: "/", label: "Dashboard", icon: "📊" },
        { href: "/explore", label: "Explore", icon: "📁" },
        { href: "/graph", label: "Graph", icon: "🔗" },
      ],
    },
    {
      title: "작업",
      items: [
        { action: "clip", label: "Clip", icon: "✂️" },
        { href: "/transcribe", label: "Transcribe", icon: "🎙️" },
      ],
    },
    {
      title: "설정",
      items: [
        { href: "/settings", label: "Settings", icon: "⚙️" },
      ],
    },
  ];

  let currentPath = $derived(page.url.pathname as string);
  let clipOpen = $state(false);
  let toastMessage = $state("");
  let toastType = $state<NotificationLevel>("success");
  let toastVisible = $state(false);

  let unlistenNotification: UnlistenFn | null = null;
  let unlistenVaultChanged: UnlistenFn | null = null;
  let unwatchSystemTheme: (() => void) | null = null;
  let rescanTimer: ReturnType<typeof setTimeout> | null = null;

  // 볼트 변경 이벤트는 편집 중 여러 번 올 수 있으므로 프론트에서도 debounce.
  function scheduleRescan() {
    if (rescanTimer) clearTimeout(rescanTimer);
    rescanTimer = setTimeout(async () => {
      rescanTimer = null;
      try {
        await rescanVault();
        vaultRefresh.bump();
      } catch (err) {
        console.warn("rescan_vault 실패", err);
      }
    }, 300);
  }

  onMount(async () => {
    try {
      const cfg = await getConfig();
      document.documentElement.dataset.density = cfg.density;
      themePref.set(cfg.theme);
      applyTheme(cfg.theme);
      sidebarStore.set(cfg.sidebar_collapsed);
      // System 선호일 때만 OS 변경에 반응 (store에서 매번 최신 선호 읽음)
      unwatchSystemTheme = watchSystemTheme((resolved) => {
        if (themePref.value === "system") {
          document.documentElement.dataset.theme = resolved;
          themeRefresh.bump();
        }
      });
    } catch (err) {
      console.error("초기 설정 로드 실패", err);
    }

    try {
      unlistenNotification = await listen<NotificationEvent>("notification", (e) => {
        toastMessage = e.payload.message;
        toastType = e.payload.level;
        toastVisible = true;
      });
    } catch (err) {
      console.error("notification listener 등록 실패", err);
    }

    try {
      unlistenVaultChanged = await listen("vault-changed", () => {
        scheduleRescan();
      });
    } catch (err) {
      console.error("vault-changed listener 등록 실패", err);
    }
  });

  onDestroy(() => {
    unlistenNotification?.();
    unlistenVaultChanged?.();
    unwatchSystemTheme?.();
    if (rescanTimer) clearTimeout(rescanTimer);
  });

  let vaultStatus = $state<VaultStatus | null>(null);
  let vaults = $state<VaultEntry[]>([]);
  let gitModalOpen = $state(false);
  let gitModalPath = $state("");
  let addVaultOpen = $state(false);
  let vaultActionBusy = $state(false);

  async function loadVaultStatus() {
    try {
      vaultStatus = await getVaultStatus();
    } catch (e) {
      console.error("get_vault_status failed", e);
      vaultStatus = { connected: false, vault_path: null };
    }
  }

  async function loadVaults() {
    try {
      vaults = await listVaults();
    } catch (e) {
      console.error("list_vaults failed", e);
      vaults = [];
    }
  }

  loadVaultStatus();
  loadVaults();

  function onConnected(path: string, created: boolean) {
    vaultStatus = { connected: true, vault_path: path };
    if (created) {
      gitModalPath = path;
      gitModalOpen = true;
    }
    loadVaults();
  }

  async function onVaultAdded(path: string, created: boolean) {
    // 새 볼트가 활성이 되므로 전체 상태 재로드
    if (created) {
      gitModalPath = path;
      gitModalOpen = true;
    }
    window.location.reload();
  }

  async function handleSwitch(path: string) {
    if (vaultActionBusy) return;
    if (vaultStatus?.vault_path === path) return;
    vaultActionBusy = true;
    try {
      await switchVault(path);
      window.location.reload();
    } catch (e) {
      toastMessage = `볼트 전환 실패: ${e}`;
      toastType = "error";
      toastVisible = true;
      vaultActionBusy = false;
    }
  }

  async function handleRemove(path: string, name: string, ev: MouseEvent) {
    ev.stopPropagation();
    if (vaultActionBusy) return;
    if (!confirm(`"${name}"를 목록에서 제거하시겠습니까?\n(실제 파일/폴더는 유지됩니다)`)) return;
    vaultActionBusy = true;
    try {
      vaults = await removeVault(path);
    } catch (e) {
      toastMessage = `제거 실패: ${e}`;
      toastType = "error";
      toastVisible = true;
    } finally {
      vaultActionBusy = false;
    }
  }

  function onClipSuccess(path: string, title: string) {
    toastMessage = `Clipped: ${title}`;
    toastType = "success";
    toastVisible = true;
  }

  // Slice 1.6 — 사이드바 토글. 즉시 UI 반영 + AppConfig persist.
  async function toggleSidebar() {
    sidebarStore.toggle(); // dataset + localStorage 동기화
    try {
      await updateConfig({ sidebar_collapsed: sidebarStore.collapsed });
    } catch (err) {
      console.warn("sidebar_collapsed persist 실패", err);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    // Cmd/Ctrl + B: 사이드바 토글. input/textarea 포커스 중에는 무시.
    if (!(e.key === "b" || e.key === "B")) return;
    if (!(e.metaKey || e.ctrlKey)) return;
    const target = e.target as HTMLElement | null;
    const tag = target?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || target?.isContentEditable) return;
    e.preventDefault();
    toggleSidebar();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if isCapture}
  {@render children()}
{:else if vaultStatus === null}
  <div class="min-h-screen flex items-center justify-center text-sm text-fg-muted">
    Loading...
  </div>
{:else if !vaultStatus.connected}
  <VaultOnboarding onconnected={onConnected} />
{:else}
  <div class="flex h-screen overflow-hidden">
    <!-- Sidebar -->
    <nav class="sidebar-root w-52 bg-surface-1 border-r border-border flex flex-col shrink-0">
      <div class="p-4 border-b border-border flex items-center justify-between">
        <h1 class="sidebar-logo text-sm font-bold tracking-wide text-fg">집현</h1>
        <button
          class="text-fg-muted hover:text-fg w-6 h-6 flex items-center justify-center rounded hover:bg-surface-2 transition-colors"
          onclick={toggleSidebar}
          title={sidebarStore.collapsed ? "사이드바 펼치기 (⌘/Ctrl+B)" : "사이드바 접기 (⌘/Ctrl+B)"}
          aria-label="사이드바 토글"
        >
          {sidebarStore.collapsed ? "›" : "‹"}
        </button>
      </div>

      <!-- Vaults -->
      <div class="border-b border-border py-2">
        <div class="sidebar-group-header flex items-center justify-between px-4 mb-1">
          <span class="sidebar-group-title text-xs font-semibold text-fg-muted uppercase tracking-wide">📓 볼트</span>
          <button
            class="sidebar-vault-add text-sm text-fg-muted hover:text-fg w-5 h-5 flex items-center justify-center rounded hover:bg-surface-2"
            onclick={() => { addVaultOpen = true; }}
            title="볼트 추가"
            aria-label="볼트 추가"
          >
            +
          </button>
        </div>
        <div>
          {#each vaults as vault}
            {@const isActive = vaultStatus?.vault_path === vault.path}
            <div class="sidebar-vault-row group flex items-center px-4 py-1 hover:bg-surface-2 transition-colors"
                 class:hidden={sidebarStore.collapsed && !isActive}>
              <button
                class="flex-1 flex items-center gap-2 min-w-0 text-left text-sm
                  {isActive ? 'text-fg font-medium' : 'text-fg-muted hover:text-fg'}
                  disabled:opacity-50"
                onclick={() => handleSwitch(vault.path)}
                disabled={vaultActionBusy || isActive}
                title={vault.name}
              >
                <span class="shrink-0 text-xs">{isActive ? '●' : '○'}</span>
                <span class="sidebar-vault-name truncate">{vault.name}</span>
              </button>
              {#if !isActive}
                <button
                  class="sidebar-vault-remove ml-1 text-xs text-fg-muted opacity-0 group-hover:opacity-100 hover:text-danger transition-opacity
                         disabled:opacity-30"
                  onclick={(e) => handleRemove(vault.path, vault.name, e)}
                  disabled={vaultActionBusy}
                  title="목록에서 제거"
                  aria-label="제거"
                >
                  ✕
                </button>
              {/if}
            </div>
          {/each}
          {#if vaults.length === 0}
            <div class="sidebar-label px-4 py-1 text-xs text-fg-muted">
              등록된 볼트 없음
            </div>
          {/if}
        </div>
      </div>

      <div class="flex-1 overflow-y-auto">
        {#each navGroups as group}
          <div class="py-2 border-b border-border last:border-b-0">
            <div class="sidebar-group-title px-4 mb-1 text-xs font-semibold text-fg-muted uppercase tracking-wide">
              {group.title}
            </div>
            {#each group.items as item}
              {#if item.action === "clip"}
                <button
                  type="button"
                  class="sidebar-item w-full flex items-center gap-2 px-4 py-2 text-sm text-left transition-colors text-fg-muted hover:text-fg hover:bg-surface-2"
                  onclick={() => { clipOpen = true; }}
                  title={item.label}
                >
                  <span>{item.icon}</span>
                  <span class="sidebar-label">{item.label}</span>
                </button>
              {:else if item.disabled}
                <button
                  type="button"
                  class="sidebar-item w-full flex items-center gap-2 px-4 py-2 text-sm text-left text-fg-muted opacity-50 cursor-not-allowed"
                  disabled
                  aria-disabled="true"
                  title={item.disabledReason ?? item.label}
                >
                  <span>{item.icon}</span>
                  <span class="sidebar-label">{item.label}</span>
                </button>
              {:else if item.href}
                <a
                  href={item.href}
                  class="sidebar-item flex items-center gap-2 px-4 py-2 text-sm transition-colors
                    {currentPath === item.href
                      ? 'text-fg bg-surface-2'
                      : 'text-fg-muted hover:text-fg hover:bg-surface-2'}"
                  title={item.label}
                >
                  <span>{item.icon}</span>
                  <span class="sidebar-label">{item.label}</span>
                </a>
              {/if}
            {/each}
          </div>
        {/each}
      </div>
      <div class="sidebar-version p-3 border-t border-border text-xs text-fg-muted">
        v0.9.0
      </div>
    </nav>

    <!-- Main content -->
    <main class="flex-1 overflow-y-auto">
      {@render children()}
    </main>
  </div>

  <!-- Global overlays -->
  <WebClipDialog
    open={clipOpen}
    onclose={() => { clipOpen = false; }}
    onsuccess={onClipSuccess}
  />
  <Toast
    message={toastMessage}
    type={toastType}
    visible={toastVisible}
    onclose={() => { toastVisible = false; }}
  />
  <GitInitModal
    open={gitModalOpen}
    vaultPath={gitModalPath}
    onclose={() => { gitModalOpen = false; }}
  />
  <AddVaultModal
    open={addVaultOpen}
    onclose={() => { addVaultOpen = false; }}
    onadded={onVaultAdded}
  />
{/if}
