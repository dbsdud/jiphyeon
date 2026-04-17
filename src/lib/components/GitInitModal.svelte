<script lang="ts">
  interface Props {
    open: boolean;
    vaultPath: string;
    onclose: () => void;
  }

  const { open, vaultPath, onclose }: Props = $props();

  const commands = $derived([
    `cd "${vaultPath}"`,
    "git init",
    "git add .",
    'git commit -m "chore: initialize vault"',
  ]);

  let copied = $state(false);

  async function copyCommands() {
    const text = commands.join("\n");
    await navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1500);
  }
</script>

{#if open}
  <div
    class="fixed inset-0 bg-black/60 z-50 flex items-center justify-center p-6"
    role="dialog"
    aria-modal="true"
    aria-labelledby="git-init-title"
  >
    <div class="bg-surface-1 border border-border rounded-xl p-6 max-w-lg w-full">
      <div class="flex items-start gap-3 mb-4">
        <div class="text-2xl">🌱</div>
        <div>
          <h2 id="git-init-title" class="text-base font-semibold text-fg mb-1">
            볼트가 생성되었습니다
          </h2>
          <p class="text-sm text-fg-muted">
            Git으로 버전 관리를 시작하려면 터미널에서 아래 명령을 실행하세요.
          </p>
        </div>
      </div>

      <div class="bg-surface-0 border border-border rounded-lg p-3 mb-4 font-mono text-xs text-fg">
        {#each commands as cmd}
          <div class="py-0.5">{cmd}</div>
        {/each}
      </div>

      <div class="flex justify-end gap-2">
        <button
          class="text-sm px-3 py-1.5 rounded border border-border text-fg-muted hover:text-fg hover:border-accent transition-colors"
          onclick={copyCommands}
        >
          {copied ? "복사됨 ✓" : "명령 복사"}
        </button>
        <button
          class="text-sm px-3 py-1.5 rounded bg-accent text-fg hover:bg-accent/80 transition-colors"
          onclick={onclose}
        >
          시작하기
        </button>
      </div>
    </div>
  </div>
{/if}
