<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    saveRecording,
    deleteRecording,
    listRecordings,
  } from "$lib/api";
  import type { RecordingEntry } from "$lib/types";
  import { RollingRecorder } from "$lib/transcribe/recorder";
  import { WaveformRenderer } from "$lib/transcribe/waveform";
  import { buildFilename, extensionForMime } from "$lib/transcribe/format";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let recording = $state(false);
  let elapsedMs = $state(0);
  let error = $state("");
  let recordings = $state<RecordingEntry[]>([]);
  let waveform: WaveformRenderer | undefined;
  let pasteSeq = 0;

  const recorder = new RollingRecorder({
    async onChunkSaved(filename, blob) {
      const bytes = new Uint8Array(await blob.arrayBuffer());
      await saveRecording(filename, bytes);
      await refresh();
    },
    onError(msg) {
      error = msg;
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

  async function refresh(): Promise<void> {
    try {
      recordings = await listRecordings();
    } catch (e) {
      error = `목록 로드 실패: ${String(e)}`;
    }
  }

  async function start(): Promise<void> {
    error = "";
    await recorder.start();
    recording = recorder.isRecording;
  }

  function stop(): void {
    recorder.stop();
    recording = false;
    elapsedMs = 0;
  }

  async function removeRecording(filename: string): Promise<void> {
    try {
      await deleteRecording(filename);
      await refresh();
    } catch (e) {
      error = `삭제 실패: ${String(e)}`;
    }
  }

  async function handlePaste(e: ClipboardEvent): Promise<void> {
    const files = e.clipboardData?.files;
    if (!files || files.length === 0) return;
    let audioFound = false;
    for (const file of Array.from(files)) {
      if (!file.type.startsWith("audio/")) continue;
      audioFound = true;
      const ext = extensionForMime(file.type);
      pasteSeq += 1;
      const filename = buildFilename(new Date(), pasteSeq, ext);
      try {
        const bytes = new Uint8Array(await file.arrayBuffer());
        await saveRecording(filename, bytes);
        error = "";
      } catch (err) {
        error = `붙여넣기 저장 실패: ${String(err)}`;
      }
    }
    if (files.length > 0 && !audioFound) {
      error = "오디오 파일만 지원합니다.";
    }
    if (audioFound) await refresh();
  }

  function formatElapsed(ms: number): string {
    const total = Math.floor(ms / 1000);
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    return [h, m, s].map((n) => n.toString().padStart(2, "0")).join(":");
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function formatDate(ts: number): string {
    if (!ts) return "";
    return new Date(ts * 1000).toLocaleString("ko-KR", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  $effect(() => {
    vaultRefresh.version;
    void refresh();
  });

  onMount(() => {
    const handler = (e: ClipboardEvent) => {
      void handlePaste(e);
    };
    window.addEventListener("paste", handler);
    return () => window.removeEventListener("paste", handler);
  });

  onDestroy(() => {
    if (recording) recorder.stop();
    waveform?.detach();
  });
</script>

<div class="h-full flex flex-col p-6 max-w-3xl mx-auto w-full">
  <div class="mb-6">
    <h2 class="text-xl font-semibold">Transcribe</h2>
    <p class="text-sm text-fg-muted mt-1">
      볼트의 <code class="text-xs">_sources/recordings/</code>에 녹음을 저장합니다. 오디오 파일은 <kbd class="text-xs px-1 py-0.5 border border-border rounded">⌘V</kbd>로 붙여넣어도 저장됩니다.
    </p>
  </div>

  {#if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-3 text-danger text-sm mb-4">
      {error}
    </div>
  {/if}

  <!-- Record / Stop -->
  <div class="flex items-center gap-4 mb-4">
    {#if !recording}
      <button
        onclick={start}
        class="px-5 py-2.5 rounded-lg border border-border bg-surface-1 hover:border-accent hover:bg-surface-2 text-fg font-medium transition-colors flex items-center gap-2.5"
      >
        <span class="w-3 h-3 rounded-full bg-danger"></span>
        Record
      </button>
    {:else}
      <button
        onclick={stop}
        class="px-5 py-2.5 rounded-lg bg-accent text-accent-fg font-medium hover:opacity-90 transition-opacity flex items-center gap-2.5"
      >
        <span class="w-3 h-3 rounded-sm bg-accent-fg"></span>
        Stop
      </button>
    {/if}
    <span class="font-mono text-lg text-fg-muted tabular-nums">
      {formatElapsed(elapsedMs)}
    </span>
  </div>

  <!-- Waveform -->
  <div class="bg-surface-1 rounded-lg border border-border p-4 mb-6">
    <canvas class="w-full h-24 block" use:setupWaveform></canvas>
    {#if !recording}
      <p class="text-xs text-fg-muted mt-2 text-center">
        Record를 눌러 녹음을 시작하세요.
      </p>
    {/if}
  </div>

  <!-- Recordings list -->
  <div>
    <div class="flex items-baseline justify-between mb-2">
      <h3 class="text-sm font-medium text-fg-muted">녹음 파일</h3>
      <span class="text-xs text-fg-muted">{recordings.length}개</span>
    </div>
    {#if recordings.length === 0}
      <p class="text-xs text-fg-muted">아직 녹음이 없습니다.</p>
    {:else}
      <div class="bg-surface-1 rounded-lg border border-border divide-y divide-border">
        {#each recordings as item}
          <div class="px-4 py-2.5 flex items-center justify-between gap-3 text-sm">
            <div class="flex items-center gap-2 min-w-0">
              <span class="truncate">{item.filename}</span>
            </div>
            <div class="flex items-center gap-3 shrink-0 text-xs text-fg-muted">
              <span>{formatSize(item.size)}</span>
              <span>{formatDate(item.modified_at)}</span>
              <button
                aria-label="녹음 삭제"
                title="녹음 삭제"
                onclick={() => removeRecording(item.filename)}
                class="hover:text-danger transition-colors text-sm leading-none px-1"
              >
                ×
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
