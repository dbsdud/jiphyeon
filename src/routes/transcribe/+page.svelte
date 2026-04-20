<script lang="ts">
  import { onDestroy } from "svelte";
  import { saveRecording, deleteRecording } from "$lib/api";
  import { RollingRecorder } from "$lib/transcribe/recorder";
  import { WaveformRenderer } from "$lib/transcribe/waveform";

  interface SavedChunk {
    filename: string;
    path: string;
    size: number;
  }

  let recording = $state(false);
  let elapsedMs = $state(0);
  let error = $state("");
  let saved = $state<SavedChunk[]>([]);
  let canvas: HTMLCanvasElement | undefined;
  let waveform: WaveformRenderer | undefined;

  const recorder = new RollingRecorder({
    async onChunkSaved(filename, blob) {
      const bytes = new Uint8Array(await blob.arrayBuffer());
      const path = await saveRecording(filename, bytes);
      saved = [{ filename, path, size: bytes.length }, ...saved];
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
    canvas = el;
    waveform = new WaveformRenderer(el);
    const dpr = window.devicePixelRatio || 1;
    el.width = el.clientWidth * dpr;
    el.height = el.clientHeight * dpr;
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
      saved = saved.filter((s) => s.filename !== filename);
    } catch (e) {
      error = `삭제 실패: ${String(e)}`;
    }
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

  onDestroy(() => {
    if (recording) recorder.stop();
    waveform?.detach();
  });
</script>

<div class="h-full flex flex-col p-6 max-w-3xl mx-auto w-full">
  <div class="mb-6">
    <h2 class="text-xl font-semibold">Transcribe</h2>
    <p class="text-sm text-fg-muted mt-1">
      볼트의 <code class="text-xs">_sources/recordings/</code>에 녹음을 저장합니다. 전사는 Claude 세션에서 <code class="text-xs">/vault-transcribe</code>로 수행합니다.
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
    <canvas
      class="w-full h-24 block"
      use:setupWaveform
    ></canvas>
    {#if !recording}
      <p class="text-xs text-fg-muted mt-2 text-center">
        Record를 눌러 녹음을 시작하세요.
      </p>
    {/if}
  </div>

  <!-- Saved files -->
  <div>
    <h3 class="text-sm font-medium text-fg-muted mb-2">방금 저장한 녹음</h3>
    {#if saved.length === 0}
      <p class="text-xs text-fg-muted">아직 없음.</p>
    {:else}
      <div class="bg-surface-1 rounded-lg border border-border divide-y divide-border">
        {#each saved as item}
          <div class="px-4 py-2 flex items-center justify-between gap-3 text-sm">
            <span class="truncate">{item.filename}</span>
            <div class="flex items-center gap-3 shrink-0">
              <span class="text-xs text-fg-muted">{formatSize(item.size)}</span>
              <button
                aria-label="녹음 삭제"
                title="녹음 삭제"
                onclick={() => removeRecording(item.filename)}
                class="text-fg-muted hover:text-danger transition-colors text-sm leading-none px-1"
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
