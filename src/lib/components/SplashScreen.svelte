<script lang="ts">
  import { loadDockSettings } from "$lib/ipc/config";
  import type { AppConfig } from "$lib/types/config";
  import type { DockSettings } from "$lib/types/dock";

  interface Props {
    onDone: (result: {
      hasConfig: boolean;
      config?: AppConfig;
      settings?: DockSettings;
    }) => void;
  }

  let { onDone }: Props = $props();
  let visible = $state(true);

  $effect(() => {
    const timer = setTimeout(async () => {
      try {
        const result = await loadDockSettings();
        visible = false;
        setTimeout(() => {
          if (result.config) {
            onDone({
              hasConfig: true,
              config: result.config,
              settings: result.settings,
            });
          } else {
            onDone({ hasConfig: false, settings: result.settings });
          }
        }, 500);
      } catch {
        visible = false;
        setTimeout(() => onDone({ hasConfig: false }), 500);
      }
    }, 1500);

    return () => clearTimeout(timer);
  });
</script>

<div
  class="fixed inset-0 flex items-center justify-center transition-opacity duration-500 splash-bg"
  class:opacity-100={visible}
  class:opacity-0={!visible}
>
  <div class="flex flex-col items-center gap-8 animate-fade-in">
    <img
      src="/logo.png"
      alt="reeln dock"
      class="splash-logo"
    />
    <div class="splash-spinner"></div>
  </div>
</div>

<style>
  .splash-bg {
    background: radial-gradient(ellipse at 50% 40%, #162a3a 0%, #0e1a22 60%, #080f14 100%);
  }

  .splash-logo {
    width: 320px;
    height: 320px;
    filter: drop-shadow(0 8px 32px rgba(30, 136, 200, 0.25));
  }

  .splash-spinner {
    width: 28px;
    height: 28px;
    border: 2px solid rgba(30, 136, 200, 0.3);
    border-top-color: #1E88C8;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  @keyframes fade-in {
    from { opacity: 0; transform: scale(0.92) translateY(8px); }
    to { opacity: 1; transform: scale(1) translateY(0); }
  }

  .animate-fade-in {
    animation: fade-in 0.6s ease-out;
  }
</style>
