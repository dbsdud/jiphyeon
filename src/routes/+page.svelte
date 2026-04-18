<script lang="ts">
  import {
    getVaultStats,
    getRecentNotes,
    getTagList,
    getClaudeTools,
    getVaultStatus,
  } from "$lib/api";
  import type {
    VaultStats,
    NoteEntry,
    TagInfo,
    ClaudeTools,
  } from "$lib/types";
  import AuditSummary from "$lib/components/AuditSummary.svelte";
  import TagHeatmap from "$lib/components/TagHeatmap.svelte";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let stats = $state<VaultStats | null>(null);
  let recentNotes = $state<NoteEntry[]>([]);
  let tags = $state<TagInfo[]>([]);
  let claudeTools = $state<ClaudeTools | null>(null);
  let vaultName = $state("");
  let error = $state("");

  function deriveName(path: string | null): string {
    if (!path) return "";
    const trimmed = path.replace(/\/+$/, "");
    const idx = trimmed.lastIndexOf("/");
    return idx >= 0 ? trimmed.slice(idx + 1) : trimmed;
  }

  async function load() {
    try {
      const [s, r, t, c, v] = await Promise.all([
        getVaultStats(),
        getRecentNotes(10),
        getTagList(),
        getClaudeTools().catch(() => null),
        getVaultStatus().catch(() => null),
      ]);
      stats = s;
      recentNotes = r;
      tags = t;
      claudeTools = c;
      vaultName = deriveName(v?.vault_path ?? null);
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    vaultRefresh.version; // 볼트 변경 시 자동 재로드
    load();
  });

  function formatDate(timestamp: number): string {
    if (!timestamp) return "";
    return new Date(timestamp * 1000).toLocaleDateString("ko-KR", {
      month: "short",
      day: "numeric",
    });
  }

  function statusColor(status: string): string {
    switch (status) {
      case "seedling": return "text-success";
      case "growing": return "text-accent";
      case "evergreen": return "text-warning";
      case "stale": return "text-danger";
      default: return "text-fg-muted";
    }
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

<div class="p-6 max-w-5xl">
  <div class="mb-6">
    <h2 class="text-xl font-semibold">Dashboard</h2>
    {#if vaultName}
      <div class="text-sm text-fg-muted mt-1">📓 {vaultName}</div>
    {/if}
  </div>

  {#if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-danger text-sm">
      {error}
    </div>
  {:else if !stats}
    <p class="text-fg-muted text-sm">Loading...</p>
  {:else}
    <!-- Stats cards -->
    <div class="grid grid-cols-4 gap-3 mb-6">
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <div class="text-2xl font-bold">{stats.total_notes}</div>
        <div class="text-xs text-fg-muted mt-1">Total Notes</div>
      </div>
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <div class="text-2xl font-bold">{stats.total_links}</div>
        <div class="text-xs text-fg-muted mt-1">Links</div>
      </div>
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <div class="text-2xl font-bold">{stats.total_tags}</div>
        <div class="text-xs text-fg-muted mt-1">Tags</div>
      </div>
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <div class="text-2xl font-bold {stats.orphan_notes > 0 ? 'text-warning' : ''}">{stats.orphan_notes}</div>
        <div class="text-xs text-fg-muted mt-1">Orphan Notes</div>
      </div>
    </div>

    <div class="grid grid-cols-3 gap-4">
      <!-- By Type -->
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <h3 class="text-sm font-medium mb-3 text-fg-muted">By Type</h3>
        {#each Object.entries(stats.by_type).sort((a, b) => b[1] - a[1]) as [type, count]}
          <div class="flex justify-between text-sm py-1">
            <span>{typeLabel(type)}</span>
            <span class="text-fg-muted">{count}</span>
          </div>
        {/each}
        {#if Object.keys(stats.by_type).length === 0}
          <p class="text-xs text-fg-muted">No typed notes</p>
        {/if}
      </div>

      <!-- By Status -->
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <h3 class="text-sm font-medium mb-3 text-fg-muted">By Status</h3>
        {#each Object.entries(stats.by_status).sort((a, b) => b[1] - a[1]) as [status, count]}
          <div class="flex justify-between text-sm py-1">
            <span class={statusColor(status)}>{status}</span>
            <span class="text-fg-muted">{count}</span>
          </div>
        {/each}
        {#if Object.keys(stats.by_status).length === 0}
          <p class="text-xs text-fg-muted">No status set</p>
        {/if}
      </div>

      <!-- Tags Heatmap -->
      <div class="bg-surface-1 rounded-lg p-4 border border-border overflow-hidden">
        <h3 class="text-sm font-medium mb-3 text-fg-muted">Tags</h3>
        {#if tags.length > 0}
          <TagHeatmap {tags} height={200} />
        {:else}
          <p class="text-xs text-fg-muted">No tags</p>
        {/if}
      </div>
    </div>

    <!-- Claude 도구 요약 -->
    {#if claudeTools}
      <a
        href="/claude"
        class="block mt-6 bg-surface-1 rounded-lg border border-border p-4
               hover:border-accent hover:bg-surface-2 transition-colors"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <span class="text-2xl">🤖</span>
            <div>
              <div class="text-sm font-medium text-fg">Claude 도구</div>
              <div class="text-xs text-fg-muted mt-0.5">
                스킬 {claudeTools.skills.length}개 · 훅 {claudeTools.hooks.length}개
                {#if claudeTools.claude_md}· CLAUDE.md{/if}
                {#if claudeTools.skill_warnings.length > 0 || claudeTools.hooks_error}
                  <span class="text-warning ml-1">
                    ⚠️ 경고 {claudeTools.skill_warnings.length +
                      (claudeTools.hooks_error ? 1 : 0)}
                  </span>
                {/if}
              </div>
            </div>
          </div>
          <span class="text-xs text-fg-muted">자세히 →</span>
        </div>
      </a>
    {/if}

    <!-- Recent Notes -->
    <div class="mt-6">
      <h3 class="text-sm font-medium mb-3 text-fg-muted">Recent Notes</h3>
      <div class="bg-surface-1 rounded-lg border border-border divide-y divide-border">
        {#each recentNotes as note}
          <a
            href="/view?path={encodeURIComponent(note.path)}"
            class="flex items-center justify-between px-4 py-2.5 hover:bg-surface-2 transition-colors"
          >
            <div class="flex items-center gap-3 min-w-0">
              <span class="text-sm truncate">{note.title}</span>
              {#if note.frontmatter}
                <span class="text-xs px-1.5 py-0.5 rounded bg-surface-3 text-fg-muted shrink-0">
                  {typeLabel(note.frontmatter.note_type)}
                </span>
              {/if}
            </div>
            <span class="text-xs text-fg-muted shrink-0 ml-3">{formatDate(note.modified_at)}</span>
          </a>
        {/each}
        {#if recentNotes.length === 0}
          <div class="px-4 py-6 text-center text-sm text-fg-muted">
            No notes found. Check vault path in config.
          </div>
        {/if}
      </div>
    </div>

    <!-- Audit Summary -->
    <div class="mt-6">
      <AuditSummary brokenLinks={stats.broken_links} />
    </div>
  {/if}
</div>
