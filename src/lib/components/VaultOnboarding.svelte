<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { registerProject } from "$lib/api";

  interface Props {
    onconnected: (path: string, created: boolean) => void;
  }

  let { onconnected }: Props = $props();

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
      // Slice B-2에서 docs/ 미존재 다이얼로그를 추가하기 전 임시: 무조건 create_docs=true.
      const entry = await registerProject(picked, null, true);
      onconnected(entry.root_path, false);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="min-h-screen flex items-center justify-center p-8">
  <div class="bg-surface-1 border border-border rounded-xl max-w-md w-full p-8 text-center">
    <h1 class="text-2xl font-bold mb-3">집현 v2.0</h1>
    <p class="text-sm text-fg-muted mb-6">
      프로젝트(레포) 폴더를 등록하면 집현이 해당 프로젝트의 docs/ 와 graphify-out/ 을 데이터 소스로 사용합니다.
    </p>
    <button
      class="px-5 py-2 rounded bg-accent text-accent-fg hover:bg-accent/80 transition-colors disabled:opacity-50"
      onclick={selectAndRegister}
      disabled={busy}
    >
      {busy ? "등록 중..." : "프로젝트 폴더 선택"}
    </button>
    {#if error}
      <p class="text-xs text-danger mt-3 break-all">{error}</p>
    {/if}
    <p class="text-xs text-fg-muted/70 mt-6">
      Slice B-2 에서 docs/ 자동 감지·생성 다이얼로그가 추가될 예정입니다.
    </p>
  </div>
</div>
