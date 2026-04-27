<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    clipUrl,
    createQuickNote,
    getActiveProject,
    listProjects,
    saveRecording,
  } from "$lib/api";
  import type { ProjectEntry } from "$lib/types";
  import { RollingRecorder } from "$lib/transcribe/recorder";
  import { WaveformRenderer } from "$lib/transcribe/waveform";

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

  // record tab state
  let recording = $state(false);
  let recordingError = $state("");
  let elapsedMs = $state(0);
  let waveform: WaveformRenderer | undefined;
  let savedDuringStop = false;
  let pendingProjectId: string | null = null;

  const recorder = new RollingRecorder({
    async onChunkSaved(filename, blob) {
      const bytes = new Uint8Array(await blob.arrayBuffer());
      try {
        await saveRecording(filename, bytes, pendingProjectId);
        savedDuringStop = true;
      } catch (e) {
        recordingError = `저장 실패: ${String(e)}`;
      }
    },
    onError(msg) {
      recordingError = msg;
      recording = false;
    },
    onElapsed(ms) {
      elapsedMs = ms;
    },
    onAnalyser(analyser) {
      waveform?.attach(analyser);
    },
  });

  function setupWaveform(el: HTMLCanvasElement): void {
    waveform = new WaveformRenderer(el);
    const dpr = window.devicePixelRatio || 1;
    el.width = el.clientWidth * dpr;
    el.height = el.clientHeight * dpr;
  }

  async function startRecording(): Promise<void> {
    if (!selectedProjectId) {
      recordingError = "프로젝트를 선택해주세요.";
      return;
    }
    recordingError = "";
    pendingProjectId = selectedProjectId;
    savedDuringStop = false;
    await recorder.start();
    recording = recorder.isRecording;
  }

  async function stopRecordingAndClose(): Promise<void> {
    recorder.stop();
    recording = false;
    elapsedMs = 0;
    // RollingRecorder.stop() 이 마지막 chunk 저장을 트리거. onChunkSaved 가 비동기라 잠깐 대기.
    const start = Date.now();
    while (!savedDuringStop && Date.now() - start < 3000) {
      await new Promise((r) => setTimeout(r, 50));
    }
    if (savedDuringStop) {
      await closeWindow();
    }
  }

  function formatElapsed(ms: number): string {
    const total = Math.floor(ms / 1000);
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    return [h, m, s].map((n) => n.toString().padStart(2, "0")).join(":");
  }

  onDestroy(() => {
    if (recording) recorder.stop();
    waveform?.detach();
  });

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
        <!-- record tab -->
        <div class="bg-surface-1 rounded border border-border p-3 shrink-0">
          <canvas class="w-full h-20 block" use:setupWaveform></canvas>
        </div>
        <div class="flex items-center gap-3 shrink-0">
          {#if !recording}
            <button
              onclick={startRecording}
              class="px-4 py-2 rounded border border-border bg-surface-1 hover:border-accent hover:bg-surface-2 text-fg text-sm flex items-center gap-2"
              disabled={!selectedProjectId}
            >
              <span class="w-2.5 h-2.5 rounded-full bg-danger"></span>
              Record
            </button>
          {:else}
            <button
              onclick={stopRecordingAndClose}
              class="px-4 py-2 rounded bg-accent text-accent-fg text-sm flex items-center gap-2"
            >
              <span class="w-2.5 h-2.5 rounded-sm bg-accent-fg"></span>
              Stop & 저장
            </button>
          {/if}
          <span class="font-mono text-sm text-fg-muted tabular-nums">
            {formatElapsed(elapsedMs)}
          </span>
        </div>
        <p class="text-xs text-fg-muted shrink-0">
          저장 위치: <code class="px-1 py-0.5 rounded bg-surface-2">docs/_sources/recordings/</code>
        </p>
        {#if recordingError}
          <p class="text-xs text-danger shrink-0">{recordingError}</p>
        {/if}
        <div class="flex-1"></div>
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
      <!-- 녹음 탭은 Record/Stop 버튼이 본문에 있음 -->
    </div>
  {/if}
</div>
