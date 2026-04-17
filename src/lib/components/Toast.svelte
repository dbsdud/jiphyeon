<script lang="ts">
  import type { NotificationLevel } from "$lib/types";

  interface Props {
    message: string;
    type?: NotificationLevel;
    visible: boolean;
    onclose: () => void;
  }

  let { message, type = "success", visible, onclose }: Props = $props();

  $effect(() => {
    if (visible) {
      const timer = setTimeout(onclose, 4000);
      return () => clearTimeout(timer);
    }
  });

  const BG_CLASSES: Record<NotificationLevel, string> = {
    success: "bg-success/90 text-fg",
    error: "bg-danger/90 text-fg",
    warn: "bg-warning/90 text-fg",
    info: "bg-surface-2/95 text-fg",
  };
</script>

{#if visible}
  <div
    class="fixed bottom-4 right-4 z-50 px-4 py-3 rounded-lg shadow-lg text-sm flex items-center gap-2 animate-slide-up {BG_CLASSES[type]}"
  >
    <span>{message}</span>
    <button class="text-fg/70 hover:text-fg text-xs ml-2" onclick={onclose}>x</button>
  </div>
{/if}

<style>
  @keyframes slide-up {
    from { transform: translateY(20px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
  .animate-slide-up {
    animation: slide-up 0.2s ease-out;
  }
</style>
