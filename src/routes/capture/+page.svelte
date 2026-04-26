<script lang="ts">
  import { onMount } from "svelte";
  import {
    clipUrl,
    createQuickNote,
    getActiveProject,
    listProjects,
  } from "$lib/api";
  import type { ProjectEntry } from "$lib/types";

  type Tab = "note" | "url" | "record";
  let tab = $state<Tab>("note");

  let projects = $state<ProjectEntry[]>([]);
  let activeId = $state<string>("");
  let selectedProjectId = $state<string>("");
  let loading = $state(true);

  // note tab fields
  let title = $state("");
  let content = $state("");
  let tagsInput = $state("");
  let saving = $state(false);
  let error = $state("");

  // url tab fields
  let url = $state("");
  let urlTagsInput = $state("");

  onMount(async () => {
    try {
      const [list, active] = await Promise.all([
        listProjects(),
        getActiveProject(),
      ]);
      projects = list;
      activeId = active?.id ?? "";
      selectedProjectId = activeId || (list[0]?.id ?? "");
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  async function closeWindow(): Promise<void> {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().close();
  }

  async function saveNote(): Promise<void> {
    if (!content.trim()) {
      error = "내용을 입력해주세요.";
      return;
    }
    if (!selectedProjectId) {
      error = "프로젝트를 선택해주세요.";
      return;
    }
    saving = true;
    error = "";
    try {
      const tags = tagsInput.split(",").map((t) => t.trim()).filter(Boolean);
      await createQuickNote(
        title.trim() || null,
        content.trim(),
        tags,
        selectedProjectId,
      );
      await closeWindow();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function clipUrlNow(): Promise<void> {
    if (!url.trim()) {
      error = "URL 을 입력해주세요.";
      return;
    }
    if (!selectedProjectId) {
      error = "프로젝트를 선택해주세요.";
      return;
    }
    saving = true;
    error = "";
    try {
      const tags = urlTagsInput.split(",").map((t) => t.trim()).filter(Boolean);
      const result = await clipUrl(
        { url: url.trim(), tags: tags.length > 0 ? tags : undefined },
        selectedProjectId,
      );
      if (result.success) {
        await closeWindow();
      } else {
        error = result.error ?? "클리핑 실패";
      }
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      closeWindow();
    } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      if (tab === "note") saveNote();
      else if (tab === "url") clipUrlNow();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-screen flex flex-col bg-surface-0 p-4 gap-3">
  {#if loading}
    <p class="text-xs text-fg-muted">Loading...</p>
  {:else if projects.length === 0}
    <div class="m-auto text-center max-w-xs">
      <p class="text-sm text-fg mb-2">등록된 프로젝트가 없습니다.</p>
      <p class="text-xs text-fg-muted">메인 윈도우에서 프로젝트를 먼저 등록하세요.</p>
    </div>
  {:else}
    <!-- Header: 프로젝트 셀렉터 -->
    <div class="flex items-center gap-2 shrink-0">
      <span class="text-xs text-fg-muted">📁</span>
      <select
        bind:value={selectedProjectId}
        class="flex-1 px-2 py-1.5 text-sm bg-surface-1 border border-border rounded text-fg focus:outline-none focus:border-accent"
      >
        {#each projects as p}
          <option value={p.id}>{p.name}{p.id === activeId ? "  (활성)" : ""}</option>
        {/each}
      </select>
    </div>

    <!-- Tabs -->
    <div class="flex rounded border border-border overflow-hidden shrink-0">
      <button
        class="flex-1 px-3 py-1.5 text-xs {tab === 'note' ? 'bg-accent text-accent-fg' : 'text-fg-muted hover:bg-surface-2'}"
        onclick={() => { tab = "note"; }}
      >
        📝 노트
      </button>
      <button
        class="flex-1 px-3 py-1.5 text-xs {tab === 'url' ? 'bg-accent text-accent-fg' : 'text-fg-muted hover:bg-surface-2'}"
        onclick={() => { tab = "url"; }}
      >
        🔗 URL
      </button>
      <button
        class="flex-1 px-3 py-1.5 text-xs {tab === 'record' ? 'bg-accent text-accent-fg' : 'text-fg-muted hover:bg-surface-2'}"
        onclick={() => { tab = "record"; }}
      >
        🎙️ 녹음
      </button>
    </div>

    <!-- Tab body -->
    <div class="flex-1 flex flex-col gap-2 min-h-0">
      {#if tab === "note"}
        <input
          type="text"
          placeholder="제목 (선택)"
          class="w-full px-3 py-2 text-sm bg-surface-1 border border-border rounded text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent shrink-0"
          bind:value={title}
        />
        <textarea
          placeholder="내용을 입력하세요..."
          class="w-full flex-1 px-3 py-2 text-sm bg-surface-1 border border-border rounded text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent resize-none min-h-0"
          bind:value={content}
        ></textarea>
        <input
          type="text"
          placeholder="태그 (쉼표로 구분)"
          class="w-full px-3 py-2 text-sm bg-surface-1 border border-border rounded text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent shrink-0"
          bind:value={tagsInput}
        />
      {:else if tab === "url"}
        <input
          type="url"
          placeholder="https://..."
          class="w-full px-3 py-2 text-sm bg-surface-1 border border-border rounded text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent shrink-0"
          bind:value={url}
        />
        <input
          type="text"
          placeholder="태그 (쉼표로 구분)"
          class="w-full px-3 py-2 text-sm bg-surface-1 border border-border rounded text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent shrink-0"
          bind:value={urlTagsInput}
        />
        <p class="text-xs text-fg-muted shrink-0">
          저장 위치: <code class="px-1 py-0.5 rounded bg-surface-2">docs/clippings/</code>
        </p>
        <div class="flex-1"></div>
      {:else}
        <div class="flex-1 flex items-center justify-center text-center px-4">
          <div>
            <p class="text-sm text-fg mb-2">녹음 탭</p>
            <p class="text-xs text-fg-muted">Slice D-3 에서 추가 예정.</p>
          </div>
        </div>
      {/if}
    </div>

    {#if error}
      <p class="text-xs text-danger shrink-0">{error}</p>
    {/if}

    <!-- Actions -->
    <div class="flex justify-end gap-2 shrink-0">
      <button
        class="text-xs px-4 py-1.5 rounded bg-surface-2 border border-border text-fg-muted hover:text-fg transition-colors"
        onclick={closeWindow}
      >
        취소
      </button>
      {#if tab === "note"}
        <button
          class="text-xs px-4 py-1.5 rounded bg-accent text-accent-fg hover:bg-accent/80 transition-colors disabled:opacity-50"
          onclick={saveNote}
          disabled={saving || !selectedProjectId}
        >
          {saving ? "저장 중..." : "저장 (⌘+Enter)"}
        </button>
      {:else if tab === "url"}
        <button
          class="text-xs px-4 py-1.5 rounded bg-accent text-accent-fg hover:bg-accent/80 transition-colors disabled:opacity-50"
          onclick={clipUrlNow}
          disabled={saving || !selectedProjectId || !url.trim()}
        >
          {saving ? "클리핑 중..." : "Clip (⌘+Enter)"}
        </button>
      {/if}
    </div>
  {/if}
</div>
