<script lang="ts">
  import { page } from "$app/state";
  import NoteViewer from "$lib/components/NoteViewer.svelte";

  let notePath = $derived(page.url.searchParams.get("path") ?? "");
</script>

<NoteViewer {notePath} />
