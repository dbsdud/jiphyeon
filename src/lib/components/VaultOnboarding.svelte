<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { connectVault } from "$lib/api";

  interface Props {
    onconnected: (path: string, created: boolean) => void;
  }

  const { onconnected }: Props = $props();

  let busy = $state(false);
  let error = $state("");

  async function pickDirectory(title: string): Promise<string | null> {
    const selected = await open({
      directory: true,
      multiple: false,
      title,
    });
    return typeof selected === "string" ? selected : null;
  }

  async function handleConnect() {
    error = "";
    const dir = await pickDirectory("연결할 볼트 폴더 선택");
    if (!dir) return;
    busy = true;
    try {
      const status = await connectVault(dir);
      if (status.vault_path) {
        onconnected(status.vault_path, false);
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center bg-surface-0 px-6">
  <div class="w-full max-w-xl">
    <header class="text-center mb-10">
      <div class="text-5xl mb-3">📓</div>
      <h1 class="text-2xl font-bold text-fg mb-2">집현 시작하기</h1>
      <p class="text-sm text-fg-muted">
        마크다운 볼트 폴더를 선택해 연결하세요
      </p>
    </header>

    <button
      class="group w-full bg-surface-1 border border-border rounded-xl p-5 text-left
             hover:border-accent hover:bg-surface-2 transition-all
             disabled:opacity-50 disabled:cursor-not-allowed"
      onclick={handleConnect}
      disabled={busy}
    >
      <div class="flex items-start gap-4">
        <div class="text-3xl">📂</div>
        <div class="flex-1">
          <h2 class="text-base font-semibold text-fg mb-1">볼트 연결</h2>
          <p class="text-sm text-fg-muted">
            마크다운 파일이 들어있는 디렉토리를 선택하세요. 빈 폴더도 가능합니다.
          </p>
        </div>
      </div>
    </button>

    {#if busy}
      <p class="text-center text-sm text-fg-muted mt-6">처리 중...</p>
    {/if}

    {#if error}
      <div class="mt-6 bg-danger/10 border border-danger/30 rounded-lg p-4 text-sm text-danger">
        {error}
      </div>
    {/if}
  </div>
</div>
