<script lang="ts">
  import SplashScreen from "$lib/components/SplashScreen.svelte";
  import SetupWizard from "$lib/components/SetupWizard.svelte";
  import MainLayout from "$lib/components/layout/MainLayout.svelte";
  import { setConfig, setDockSettings } from "$lib/stores/config.svelte";
  import type { AppConfig } from "$lib/types/config";
  import type { DockSettings } from "$lib/types/dock";

  type AppPhase = "splash" | "setup" | "main";

  let phase = $state<AppPhase>("splash");

  function handleSplashDone(result: {
    hasConfig: boolean;
    config?: AppConfig;
    settings?: DockSettings;
  }) {
    if (result.settings) setDockSettings(result.settings);
    if (result.hasConfig && result.config) {
      setConfig(result.config);
      phase = "main";
    } else {
      phase = "setup";
    }
  }

  function handleSetupDone(config: AppConfig, settings: DockSettings) {
    setConfig(config);
    setDockSettings(settings);
    phase = "main";
  }
</script>

{#if phase === "splash"}
  <SplashScreen onDone={handleSplashDone} />
{:else if phase === "setup"}
  <SetupWizard onDone={handleSetupDone} />
{:else}
  <MainLayout />
{/if}
