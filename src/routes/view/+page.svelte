<script lang="ts">
  import { page } from "$app/state";
  import { getNote, openInEditor } from "$lib/api";
  import type { RenderedNote } from "$lib/types";

  let note = $state<RenderedNote | null>(null);
  let error = $state("");
  let loading = $state(true);

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

  $effect(() => {
    load(notePath);
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
    <p class="text-sm text-muted">Loading...</p>
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
          class="text-xs px-3 py-1.5 rounded bg-surface-2 border border-border text-muted hover:text-white hover:border-accent transition-colors"
          onclick={handleOpenEditor}
        >
          Open in Editor
        </button>
      </div>

      <!-- Metadata -->
      {#if note.frontmatter}
        <div class="flex items-center gap-3 text-sm text-muted">
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
      <div class="text-xs text-muted mt-1">{note.path}</div>
    </div>

    <!-- Content -->
    <article class="prose prose-invert prose-sm max-w-none mb-8">
      {@html note.html}
    </article>

    <!-- Outgoing Links -->
    {#if note.outgoing_links.length > 0}
      <div class="border-t border-border pt-4 mb-4">
        <h3 class="text-sm font-medium text-muted mb-2">Outgoing Links ({note.outgoing_links.length})</h3>
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
        <h3 class="text-sm font-medium text-muted mb-2">Backlinks ({note.backlinks.length})</h3>
        <div class="space-y-1">
          {#each note.backlinks as bl}
            <a
              href="/view?path={encodeURIComponent(bl.path)}"
              class="flex items-center gap-2 px-3 py-2 rounded-lg hover:bg-surface-1 transition-colors"
            >
              <span class="text-sm text-accent">{bl.title}</span>
              {#if bl.note_type}
                <span class="text-xs text-muted">{typeLabel(bl.note_type)}</span>
              {/if}
              <span class="text-xs text-muted truncate">{bl.context}</span>
            </a>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  :global(article.prose a.wikilink) {
    color: var(--color-accent);
    text-decoration: none;
    border-bottom: 1px dashed var(--color-accent-dim);
  }
  :global(article.prose a.wikilink:hover) {
    border-bottom-style: solid;
  }
  :global(article.prose h1) { font-size: 1.5rem; font-weight: 700; margin: 1.5rem 0 0.75rem; }
  :global(article.prose h2) { font-size: 1.25rem; font-weight: 600; margin: 1.25rem 0 0.5rem; }
  :global(article.prose h3) { font-size: 1.1rem; font-weight: 600; margin: 1rem 0 0.5rem; }
  :global(article.prose p) { margin: 0.5rem 0; line-height: 1.7; }
  :global(article.prose ul) { list-style: disc; padding-left: 1.5rem; margin: 0.5rem 0; }
  :global(article.prose ol) { list-style: decimal; padding-left: 1.5rem; margin: 0.5rem 0; }
  :global(article.prose li) { margin: 0.25rem 0; }
  :global(article.prose code) {
    background: var(--color-surface-2);
    padding: 0.15rem 0.35rem;
    border-radius: 0.25rem;
    font-size: 0.85em;
  }
  :global(article.prose pre) {
    background: var(--color-surface-2);
    padding: 1rem;
    border-radius: 0.5rem;
    overflow-x: auto;
    margin: 0.75rem 0;
  }
  :global(article.prose pre code) {
    background: none;
    padding: 0;
  }
  :global(article.prose blockquote) {
    border-left: 3px solid var(--color-border);
    padding-left: 1rem;
    color: var(--color-muted);
    margin: 0.75rem 0;
  }
</style>
