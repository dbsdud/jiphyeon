<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createVault, connectVault } from "$lib/api";

  interface Props {
    open: boolean;
    onclose: () => void;
    onadded: (path: string, created: boolean) => void;
  }

  const { open: isOpen, onclose, onadded }: Props = $props();

  type Step = "choose" | "name-vault";

  let step = $state<Step>("choose");
  let parentDir = $state("");
  let vaultName = $state("my-vault");
  let busy = $state(false);
  let error = $state("");

  const targetPath = $derived(
    parentDir && vaultName.trim() ? joinPath(parentDir, vaultName.trim()) : "",
  );
  const nameIsValid = $derived(
    /^[^/\\:*?"<>|]+$/.test(vaultName.trim()) && !vaultName.trim().startsWith("."),
  );

  function joinPath(parent: string, name: string): string {
    return parent.endsWith("/") ? parent + name : parent + "/" + name;
  }

  async function pickDirectory(title: string): Promise<string | null> {
    const selected = await open({ directory: true, multiple: false, title });
    return typeof selected === "string" ? selected : null;
  }

  function resetAndClose() {
    step = "choose";
    parentDir = "";
    vaultName = "my-vault";
    error = "";
    busy = false;
    onclose();
  }

  async function handleCreate() {
    error = "";
    const dir = await pickDirectory("새 볼트를 저장할 부모 폴더 선택");
    if (!dir) return;
    parentDir = dir;
    step = "name-vault";
  }

  async function confirmCreate() {
    if (!nameIsValid || !targetPath) return;
    busy = true;
    error = "";
    try {
      const status = await createVault(targetPath);
      if (status.vault_path) {
        onadded(status.vault_path, true);
        resetAndClose();
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function handleConnect() {
    error = "";
    const dir = await pickDirectory("연결할 기존 볼트 선택");
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
        <h2 class="text-base font-semibold text-fg">
          {step === "choose" ? "볼트 추가" : "새 볼트 이름"}
        </h2>
        <button
          class="text-fg-muted hover:text-fg"
          onclick={resetAndClose}
          disabled={busy}
          aria-label="닫기"
        >
          ✕
        </button>
      </div>

      {#if step === "choose"}
        <div class="space-y-2">
          <button
            class="w-full flex items-start gap-3 p-4 rounded-lg border border-border text-left
                   hover:border-accent hover:bg-surface-2 transition-colors
                   disabled:opacity-50"
            onclick={handleCreate}
            disabled={busy}
          >
            <span class="text-xl">✨</span>
            <div>
              <div class="text-sm font-medium text-fg">새 볼트 생성</div>
              <div class="text-xs text-fg-muted mt-1">
                부모 폴더와 이름을 정해 기본 구조로 초기화합니다.
              </div>
            </div>
          </button>

          <button
            class="w-full flex items-start gap-3 p-4 rounded-lg border border-border text-left
                   hover:border-accent hover:bg-surface-2 transition-colors
                   disabled:opacity-50"
            onclick={handleConnect}
            disabled={busy}
          >
            <span class="text-xl">📂</span>
            <div>
              <div class="text-sm font-medium text-fg">기존 볼트 연결</div>
              <div class="text-xs text-fg-muted mt-1">
                이미 존재하는 볼트 디렉토리를 선택하여 등록합니다.
              </div>
            </div>
          </button>
        </div>
      {:else}
        <div class="text-xs text-fg-muted mb-1">부모 폴더</div>
        <div class="font-mono text-xs text-fg break-all mb-3">{parentDir}</div>

        <label for="add-vault-name" class="text-xs text-fg-muted block mb-1">볼트 이름</label>
        <input
          id="add-vault-name"
          type="text"
          class="w-full bg-surface-0 border border-border rounded px-3 py-2 text-sm text-fg
                 focus:border-accent focus:outline-none"
          bind:value={vaultName}
          disabled={busy}
          onkeydown={(e) => {
            if (e.key === "Enter" && nameIsValid) confirmCreate();
          }}
        />

        {#if !nameIsValid && vaultName.trim().length > 0}
          <p class="text-xs text-danger mt-2">
            이름에 `/ \ : * ? " &lt; &gt; |` 문자 사용 금지. `.`로 시작 금지.
          </p>
        {/if}

        {#if targetPath && nameIsValid}
          <div class="mt-3 text-xs text-fg-muted">
            생성 위치: <span class="font-mono text-fg">{targetPath}</span>
          </div>
        {/if}

        <div class="flex justify-end gap-2 mt-4">
          <button
            class="text-xs px-3 py-1.5 rounded border border-border text-fg-muted
                   hover:text-fg hover:border-accent transition-colors disabled:opacity-50"
            onclick={() => {
              step = "choose";
            }}
            disabled={busy}
          >
            뒤로
          </button>
          <button
            class="text-xs px-3 py-1.5 rounded bg-accent text-fg
                   hover:bg-accent/80 transition-colors
                   disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={confirmCreate}
            disabled={busy || !nameIsValid}
          >
            {busy ? "생성 중..." : "볼트 생성"}
          </button>
        </div>
      {/if}

      {#if error}
        <p class="mt-4 text-xs text-danger">{error}</p>
      {/if}
    </div>
  </div>
{/if}
