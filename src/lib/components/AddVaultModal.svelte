<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { connectVault } from "$lib/api";

  interface Props {
    open: boolean;
    onclose: () => void;
    onadded: (path: string, created: boolean) => void;
  }

  const { open: isOpen, onclose, onadded }: Props = $props();

  let busy = $state(false);
  let error = $state("");

  async function pickDirectory(title: string): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false, title });
    return typeof selected === "string" ? selected : null;
  }

  function resetAndClose() {
    error = "";
    busy = false;
    onclose();
  }

  async function handleConnect() {
    error = "";
    const dir = await pickDirectory("연결할 볼트 폴더 선택");
    if (!dir) return;
    busy = true;
    try {
      const status = await connectVault(dir);
      if (status.vault_path) {
        onadded(status.vault_path, false);
        resetAndClose();
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-6"
    role="dialog"
    aria-modal="true"
  >
    <div class="bg-surface-1 border border-border rounded-xl p-6 max-w-md w-full">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-base font-semibold text-fg">볼트 추가</h2>
        <button
          class="text-fg-muted hover:text-fg"
          onclick={resetAndClose}
          disabled={busy}
          aria-label="닫기"
        >
          ✕
        </button>
      </div>

      <button
        class="w-full flex items-start gap-3 p-4 rounded-lg border border-border text-left
               hover:border-accent hover:bg-surface-2 transition-colors
               disabled:opacity-50"
        onclick={handleConnect}
        disabled={busy}
      >
        <span class="text-xl">📂</span>
        <div>
          <div class="text-sm font-medium text-fg">볼트 연결</div>
          <div class="text-xs text-fg-muted mt-1">
            마크다운 파일이 들어있는 디렉토리를 선택합니다.
          </div>
        </div>
      </button>

      {#if error}
        <p class="mt-4 text-xs text-danger">{error}</p>
      {/if}
    </div>
  </div>
{/if}
