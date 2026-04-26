<script lang="ts">
  import {
    getActiveProject,
    getGraphifyReport,
    getGraphifyStatus,
  } from "$lib/api";
  import type { GraphReport, GraphifyStatus, ProjectEntry } from "$lib/types";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let project = $state<ProjectEntry | null>(null);
  let status = $state<GraphifyStatus | null>(null);
  let report = $state<GraphReport | null>(null);
  let loading = $state(true);
  let error = $state("");

  async function load(): Promise<void> {
    loading = true;
    error = "";
    try {
      const [p, s] = await Promise.all([
        getActiveProject().catch(() => null),
        getGraphifyStatus(),
      ]);
      project = p;
      status = s;
      report = s.graph_json_exists && s.report_md_exists
        ? await getGraphifyReport().catch(() => null)
        : null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    vaultRefresh.version;
    load();
  });

  function formatRelative(iso: string | null): string {
    if (!iso) return "";
    const ts = Date.parse(iso);
    if (Number.isNaN(ts)) return iso;
    const diffSec = Math.round((Date.now() - ts) / 1000);
    const rtf = new Intl.RelativeTimeFormat("ko", { numeric: "auto" });
    if (diffSec < 60) return rtf.format(-diffSec, "second");
    if (diffSec < 3600) return rtf.format(-Math.round(diffSec / 60), "minute");
    if (diffSec < 86400) return rtf.format(-Math.round(diffSec / 3600), "hour");
    return rtf.format(-Math.round(diffSec / 86400), "day");
  }

  const topCommunities = $derived(
    (report?.communities ?? [])
      .slice()
      .sort((a, b) => (b.nodes_count ?? 0) - (a.nodes_count ?? 0))
      .slice(0, 5),
  );
</script>

