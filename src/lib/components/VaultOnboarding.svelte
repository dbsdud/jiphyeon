<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createVault, connectVault } from "$lib/api";

  interface Props {
    onconnected: (path: string, created: boolean) => void;
  }

  const { onconnected }: Props = $props();

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
    const selected = await open({
      directory: true,
      multiple: false,
      title,
    });
    return typeof selected === "string" ? selected : null;
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
    error = "";
    busy = true;
    try {
      const status = await createVault(targetPath);
      if (status.vault_path) {
        onconnected(status.vault_path, true);
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function cancelCreate() {
    step = "choose";
    parentDir = "";
    error = "";
  }

  async function handleConnect() {
    error = "";
    const dir = await pickDirectory("연결할 기존 볼트 선택");
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
      <h1 class="text-2xl font-bold text-white mb-2">Co-Vault 시작하기</h1>
      <p class="text-sm text-muted">
        {step === "choose"
          ? "마크다운 볼트를 연결하거나 새로 생성하세요"
          : "새 볼트의 이름을 정해주세요"}
      </p>
    </header>

    {#if step === "choose"}
      <div class="grid gap-4">
        <button
          class="group bg-surface-1 border border-border rounded-xl p-5 text-left
                 hover:border-accent hover:bg-surface-2 transition-all
                 disabled:opacity-50 disabled:cursor-not-allowed"
          onclick={handleCreate}
          disabled={busy}
        >
          <div class="flex items-start gap-4">
            <div class="text-3xl">✨</div>
            <div class="flex-1">
              <h2 class="text-base font-semibold text-white mb-1">새 볼트 생성</h2>
              <p class="text-sm text-muted">
                부모 폴더를 선택하고 이름을 정하면, 그 위치에 기본 구조와 템플릿으로 초기화된 볼트가 만들어집니다.
              </p>
            </div>
          </div>
        </button>

        <button
          class="group bg-surface-1 border border-border rounded-xl p-5 text-left
                 hover:border-accent hover:bg-surface-2 transition-all
                 disabled:opacity-50 disabled:cursor-not-allowed"
          onclick={handleConnect}
          disabled={busy}
        >
          <div class="flex items-start gap-4">
            <div class="text-3xl">📂</div>
            <div class="flex-1">
              <h2 class="text-base font-semibold text-white mb-1">기존 볼트 연결</h2>
              <p class="text-sm text-muted">
                이미 존재하는 볼트 디렉토리를 선택하여 연결합니다.
              </p>
            </div>
          </div>
        </button>
      </div>
    {:else}
      <div class="bg-surface-1 border border-border rounded-xl p-5">
        <div class="text-xs text-muted mb-1">부모 폴더</div>
        <div class="font-mono text-sm text-white break-all mb-4">{parentDir}</div>

        <label for="vault-name" class="text-xs text-muted block mb-1">볼트 이름</label>
        <input
          id="vault-name"
          type="text"
          class="w-full bg-surface-0 border border-border rounded px-3 py-2 text-sm text-white
                 focus:border-accent focus:outline-none"
          bind:value={vaultName}
          placeholder="my-vault"
          disabled={busy}
          onkeydown={(e) => {
            if (e.key === "Enter" && nameIsValid) confirmCreate();
          }}
        />

        {#if !nameIsValid && vaultName.trim().length > 0}
          <p class="text-xs text-danger mt-2">
            이름에 `/ \ : * ? " &lt; &gt; |` 문자를 쓸 수 없고, `.`로 시작할 수 없습니다.
          </p>
        {/if}

        {#if targetPath && nameIsValid}
          <div class="mt-3 text-xs text-muted">
            생성 위치:
            <span class="font-mono text-white">{targetPath}</span>
          </div>
        {/if}

        <div class="flex justify-end gap-2 mt-5">
          <button
            class="text-sm px-4 py-2 rounded border border-border text-muted
                   hover:text-white hover:border-accent transition-colors
                   disabled:opacity-50"
            onclick={cancelCreate}
            disabled={busy}
          >
            취소
          </button>
          <button
            class="text-sm px-4 py-2 rounded bg-accent text-white
                   hover:bg-accent/80 transition-colors
                   disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={confirmCreate}
            disabled={busy || !nameIsValid}
          >
            {busy ? "생성 중..." : "볼트 생성"}
          </button>
        </div>
      </div>
    {/if}

    {#if busy && step === "choose"}
      <p class="text-center text-sm text-muted mt-6">처리 중...</p>
    {/if}

    {#if error}
      <div class="mt-6 bg-danger/10 border border-danger/30 rounded-lg p-4 text-sm text-danger">
        {error}
      </div>
    {/if}
  </div>
</div>
