<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { registerProject } from "$lib/api";

  interface Props {
    open: boolean;
    onclose: () => void;
    onadded: (path: string, created: boolean) => void;
  }

  let { open, onclose, onadded }: Props = $props();

  let busy = $state(false);
  let error = $state("");

  async function selectAndRegister(): Promise<void> {
    busy = true;
    error = "";
    try {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked !== "string") {
        busy = false;
        return;
      }
      const entry = await registerProject(picked, null, true);
      onadded(entry.root_path, false);
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if open}
  <div class="fixed inset-0 bg-black/40 z-40 flex items-center justify-center p-6">
    <div class="bg-surface-1 border border-border rounded-xl w-full max-w-md p-6">
      <h3 class="text-lg font-semibold mb-3">프로젝트 추가</h3>
      <p class="text-sm text-fg-muted mb-4">
        등록할 프로젝트 폴더를 선택하세요. docs/ 가 없으면 자동 생성됩니다 (B-2 에서 다이얼로그로 분리 예정).
      </p>
      <div class="flex gap-2 justify-end">
        <button
          class="px-3 py-1.5 text-sm rounded border border-border text-fg-muted hover:text-fg"
          onclick={onclose}
          disabled={busy}
        >
          취소
        </button>
        <button
          class="px-3 py-1.5 text-sm rounded bg-accent text-accent-fg hover:bg-accent/80 disabled:opacity-50"
          onclick={selectAndRegister}
          disabled={busy}
        >
          {busy ? "등록 중..." : "폴더 선택"}
        </button>
      </div>
      {#if error}
        <p class="text-xs text-danger mt-3 break-all">{error}</p>
      {/if}
    </div>
  </div>
{/if}
