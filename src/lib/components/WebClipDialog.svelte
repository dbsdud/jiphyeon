<script lang="ts">
  import { clipUrl } from "$lib/api";

  interface Props {
    open: boolean;
    onclose: () => void;
    onsuccess: (path: string, title: string) => void;
  }

  let { open, onclose, onsuccess }: Props = $props();

  let url = $state("");
  let tagsInput = $state("");
  let loading = $state(false);
  let error = $state("");

  async function handleClip() {
    if (!url.trim()) return;
    loading = true;
    error = "";

    try {
      const tags = tagsInput
        .split(",")
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

      const result = await clipUrl({
        url: url.trim(),
        tags: tags.length > 0 ? tags : undefined,
      });

      if (result.success) {
        onsuccess(result.path, result.title);
        url = "";
        tagsInput = "";
        onclose();
      } else {
        error = result.error ?? "Unknown error";
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

{#if open}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 bg-black/60 z-40"
    onclick={onclose}
    role="presentation"
  ></div>

  <!-- Dialog -->
  <div class="fixed top-1/3 left-1/2 -translate-x-1/2 -translate-y-1/2 z-50 w-96">
    <div class="bg-surface-1 border border-border rounded-xl shadow-2xl p-5">
      <h3 class="text-sm font-semibold mb-4">Web Clip</h3>

      <div class="space-y-3">
        <div>
          <label for="clip-url" class="text-xs text-fg-muted block mb-1">URL</label>
          <input
            id="clip-url"
            type="url"
            placeholder="https://..."
            class="w-full px-3 py-2 text-sm bg-surface-2 border border-border rounded-lg text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent"
            bind:value={url}
            onkeydown={(e) => { if (e.key === "Enter" && !loading) handleClip(); }}
          />
        </div>

        <div>
          <label for="clip-tags" class="text-xs text-fg-muted block mb-1">Tags (comma separated)</label>
          <input
            id="clip-tags"
            type="text"
            placeholder="rust, web, ..."
            class="w-full px-3 py-2 text-sm bg-surface-2 border border-border rounded-lg text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent"
            bind:value={tagsInput}
          />
        </div>

        {#if error}
          <p class="text-xs text-danger">{error}</p>
        {/if}

        <div class="flex justify-end gap-2 pt-1">
          <button
            class="text-xs px-3 py-1.5 rounded-lg text-fg-muted hover:text-fg transition-colors"
            onclick={onclose}
            disabled={loading}
          >
            Cancel
          </button>
          <button
            class="text-xs px-4 py-1.5 rounded-lg bg-accent text-accent-fg hover:bg-accent/80 transition-colors disabled:opacity-50"
            onclick={handleClip}
            disabled={loading || !url.trim()}
          >
            {loading ? "Clipping..." : "Clip"}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
