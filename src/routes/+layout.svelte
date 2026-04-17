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
    type VaultStatus,
  } from "$lib/api";
  import type {
    VaultEntry,
    NotificationEvent,
    NotificationLevel,
  } from "$lib/types";

  const { children } = $props();

  let isCapture = $derived((page.url.pathname as string) === "/capture");

  const navItems = [
    { href: "/", label: "Dashboard", icon: "📊" },
    { href: "/explore", label: "Explore", icon: "📁" },
    { href: "/graph", label: "Graph", icon: "🔗" },
    { href: "/claude", label: "Claude", icon: "🤖" },
    { href: "/settings", label: "Settings", icon: "⚙️" },
  ];

  let currentPath = $state("/");
  let clipOpen = $state(false);
  let toastMessage = $state("");
  let toastType = $state<NotificationLevel>("success");
  let toastVisible = $state(false);

  let unlistenNotification: UnlistenFn | null = null;

  onMount(async () => {
    try {
      unlistenNotification = await listen<NotificationEvent>("notification", (e) => {
        toastMessage = e.payload.message;
        toastType = e.payload.level;
        toastVisible = true;
      });
    } catch (err) {
      console.error("notification listener 등록 실패", err);
    }
  });

  onDestroy(() => {
    unlistenNotification?.();
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

  function navigate(href: string) {
    currentPath = href;
  }

  function onClipSuccess(path: string, title: string) {
    toastMessage = `Clipped: ${title}`;
    toastType = "success";
    toastVisible = true;
  }
</script>

{#if isCapture}
  {@render children()}
{:else if vaultStatus === null}
  <div class="min-h-screen flex items-center justify-center text-sm text-muted">
    Loading...
  </div>
{:else if !vaultStatus.connected}
  <VaultOnboarding onconnected={onConnected} />
{:else}
  <div class="flex h-screen overflow-hidden">
    <!-- Sidebar -->
    <nav class="w-52 bg-surface-1 border-r border-border flex flex-col shrink-0">
      <div class="p-4 border-b border-border flex items-center justify-between">
        <h1 class="text-sm font-bold tracking-wide text-white">Co-Vault</h1>
        <button
          class="text-xs px-2 py-1 rounded bg-surface-2 border border-border text-muted hover:text-white hover:border-accent transition-colors"
          onclick={() => { clipOpen = true; }}
          title="Web Clip"
        >
          + Clip
        </button>
      </div>

      <!-- Vaults -->
      <div class="border-b border-border py-2">
        <div class="flex items-center justify-between px-4 mb-1">
          <span class="text-xs font-semibold text-muted uppercase tracking-wide">📓 볼트</span>
          <button
            class="text-sm text-muted hover:text-white w-5 h-5 flex items-center justify-center rounded hover:bg-surface-2"
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
            <div class="group flex items-center px-4 py-1 hover:bg-surface-2 transition-colors">
              <button
                class="flex-1 flex items-center gap-2 min-w-0 text-left text-sm
                  {isActive ? 'text-white font-medium' : 'text-muted hover:text-white'}
                  disabled:opacity-50"
                onclick={() => handleSwitch(vault.path)}
                disabled={vaultActionBusy || isActive}
                title={vault.path}
              >
                <span class="shrink-0 text-xs">{isActive ? '●' : '○'}</span>
                <span class="truncate">{vault.name}</span>
              </button>
              {#if !isActive}
                <button
                  class="ml-1 text-xs text-muted opacity-0 group-hover:opacity-100 hover:text-danger transition-opacity
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
            <div class="px-4 py-1 text-xs text-muted">
              등록된 볼트 없음
            </div>
          {/if}
        </div>
      </div>

      <div class="flex-1 py-2">
        {#each navItems as item}
          <a
            href={item.href}
            class="flex items-center gap-2 px-4 py-2 text-sm transition-colors
              {currentPath === item.href
                ? 'text-white bg-surface-2'
                : 'text-muted hover:text-white hover:bg-surface-2'}"
            onclick={(e) => { navigate(item.href); }}
          >
            <span>{item.icon}</span>
            <span>{item.label}</span>
          </a>
        {/each}
      </div>
      <div class="p-3 border-t border-border text-xs text-muted">
        v0.5.0
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
