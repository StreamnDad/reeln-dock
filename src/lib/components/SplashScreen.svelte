<script lang="ts">
  import { loadDockSettings } from "$lib/ipc/config";
  import { initCliStatus } from "$lib/stores/cli.svelte";
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
        // Load dock settings and detect CLI in parallel
        const [settingsResult] = await Promise.allSettled([
          loadDockSettings(),
          initCliStatus(),
        ]);

        const result = settingsResult.status === "fulfilled" ? settingsResult.value : null;
        visible = false;
        setTimeout(() => {
          if (result?.config) {
            onDone({
              hasConfig: true,
              config: result.config,
              settings: result.settings,
            });
          } else {
            onDone({ hasConfig: false, settings: result?.settings });
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
  class="fixed inset-0 flex items-center justify-center splash-bg"
  class:splash-visible={visible}
  class:splash-hidden={!visible}
>
  <div class="flex flex-col items-center gap-8 splash-content">
    <div class="splash-logo-wrap">
      <img
        src="/logo.png"
        alt="reeln dock"
        class="splash-logo"
      />
    </div>
    <div class="splash-spinner"></div>
  </div>
</div>

<style>
  .splash-bg {
    background: radial-gradient(ellipse at 50% 40%, #162a3a 0%, #0e1a22 60%, #080f14 100%);
    transition: opacity 0.8s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .splash-visible {
    opacity: 1;
  }

  .splash-hidden {
    opacity: 0;
  }

  .splash-content {
    animation: content-enter 1s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  .splash-logo-wrap {
    position: relative;
  }

  .splash-logo-wrap::before {
    content: '';
    position: absolute;
    inset: -20%;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(30, 136, 200, 0.15) 0%, transparent 70%);
    animation: glow-pulse 2.5s ease-in-out infinite;
  }

  .splash-logo {
    position: relative;
    width: 280px;
    height: 280px;
    border-radius: 24px;
    filter: drop-shadow(0 4px 24px rgba(30, 136, 200, 0.3));
    animation: logo-enter 1.2s cubic-bezier(0.16, 1, 0.3, 1) forwards;
  }

  .splash-spinner {
    width: 24px;
    height: 24px;
    border: 2px solid rgba(30, 136, 200, 0.2);
    border-top-color: #1E88C8;
    border-radius: 50%;
    animation: spin 0.8s linear infinite, spinner-fade-in 0.8s ease-out 0.6s both;
  }

  @keyframes content-enter {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes logo-enter {
    from {
      opacity: 0;
      transform: scale(0.85);
      filter: drop-shadow(0 4px 24px rgba(30, 136, 200, 0));
    }
    to {
      opacity: 1;
      transform: scale(1);
      filter: drop-shadow(0 4px 24px rgba(30, 136, 200, 0.3));
    }
  }

  @keyframes glow-pulse {
    0%, 100% { opacity: 0.6; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.05); }
  }

  @keyframes spinner-fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
