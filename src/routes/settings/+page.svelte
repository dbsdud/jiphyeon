<script lang="ts">
  import { getConfig, updateConfig, detectEditors } from "$lib/api";
  import type {
    AppConfig,
    AppConfigPatch,
    Density,
    DetectedEditor,
    ThemePreference,
  } from "$lib/types";
  import { applyTheme, themeRefresh, themePref } from "$lib/stores/theme.svelte";

  let config = $state<AppConfig | null>(null);
  let editors = $state<DetectedEditor[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let error = $state("");
  let savedMessage = $state("");

  let editorCommand = $state("");
  let excludeDirsInput = $state("");
  let globalShortcut = $state("");
  let density = $state<Density>("regular");
  let theme = $state<ThemePreference>("system");

  async function load(): Promise<void> {
    loading = true;
    error = "";
    try {
      const [cfg, eds] = await Promise.all([getConfig(), detectEditors()]);
      config = cfg;
      editors = eds;
      editorCommand = cfg.editor_command;
      excludeDirsInput = cfg.exclude_dirs.join(", ");
      globalShortcut = cfg.global_shortcut;
      density = cfg.density;
      theme = cfg.theme;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  load();

  function pickEditor(editor: DetectedEditor) {
    editorCommand = editor.command;
  }

  async function rescanEditors() {
    try {
      editors = await detectEditors();
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    if (!config) return;
    saving = true;
    error = "";
    savedMessage = "";

    const excludeDirs = excludeDirsInput
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

    const patch: AppConfigPatch = {
      editor_command: editorCommand,
      exclude_dirs: excludeDirs,
      global_shortcut: globalShortcut,
      density,
      theme,
    };

    try {
      config = await updateConfig(patch);
      document.documentElement.dataset.density = config.density;
      themePref.set(config.theme);
      applyTheme(config.theme);
      themeRefresh.bump();
      savedMessage = "저장되었습니다";
      setTimeout(() => {
        savedMessage = "";
      }, 2000);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="p-6 max-w-3xl">
  <h2 class="text-xl font-semibold mb-6">⚙️ Settings</h2>

  {#if loading}
    <p class="text-fg-muted text-sm">Loading...</p>
  {:else if !config}
    <div class="bg-danger/10 border border-danger/30 rounded-lg p-4 text-sm text-danger">
      설정을 불러올 수 없습니다.
    </div>
  {:else}
    <div class="space-y-6">
      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <h3 class="text-sm font-semibold text-fg mb-3">프로젝트</h3>

        <p class="text-xs text-fg-muted">
          프로젝트 추가/전환/제거는 왼쪽 사이드바의 "📁 프로젝트" 섹션에서 할 수 있습니다.
        </p>
      </section>

      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <h3 class="text-sm font-semibold text-fg mb-3">디스플레이</h3>

        <div class="text-xs text-fg-muted mb-2">밀도</div>
        <div class="space-y-2 mb-5">
          <label class="flex items-start gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="density"
              value="regular"
              bind:group={density}
              disabled={saving}
              class="mt-0.5"
            />
            <div>
              <div class="text-fg">기본 (Regular)</div>
              <div class="text-xs text-fg-muted">넉넉한 여백과 표준 글자 크기</div>
            </div>
          </label>

          <label class="flex items-start gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="density"
              value="compact"
              bind:group={density}
              disabled={saving}
              class="mt-0.5"
            />
            <div>
              <div class="text-fg">컴팩트 (Compact)</div>
              <div class="text-xs text-fg-muted">좁은 여백, 더 많은 정보를 한 화면에</div>
            </div>
          </label>
        </div>

        <div class="text-xs text-fg-muted mb-2">테마</div>
        <div class="space-y-2">
          <label class="flex items-start gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="theme"
              value="system"
              bind:group={theme}
              disabled={saving}
              class="mt-0.5"
            />
            <div>
              <div class="text-fg">시스템 설정 따르기 (System)</div>
              <div class="text-xs text-fg-muted">OS의 다크 모드 선호에 맞춰 자동 전환</div>
            </div>
          </label>

          <label class="flex items-start gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="theme"
              value="light"
              bind:group={theme}
              disabled={saving}
              class="mt-0.5"
            />
            <div>
              <div class="text-fg">밝은 테마 (Light)</div>
              <div class="text-xs text-fg-muted">햇살 들어오는 도서관 톤</div>
            </div>
          </label>

          <label class="flex items-start gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="theme"
              value="dark"
              bind:group={theme}
              disabled={saving}
              class="mt-0.5"
            />
            <div>
              <div class="text-fg">어두운 테마 (Dark)</div>
              <div class="text-xs text-fg-muted">집현전의 밤, 낮은 눈부심</div>
            </div>
          </label>
        </div>
      </section>

      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <div class="flex items-center justify-between mb-3">
          <h3 class="text-sm font-semibold text-fg">외부 에디터</h3>
          <button
            class="text-xs px-2 py-1 rounded border border-border text-fg-muted hover:text-fg hover:border-accent transition-colors"
            onclick={rescanEditors}
          >
            재검색
          </button>
        </div>

        {#if editors.length > 0}
          <div class="flex flex-wrap gap-2 mb-3">
            {#each editors as editor}
              <button
                class="text-xs px-3 py-1.5 rounded-full border transition-colors
                  {editorCommand === editor.command
                    ? 'bg-accent text-accent-fg border-accent'
                    : 'border-border text-fg-muted hover:text-fg hover:border-accent'}"
                onclick={() => pickEditor(editor)}
              >
                {editor.label}
              </button>
            {/each}
          </div>
        {:else}
          <p class="text-xs text-fg-muted mb-3">
            감지된 에디터가 없습니다. 아래에 직접 경로/URL을 입력하세요.
          </p>
        {/if}

        <label for="editor-command" class="text-xs text-fg-muted block mb-1">
          커맨드 또는 URL
          <span class="text-fg-muted/70">
            (`{"{path}"}` 플레이스홀더 지원, `://` 포함 시 URL 모드)
          </span>
        </label>
        <input
          id="editor-command"
          type="text"
          class="w-full bg-surface-0 border border-border rounded px-3 py-2 text-sm text-fg font-mono
                 focus:border-accent focus:outline-none"
          bind:value={editorCommand}
          placeholder={`/usr/local/bin/code 또는 obsidian://open?path={path}`}
        />
      </section>

      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <h3 class="text-sm font-semibold text-fg mb-3">감시</h3>

        <label for="exclude-dirs" class="text-xs text-fg-muted block mb-1">
          제외할 디렉토리 (쉼표 구분)
          <span class="text-fg-muted/70">
            — 변경사항은 앱 재시작 후 반영
          </span>
        </label>
        <input
          id="exclude-dirs"
          type="text"
          class="w-full bg-surface-0 border border-border rounded px-3 py-2 text-sm text-fg font-mono
                 focus:border-accent focus:outline-none mb-3"
          bind:value={excludeDirsInput}
          placeholder=".git, .claude, node_modules"
        />

        <label for="global-shortcut" class="text-xs text-fg-muted block mb-1">
          글로벌 단축키
          <span class="text-fg-muted/70">— 변경은 앱 재시작 후 반영</span>
        </label>
        <input
          id="global-shortcut"
          type="text"
          class="w-full bg-surface-0 border border-border rounded px-3 py-2 text-sm text-fg font-mono
                 focus:border-accent focus:outline-none"
          bind:value={globalShortcut}
          placeholder="CmdOrCtrl+Shift+N"
        />
      </section>

      <div class="flex items-center gap-3">
        <button
          class="text-sm px-5 py-2 rounded bg-accent text-accent-fg
                 hover:bg-accent/80 transition-colors
                 disabled:opacity-50 disabled:cursor-not-allowed"
          onclick={save}
          disabled={saving}
        >
          {saving ? "저장 중..." : "저장"}
        </button>

        {#if savedMessage}
          <span class="text-sm text-success">✓ {savedMessage}</span>
        {/if}

        {#if error}
          <span class="text-sm text-danger">{error}</span>
        {/if}
      </div>
    </div>
  {/if}
</div>
