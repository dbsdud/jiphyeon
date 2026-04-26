<script lang="ts">
  import { tick } from "svelte";
  import { getNote, openInEditor } from "$lib/api";
  import type { RenderedNote } from "$lib/types";
  import { themeRefresh } from "$lib/stores/theme.svelte";

  interface Props {
    notePath: string;
  }

  let { notePath }: Props = $props();

  let note = $state<RenderedNote | null>(null);
  let error = $state("");
  let loading = $state(false);
  let articleEl = $state<HTMLElement | null>(null);

  async function load(path: string): Promise<void> {
    if (!path) {
      note = null;
      error = "";
      loading = false;
      return;
    }
    loading = true;
    error = "";
    try {
      note = await getNote(path);
    } catch (e) {
      error = String(e);
      note = null;
    } finally {
      loading = false;
    }
  }

  async function applyMarkdownPipeline(): Promise<void> {
    await tick();
    if (!articleEl) return;
    const { renderMarkdownPipeline } = await import("$lib/markdown/pipeline");
    await renderMarkdownPipeline(articleEl);
  }

  $effect(() => {
    load(notePath);
  });

  $effect(() => {
    if (note) {
      applyMarkdownPipeline();
    }
  });

  // 테마 변경 시 원본 HTML 재삽입 후 파이프라인 재실행.
  let themeRefreshInitialized = false;
  $effect(() => {
    themeRefresh.version;
    if (!themeRefreshInitialized) {
      themeRefreshInitialized = true;
      return;
    }
    if (!note || !articleEl) return;
    articleEl.innerHTML = note.html;
    applyMarkdownPipeline();
  });

  function handleOpenEditor(): void {
    if (notePath) openInEditor(notePath);
  }

  function typeLabel(type: string): string {
    const labels: Record<string, string> = {
      til: "TIL",
      decision: "Decision",
      reading: "Reading",
      meeting: "Meeting",
      idea: "Idea",
      artifact: "Artifact",
      clipping: "Clipping",
      moc: "MOC",
      unknown: "Other",
    };
    return labels[type] ?? type;
  }
</script>

{#if !notePath}
  <div class="h-full flex items-center justify-center text-sm text-fg-muted">
    좌측에서 파일을 선택하세요.
  </div>
{:else if loading}
  <div class="p-6 text-sm text-fg-muted">Loading...</div>
{:else if error}
  <div class="p-6">
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger text-sm">{error}</div>
  </div>
{:else if note}
  <div class="p-6 max-w-3xl mx-auto">
    <!-- Header -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-2">
        <h1 class="text-2xl font-bold">{note.title}</h1>
        <button
          class="text-xs px-3 py-1.5 rounded bg-surface-2 border border-border text-fg-muted hover:text-fg hover:border-accent transition-colors"
          onclick={handleOpenEditor}
        >
          Open in Editor
        </button>
      </div>

      {#if note.frontmatter}
        <div class="flex items-center gap-3 text-sm text-fg-muted flex-wrap">
          <span class="px-2 py-0.5 rounded bg-surface-2">{typeLabel(note.frontmatter.note_type)}</span>
          {#if note.frontmatter.status}
            <span>{note.frontmatter.status}</span>
          {/if}
          <span>{note.frontmatter.created}</span>
          {#each note.frontmatter.tags as tag}
            <span class="px-1.5 py-0.5 rounded-full bg-surface-3 text-xs">{tag}</span>
          {/each}
        </div>
      {/if}
      <div class="text-xs text-fg-muted mt-1">{note.path}</div>
    </div>

    <article class="markdown-body prose prose-sm max-w-none mb-8" bind:this={articleEl}>
      {@html note.html}
    </article>

    {#if note.outgoing_links.length > 0}
      <div class="border-t border-border pt-4 mb-4">
        <h3 class="text-sm font-medium text-fg-muted mb-2">Outgoing Links ({note.outgoing_links.length})</h3>
        <div class="flex flex-wrap gap-1.5">
          {#each note.outgoing_links as link}
            <a
              href="/view?path={encodeURIComponent(link + '.md')}"
              class="text-xs px-2 py-1 rounded bg-surface-2 border border-border text-accent hover:bg-accent/10 transition-colors"
            >
              {link}
            </a>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}
