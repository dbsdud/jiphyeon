<script lang="ts">
  import {
    getConfig,
    updateConfig,
    detectEditors,
    rescaffoldActiveVault,
  } from "$lib/api";
  import type {
    AppConfig,
    AppConfigPatch,
    DetectedEditor,
    RescaffoldMode,
    RescaffoldReport,
  } from "$lib/types";

  let config = $state<AppConfig | null>(null);
  let editors = $state<DetectedEditor[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let error = $state("");
  let savedMessage = $state("");

  // 폼 필드 (config 로드 후 초기화)
  let editorCommand = $state("");
  let excludeDirsInput = $state("");
  let recentNotesLimit = $state(20);
  let globalShortcut = $state("");
  let quickNoteFolder = $state("");

  // 볼트 업데이트(재스캐폴드) 상태
  let rescaffoldMode = $state<RescaffoldMode>("add-missing");
  let rescaffoldReport = $state<RescaffoldReport | null>(null);
  let rescaffoldBusy = $state(false);
  let rescaffoldError = $state("");
  let rescaffoldMessage = $state("");

  async function load() {
    loading = true;
    error = "";
    try {
      const [cfg, eds] = await Promise.all([getConfig(), detectEditors()]);
      config = cfg;
      editors = eds;
      editorCommand = cfg.editor_command;
      excludeDirsInput = cfg.exclude_dirs.join(", ");
      recentNotesLimit = cfg.recent_notes_limit;
      globalShortcut = cfg.global_shortcut;
      quickNoteFolder = cfg.quick_note_folder;
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
      recent_notes_limit: recentNotesLimit,
      global_shortcut: globalShortcut,
      quick_note_folder: quickNoteFolder,
    };

    try {
      config = await updateConfig(patch);
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

  async function previewRescaffold() {
    rescaffoldBusy = true;
    rescaffoldError = "";
    rescaffoldMessage = "";
    try {
      rescaffoldReport = await rescaffoldActiveVault(rescaffoldMode, true);
    } catch (e) {
      rescaffoldError = String(e);
      rescaffoldReport = null;
    } finally {
      rescaffoldBusy = false;
    }
  }

  async function applyRescaffold() {
    if (!rescaffoldReport) return;
    rescaffoldBusy = true;
    rescaffoldError = "";
    rescaffoldMessage = "";
    try {
      rescaffoldReport = await rescaffoldActiveVault(rescaffoldMode, false);
      const { created, overwritten, unchanged } = rescaffoldReport;
      rescaffoldMessage = `적용 완료: 생성 ${created.length}개 / 덮어쓰기 ${overwritten.length}개 / 변경 없음 ${unchanged}개`;
    } catch (e) {
      rescaffoldError = String(e);
    } finally {
      rescaffoldBusy = false;
    }
  }

  function onModeChange() {
    // 모드 바꾸면 기존 미리보기는 무효
    rescaffoldReport = null;
    rescaffoldMessage = "";
    rescaffoldError = "";
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
      <!-- Vault (read-only 표시; 관리는 사이드바에서) -->
      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <h3 class="text-sm font-semibold text-fg mb-3">볼트</h3>

        <div class="text-xs text-fg-muted mb-1">현재 연결된 볼트</div>
        <div class="font-mono text-sm text-fg break-all mb-2">
          {config.vault_path ?? "(연결 안 됨)"}
        </div>

        <p class="text-xs text-fg-muted">
          볼트 추가/전환/제거는 왼쪽 사이드바의 "📓 볼트" 섹션에서 할 수 있습니다.
        </p>
      </section>

      <!-- Editor -->
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
                    ? 'bg-accent text-fg border-accent'
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

      <!-- Indexing -->
      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <h3 class="text-sm font-semibold text-fg mb-3">인덱싱</h3>

        <label for="exclude-dirs" class="text-xs text-fg-muted block mb-1">
          제외할 디렉토리 (쉼표 구분)
          <span class="text-fg-muted/70">
            — 변경사항은 앱 재시작 또는 rescan 시 반영
          </span>
        </label>
        <input
          id="exclude-dirs"
          type="text"
          class="w-full bg-surface-0 border border-border rounded px-3 py-2 text-sm text-fg font-mono
                 focus:border-accent focus:outline-none mb-3"
          bind:value={excludeDirsInput}
          placeholder=".git, .claude, _templates"
        />

        <label for="recent-limit" class="text-xs text-fg-muted block mb-1">최근 노트 개수</label>
        <input
          id="recent-limit"
          type="number"
          min="1"
          max="200"
          class="w-32 bg-surface-0 border border-border rounded px-3 py-2 text-sm text-fg
                 focus:border-accent focus:outline-none"
          bind:value={recentNotesLimit}
        />
      </section>

      <!-- Quick Note -->
      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <h3 class="text-sm font-semibold text-fg mb-3">퀵 노트</h3>

        <label for="quick-folder" class="text-xs text-fg-muted block mb-1">저장 폴더</label>
        <input
          id="quick-folder"
          type="text"
          class="w-full bg-surface-0 border border-border rounded px-3 py-2 text-sm text-fg font-mono
                 focus:border-accent focus:outline-none mb-3"
          bind:value={quickNoteFolder}
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

      <!-- Save -->
      <div class="flex items-center gap-3">
        <button
          class="text-sm px-5 py-2 rounded bg-accent text-fg
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

      <!-- Vault Update (Rescaffold) -->
      <section class="bg-surface-1 border border-border rounded-xl p-5">
        <h3 class="text-sm font-semibold text-fg mb-2">🛠️ 볼트 업데이트</h3>
        <p class="text-xs text-fg-muted mb-4">
          앱 릴리즈마다 포함되는 <span class="font-mono">.claude/</span> 템플릿(훅·스킬·settings)을
          이 볼트에 반영합니다. 사용자 자산(<span class="font-mono">_moc/</span>,
          <span class="font-mono">_templates/</span>, 노트 파일)은 건드리지 않습니다.
        </p>

        <div class="space-y-2 mb-4">
          <label class="flex items-start gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="rescaffoldMode"
              value="add-missing"
              bind:group={rescaffoldMode}
              onchange={onModeChange}
              class="mt-0.5"
            />
            <span class="flex-1">
              <span class="text-fg">누락된 파일만 채우기 (안전)</span>
              <span class="block text-xs text-fg-muted">
                없는 파일만 생성합니다. 기존 파일은 내용이 달라도 그대로 둡니다.
              </span>
            </span>
          </label>
          <label class="flex items-start gap-2 text-sm cursor-pointer">
            <input
              type="radio"
              name="rescaffoldMode"
              value="force-claude"
              bind:group={rescaffoldMode}
              onchange={onModeChange}
              class="mt-0.5"
            />
            <span class="flex-1">
              <span class="text-warning">⚠️ <span class="font-mono">.claude/</span> 강제 업데이트</span>
              <span class="block text-xs text-fg-muted">
                <span class="font-mono">.claude/</span> 하위에서 템플릿과 다른 파일을 덮어씁니다.
                사용자가 수정한 파일은 적용 전에 미리보기로 확인할 수 있습니다.
              </span>
            </span>
          </label>
        </div>

        <div class="flex items-center gap-2 mb-3">
          <button
            class="text-sm px-4 py-1.5 rounded border border-border text-fg
                   hover:border-accent transition-colors
                   disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={previewRescaffold}
            disabled={rescaffoldBusy}
          >
            {rescaffoldBusy && !rescaffoldReport ? "확인 중..." : "미리보기"}
          </button>
          <button
            class="text-sm px-4 py-1.5 rounded bg-accent text-fg
                   hover:bg-accent/80 transition-colors
                   disabled:opacity-40 disabled:cursor-not-allowed"
            onclick={applyRescaffold}
            disabled={rescaffoldBusy || !rescaffoldReport || !rescaffoldReport.dry_run}
            title={!rescaffoldReport ? "먼저 미리보기를 실행하세요" : ""}
          >
            {rescaffoldBusy && rescaffoldReport ? "적용 중..." : "적용"}
          </button>
        </div>

        {#if rescaffoldReport}
          <div class="bg-surface-0 border border-border rounded-lg p-4 text-sm">
            <div class="flex items-center justify-between mb-2">
              <span class="text-fg font-medium">
                {rescaffoldReport.dry_run ? "미리보기" : "적용 결과"}
              </span>
              <span class="text-xs text-fg-muted">
                모드: <span class="font-mono">{rescaffoldMode}</span>
              </span>
            </div>

            <ul class="space-y-1 text-xs">
              <li>
                <span class="text-success">✨ 새로 {rescaffoldReport.dry_run ? "생성될" : "생성된"} 파일</span>:
                <span class="text-fg">{rescaffoldReport.created.length}개</span>
              </li>
              {#if rescaffoldReport.created.length > 0}
                <li>
                  <details class="ml-4">
                    <summary class="cursor-pointer text-fg-muted hover:text-fg">목록 보기</summary>
                    <ul class="mt-1 font-mono text-fg-muted space-y-0.5">
                      {#each rescaffoldReport.created as p}
                        <li>{p}</li>
                      {/each}
                    </ul>
                  </details>
                </li>
              {/if}
              <li>
                <span class={rescaffoldReport.overwritten.length > 0 ? "text-warning" : "text-fg-muted"}>
                  ⚠️ {rescaffoldReport.dry_run ? "덮어쓸" : "덮어쓴"} 파일
                </span>:
                <span class="text-fg">{rescaffoldReport.overwritten.length}개</span>
                {#if rescaffoldReport.modified_by_user.length > 0}
                  <span class="text-danger ml-1">
                    (사용자 수정분 {rescaffoldReport.modified_by_user.length}개 포함)
                  </span>
                {/if}
              </li>
              {#if rescaffoldReport.overwritten.length > 0}
                <li>
                  <details class="ml-4">
                    <summary class="cursor-pointer text-fg-muted hover:text-fg">목록 보기</summary>
                    <ul class="mt-1 font-mono text-fg-muted space-y-0.5">
                      {#each rescaffoldReport.overwritten as p}
                        <li class={rescaffoldReport.modified_by_user.includes(p) ? "text-danger" : ""}>
                          {rescaffoldReport.modified_by_user.includes(p) ? "⚠️ " : ""}{p}
                        </li>
                      {/each}
                    </ul>
                  </details>
                </li>
              {/if}
              <li>
                <span class="text-fg-muted">변경 없음</span>:
                <span class="text-fg">{rescaffoldReport.unchanged}개</span>
              </li>
            </ul>
          </div>
        {/if}

        {#if rescaffoldMessage}
          <p class="mt-3 text-sm text-success">✓ {rescaffoldMessage}</p>
        {/if}
        {#if rescaffoldError}
          <p class="mt-3 text-sm text-danger">{rescaffoldError}</p>
        {/if}
      </section>
    </div>
  {/if}
</div>