<div class="p-6 max-w-5xl">
  <div class="mb-6 flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">Dashboard</h2>
      {#if project}
        <div class="text-sm text-fg-muted mt-1">📁 {project.name}</div>
      {/if}
    </div>
    {#if status?.last_run_at}
      <div class="text-xs text-fg-muted">last graphify: {formatRelative(status.last_run_at)}</div>
    {/if}
  </div>

  {#if loading}
    <p class="text-sm text-fg-muted">Loading...</p>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-sm text-danger break-all">
      {error}
    </div>
  {:else if !project}
    <div class="bg-surface-1 border border-border rounded-xl p-8 text-center">
      <p class="text-sm text-fg-muted">활성 프로젝트가 없습니다. 사이드바에서 프로젝트를 등록하세요.</p>
    </div>
  {:else if !status?.graph_json_exists}
    <div class="bg-surface-1 border border-border rounded-xl p-8 text-center">
      <p class="text-sm text-fg mb-3">이 프로젝트에서 graphify 가 실행되지 않았습니다.</p>
      {#if status?.graphify_out_path}
        <p class="text-xs text-fg-muted mb-2">
          hub 경로: <code class="px-1 py-0.5 rounded bg-surface-2">{status.graphify_out_path}</code>
        </p>
      {/if}
      <p class="text-xs text-fg-muted">
        터미널에서
        <code class="px-1 py-0.5 rounded bg-surface-2">cd ~/Jiphyeon/{project.name}</code>
        후 Claude Code 에서
        <code class="px-1 py-0.5 rounded bg-surface-2">/graphify</code>
        를 실행하세요.
      </p>
    </div>
  {:else}
    <!-- Summary -->
    <div class="grid grid-cols-4 gap-3 mb-6">
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <div class="text-2xl font-bold">{status.nodes_count ?? "—"}</div>
        <div class="text-xs text-fg-muted mt-1">Nodes</div>
      </div>
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <div class="text-2xl font-bold">{status.edges_count ?? "—"}</div>
        <div class="text-xs text-fg-muted mt-1">Edges</div>
      </div>
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        <div class="text-2xl font-bold">{report?.summary.communities_count ?? "—"}</div>
        <div class="text-xs text-fg-muted mt-1">Communities</div>
      </div>
      <div class="bg-surface-1 rounded-lg p-4 border border-border">
        {#if report?.summary.token_input !== undefined && report?.summary.token_output !== undefined && report?.summary.token_input !== null}
          <div class="text-sm font-mono">
            in {report.summary.token_input.toLocaleString()}<br />
            out {report.summary.token_output?.toLocaleString() ?? 0}
          </div>
          <div class="text-xs text-fg-muted mt-1">Token cost</div>
        {:else}
          <div class="text-2xl font-bold">—</div>
          <div class="text-xs text-fg-muted mt-1">Token cost</div>
        {/if}
      </div>
    </div>

    <!-- Extraction breakdown -->
    {#if report?.summary.extracted_pct !== null && report?.summary.extracted_pct !== undefined}
      <div class="bg-surface-1 border border-border rounded-lg p-4 mb-6 text-xs text-fg-muted">
        Extraction: {report.summary.extracted_pct}% EXTRACTED ·
        {report.summary.inferred_pct ?? 0}% INFERRED ·
        {report.summary.ambiguous_pct ?? 0}% AMBIGUOUS
      </div>
    {/if}

    {#if !report}
      <div class="bg-surface-1 border border-border rounded-lg p-4 text-sm text-fg-muted">
        GRAPH_REPORT.md 가 없거나 파싱할 수 없습니다. /graph 에서 그래프를 직접 보세요.
      </div>
    {:else}
      <!-- God Nodes -->
      <div class="bg-surface-1 rounded-lg border border-border mb-6">
        <h3 class="text-sm font-medium text-fg-muted px-4 pt-4 pb-2">God Nodes</h3>
        {#if report.god_nodes.length > 0}
          <div class="divide-y divide-border">
            {#each report.god_nodes.slice(0, 10) as node}
              <div class="flex items-center justify-between px-4 py-2.5">
                <div class="flex items-center gap-3 min-w-0">
                  <span class="text-xs text-fg-muted w-4 text-right shrink-0">{node.rank}</span>
                  <code class="text-sm truncate">{node.name}</code>
                </div>
                <span class="text-xs text-fg-muted shrink-0 ml-3">{node.edge_count} edges</span>
              </div>
            {/each}
          </div>
        {:else}
          <p class="text-xs text-fg-muted px-4 pb-4">God Nodes 섹션이 비어있습니다.</p>
        {/if}
      </div>

      <!-- Surprising -->
      <div class="bg-surface-1 rounded-lg border border-border mb-6">
        <h3 class="text-sm font-medium text-fg-muted px-4 pt-4 pb-2">Surprising Connections</h3>
        {#if report.surprising_connections.length > 0}
          <div class="divide-y divide-border max-h-64 overflow-y-auto">
            {#each report.surprising_connections.slice(0, 5) as conn}
              <div class="px-4 py-2.5 text-sm">
                <code>{conn.source}</code>
                <span class="text-fg-muted text-xs">--{conn.relation}--></span>
                <code>{conn.target}</code>
                <span class="ml-2 text-[10px] px-1.5 py-0.5 rounded bg-surface-3 text-fg-muted">
                  {conn.confidence}
                </span>
              </div>
            {/each}
          </div>
        {:else}
          <p class="text-xs text-fg-muted px-4 pb-4">Surprising 섹션이 비어있습니다.</p>
        {/if}
      </div>

      <!-- Communities -->
      <div class="bg-surface-1 rounded-lg border border-border mb-6">
        <h3 class="text-sm font-medium text-fg-muted px-4 pt-4 pb-2">Top Communities</h3>
        {#if topCommunities.length > 0}
          <div class="divide-y divide-border">
            {#each topCommunities as comm}
              <div class="px-4 py-2.5">
                <div class="flex items-center justify-between">
                  <span class="text-sm">#{comm.id} · {comm.label}</span>
                  <span class="text-xs text-fg-muted">{comm.nodes_count ?? "?"} nodes</span>
                </div>
                {#if comm.sample_nodes.length > 0}
                  <p class="text-xs text-fg-muted mt-1 truncate">
                    {comm.sample_nodes.slice(0, 5).join(", ")}
                  </p>
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <p class="text-xs text-fg-muted px-4 pb-4">Communities 섹션이 비어있습니다.</p>
        {/if}
      </div>
    {/if}
  {/if}
</div>
