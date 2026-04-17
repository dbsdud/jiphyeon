<script lang="ts">
  import { createQuickNote } from "$lib/api";

  let title = $state("");
  let content = $state("");
  let tagsInput = $state("");
  let saving = $state(false);
  let error = $state("");

  async function save() {
    if (!content.trim()) {
      error = "내용을 입력해주세요.";
      return;
    }

    saving = true;
    error = "";

    try {
      const tags = tagsInput
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean);

      await createQuickNote(title.trim() || null, content.trim(), tags);

      // 저장 성공 → 윈도우 닫기
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function cancel() {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      cancel();
    } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      save();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-screen flex flex-col bg-surface p-4">
  <h2 class="text-sm font-semibold text-muted mb-3">Quick Note</h2>

  <input
    type="text"
    placeholder="제목 (선택)"
    class="w-full px-3 py-2 text-sm bg-surface-1 border border-border rounded text-white placeholder:text-muted focus:outline-none focus:border-accent mb-2"
    bind:value={title}
  />

  <textarea
    placeholder="내용을 입력하세요..."
    class="w-full flex-1 px-3 py-2 text-sm bg-surface-1 border border-border rounded text-white placeholder:text-muted focus:outline-none focus:border-accent resize-none mb-2"
    bind:value={content}
  ></textarea>

  <input
    type="text"
    placeholder="태그 (쉼표로 구분)"
    class="w-full px-3 py-2 text-sm bg-surface-1 border border-border rounded text-white placeholder:text-muted focus:outline-none focus:border-accent mb-3"
    bind:value={tagsInput}
  />

  {#if error}
    <p class="text-xs text-danger mb-2">{error}</p>
  {/if}

  <div class="flex justify-end gap-2">
    <button
      class="text-xs px-4 py-1.5 rounded bg-surface-2 border border-border text-muted hover:text-white transition-colors"
      onclick={cancel}
    >
      취소
    </button>
    <button
      class="text-xs px-4 py-1.5 rounded bg-accent text-white hover:bg-accent/80 transition-colors disabled:opacity-50"
      onclick={save}
      disabled={saving}
    >
      {saving ? "저장 중..." : "저장 (⌘+Enter)"}
    </button>
  </div>
</div>
