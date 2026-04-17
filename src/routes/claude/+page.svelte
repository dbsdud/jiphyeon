<script lang="ts">
  import { getClaudeTools, getNote } from "$lib/api";
  import type { ClaudeTools, RenderedNote } from "$lib/types";

  let tools = $state<ClaudeTools | null>(null);
  let claudeNote = $state<RenderedNote | null>(null);
  let loading = $state(true);
  let error = $state("");
  let warningsExpanded = $state(false);

  // 이벤트별 훅 그룹 (UI 표시 순서 고정)
  const EVENT_ORDER = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Stop",
  ];

  let hooksByEvent = $derived.by(() => {
    if (!tools) return new Map();
    const groups = new Map<string, typeof tools.hooks>();
    for (const hook of tools.hooks) {
      const list = groups.get(hook.event) ?? [];
      list.push(hook);
      groups.set(hook.event, list);
    }
    // 알려진 이벤트 먼저, 나머지는 알파벳 순
    const sorted = new Map();
    for (const event of EVENT_ORDER) {
      if (groups.has(event)) sorted.set(event, groups.get(event));
    }
    for (const [event, list] of [...groups.entries()].sort()) {
      if (!sorted.has(event)) sorted.set(event, list);
    }
    return sorted;
  });

  async function load() {
    loading = true;
    error = "";
    try {
      tools = await getClaudeTools();
      if (tools.claude_md) {
        try {
          claudeNote = await getNote(tools.claude_md);
        } catch (e) {
          // CLAUDE.md 렌더링 실패는 페이지 전체를 깨뜨리지 않음
          console.warn("CLAUDE.md 렌더링 실패", e);
        }
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  load();
</script>

<div class="p-6 max-w-5xl">
  <h2 class="text-xl font-semibold mb-6">🤖 Claude 도구</h2>

  {#if loading}
    <p class="text-muted text-sm">Loading...</p>
  {:else if error}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-sm text-danger">
      {error}
    </div>
  {:else if tools}
    <div class="space-y-8">
      <!-- CLAUDE.md -->
      <section>
        <h3 class="text-sm font-semibold text-muted mb-3">CLAUDE.md</h3>
        {#if tools.claude_md && claudeNote}
          <article
            class="bg-surface-1 border border-border rounded-xl p-6 prose prose-invert prose-sm max-w-none"
          >
            {@html claudeNote.html}
          </article>
        {:else if tools.claude_md}
          <div class="bg-surface-1 border border-border rounded-xl p-4 text-sm text-muted">
            CLAUDE.md 파일을 읽을 수 없습니다.
          </div>
        {:else}
          <div class="bg-surface-1 border border-border rounded-xl p-4 text-sm text-muted">
            볼트 루트에 CLAUDE.md가 없습니다. 스캐폴드된 볼트에서는 자동 생성됩니다.
          </div>
        {/if}
      </section>

      <!-- Skills -->
      <section>
        <h3 class="text-sm font-semibold text-muted mb-3">
          Skills ({tools.skills.length})
        </h3>

        {#if tools.skills.length > 0}
          <div class="grid grid-cols-2 gap-3">
            {#each tools.skills as skill}
              <a
                href="/view?path={encodeURIComponent(skill.path)}"
                class="block bg-surface-1 border border-border rounded-xl p-4
                       hover:border-accent hover:bg-surface-2 transition-colors"
              >
                <div class="font-mono text-sm text-white mb-1">{skill.name}</div>
                <div class="text-xs text-muted line-clamp-2">{skill.description}</div>
              </a>
            {/each}
          </div>
        {:else if tools.skill_warnings.length === 0}
          <div class="bg-surface-1 border border-border rounded-xl p-4 text-sm text-muted">
            스킬이 없습니다. <code class="font-mono">.claude/skills/&lt;name&gt;/SKILL.md</code> 파일을 추가하세요.
          </div>
        {/if}

        {#if tools.skill_warnings.length > 0}
          <div class="mt-4 bg-warning/10 border border-warning/30 rounded-xl overflow-hidden">
            <button
              class="w-full flex items-center justify-between px-4 py-3 text-left
                     hover:bg-warning/5 transition-colors"
              onclick={() => {
                warningsExpanded = !warningsExpanded;
              }}
            >
              <span class="text-sm text-warning">
                ⚠️ {tools.skill_warnings.length}개의 SKILL.md를 읽을 수 없습니다
              </span>
              <span class="text-xs text-warning">
                {warningsExpanded ? "접기 ▲" : "펼치기 ▼"}
              </span>
            </button>
            {#if warningsExpanded}
              <ul class="border-t border-warning/30 divide-y divide-warning/20">
                {#each tools.skill_warnings as warning}
                  <li class="px-4 py-3">
                    <div class="font-mono text-xs text-white mb-1">{warning.path}</div>
                    <div class="text-xs text-warning">{warning.reason}</div>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      </section>

      <!-- Hooks -->
      <section>
        <h3 class="text-sm font-semibold text-muted mb-3">
          Hooks ({tools.hooks.length})
        </h3>

        {#if tools.hooks_error}
          <div class="mb-3 bg-danger/10 border border-danger/30 rounded-xl p-4 text-sm text-danger">
            <div class="font-medium mb-1">settings.json 읽기 실패</div>
            <div class="font-mono text-xs">{tools.hooks_error}</div>
          </div>
        {/if}

        {#if tools.hooks.length > 0}
          <div class="space-y-4">
            {#each [...hooksByEvent.entries()] as [event, eventHooks]}
              <div class="bg-surface-1 border border-border rounded-xl overflow-hidden">
                <div class="px-4 py-2 border-b border-border bg-surface-2">
                  <span class="text-xs font-mono text-white">{event}</span>
                  <span class="text-xs text-muted ml-2">({eventHooks.length})</span>
                </div>
                <div class="divide-y divide-border">
                  {#each eventHooks as hook}
                    <div class="px-4 py-3">
                      {#if hook.matcher}
                        <div class="text-xs text-muted mb-1">
                          매처: <span class="font-mono text-white">{hook.matcher}</span>
                        </div>
                      {/if}
                      {#if hook.script_path}
                        <a
                          href="/view?path={encodeURIComponent(hook.script_path)}"
                          class="font-mono text-xs text-accent hover:underline break-all"
                        >
                          {hook.script_path}
                        </a>
                      {:else}
                        <div class="font-mono text-xs text-white break-all">
                          {hook.command}
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {:else if !tools.hooks_error}
          <div class="bg-surface-1 border border-border rounded-xl p-4 text-sm text-muted">
            등록된 훅이 없습니다. `.claude/settings.json`의 `hooks` 섹션에서 설정합니다.
          </div>
        {/if}
      </section>
    </div>
  {/if}
</div>
