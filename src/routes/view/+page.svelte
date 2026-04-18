<script lang="ts">
  import { page } from "$app/state";
  import { tick } from "svelte";
  import { getNote, openInEditor } from "$lib/api";
  import type { RenderedNote } from "$lib/types";
  import { themeRefresh } from "$lib/stores/theme.svelte";

  let note = $state<RenderedNote | null>(null);
  let error = $state("");
  let loading = $state(true);
  let articleEl = $state<HTMLElement | null>(null);

  let notePath = $derived(page.url.searchParams.get("path") ?? "");

  async function load(path: string) {
    if (!path) {
      error = "No note path specified.";
      loading = false;
      return;
    }
    loading = true;
    error = "";
    try {
      note = await getNote(path);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function applyMarkdownPipeline() {
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

  // 테마 변경 시 현재 노트를 원본 HTML로 되돌리고 파이프라인 재실행.
  // Mermaid는 `<pre>`를 `<div class="diagram">`으로 치환하므로 단순 마커 리셋만으로는
  // 재렌더가 불가능 — 원본 HTML 재삽입이 가장 간단.
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

  function handleOpenEditor() {
    if (notePath) openInEditor(notePath);
  }

  function typeLabel(type: string): string {
    const labels: Record<string, string> = {
      til: "TIL", decision: "Decision", reading: "Reading", meeting: "Meeting",
      idea: "Idea", artifact: "Artifact", clipping: "Clipping", moc: "MOC", unknown: "Other",
    };
    return labels[type] ?? type;
  }
</script>

<div class="p-6 max-w-3xl mx-auto">
  {#if loading}
    <p class="text-sm text-fg-muted">Loading...</p>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger text-sm">
      {error}
    </div>
  {:else if note}
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

      <!-- Metadata -->
      {#if note.frontmatter}
        <div class="flex items-center gap-3 text-sm text-fg-muted">
          <span class="px-2 py-0.5 rounded bg-surface-2">{typeLabel(note.frontmatter.note_type)}</span>
          {#if note.frontmatter.status}
            <span>{note.frontmatter.status}</span>
          {/if}
          <span>{note.frontmatter.created}</span>
          {#each note.frontmatter.tags as tag}
            <a
              href="/explore?tag={encodeURIComponent(tag)}"
              class="px-1.5 py-0.5 rounded-full bg-surface-3 text-xs hover:bg-accent/20 hover:text-accent transition-colors"
            >
              {tag}
            </a>
          {/each}
        </div>
      {/if}
      <div class="text-xs text-fg-muted mt-1">{note.path}</div>
    </div>

    <!-- Content -->
    <article class="markdown-body prose prose-sm max-w-none mb-8" bind:this={articleEl}>
      {@html note.html}
    </article>

    <!-- Outgoing Links -->
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

    <!-- Backlinks -->
    {#if note.backlinks.length > 0}
      <div class="border-t border-border pt-4">
        <h3 class="text-sm font-medium text-fg-muted mb-2">Backlinks ({note.backlinks.length})</h3>
        <div class="space-y-1">
          {#each note.backlinks as bl}
            <a
              href="/view?path={encodeURIComponent(bl.path)}"
              class="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-surface-1 transition-colors"
            >
              <span class="text-sm text-accent">{bl.title}</span>
              {#if bl.note_type}
                <span class="text-xs text-fg-muted">{typeLabel(bl.note_type)}</span>
              {/if}
              <span class="text-xs text-fg-muted truncate">{bl.context}</span>
            </a>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

