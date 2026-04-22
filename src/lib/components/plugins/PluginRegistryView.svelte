<script lang="ts">
  import {
    getRegistry,
    isRegistryLoading,
    getRegistryError,
    loadRegistry,
    getVersionInfo_,
    loadVersionInfo,
  } from "$lib/stores/plugins.svelte";
  import { isPluginInstalled, isCliAvailable, refreshCliStatus } from "$lib/stores/cli.svelte";
  import { installPluginViaCli } from "$lib/ipc/plugins";
  import { help } from "$lib/help";
  import HelpLink from "$lib/components/HelpLink.svelte";
  import { log } from "$lib/stores/log.svelte";
  import { getPluginUpdate } from "$lib/stores/updates.svelte";
  import { open } from "@tauri-apps/plugin-shell";

  let registryPlugins = $derived(getRegistry());
  let regLoading = $derived(isRegistryLoading());
  let regError = $derived(getRegistryError());
  let version = $derived(getVersionInfo_());

  // Expanded detail panel per plugin
  let expandedPlugin = $state<string | null>(null);

  // Install state per plugin
  let installingPlugin = $state<string | null>(null);
  let installError = $state<string | null>(null);

  $effect(() => {
    loadRegistry();
    loadVersionInfo();
  });

  function toggleExpand(name: string) {
    expandedPlugin = expandedPlugin === name ? null : name;
  }

  async function handleInstall(name: string) {
    installingPlugin = name;
    installError = null;
    try {
      const result = await installPluginViaCli(name);
      if (result.success) {
        log.info("Registry", `Installed ${name}`);
        await refreshCliStatus();
      } else {
        installError = result.output || "Installation failed";
        log.error("Registry", `Failed to install ${name}: ${result.output}`);
      }
    } catch (err) {
      installError = String(err);
      log.error("Registry", `Failed to install ${name}: ${err}`);
    } finally {
      installingPlugin = null;
    }
  }

  /** Group capabilities by hook lifecycle phase. */
  function hookPhase(cap: string): string {
    const hook = cap.replace("hook:", "");
    if (hook.startsWith("ON_GAME_INIT")) return "Init";
    if (hook.startsWith("ON_GAME_READY")) return "Ready";
    if (hook.startsWith("ON_GAME_FINISH") || hook.startsWith("ON_POST_GAME_FINISH")) return "Finish";
    if (hook.startsWith("ON_HIGHLIGHTS")) return "Highlights";
    if (hook.startsWith("POST_RENDER")) return "Post-Render";
    if (hook.startsWith("ON_FRAMES")) return "Frames";
    return hook;
  }
</script>

<div>
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-lg font-bold">Plugin Registry <HelpLink text={help["plugins.registry"].text} url={help["plugins.registry"].url} /></h2>
      <p class="text-xs text-text-muted mt-0.5">
        Available plugins for the reeln ecosystem
        {#if version}
          &middot; dock v{version.app_version}
        {/if}
      </p>
    </div>
    <button
      class="px-3 py-1.5 text-xs border border-border text-text-muted hover:text-text hover:border-secondary rounded-lg transition-colors"
      onclick={() => loadRegistry()}
    >
      Refresh
    </button>
  </div>

  {#if regLoading}
    <div class="text-text-muted text-center py-12">
      <p class="text-sm">Loading registry...</p>
    </div>
  {:else if regError}
    <div class="text-center py-12">
      <p class="text-sm text-accent mb-2">{regError}</p>
      <button
        class="px-3 py-1.5 text-xs border border-border text-text-muted hover:text-text rounded transition-colors"
        onclick={() => loadRegistry()}
      >
        Retry
      </button>
    </div>
  {:else if registryPlugins.length === 0}
    <div class="text-text-muted text-center py-12">
      <p class="text-sm">No plugins in registry.</p>
    </div>
  {:else}
    <div class="space-y-4">
      {#each registryPlugins as plugin (plugin.name)}
        {@const expanded = expandedPlugin === plugin.name}
        <div class="bg-surface rounded-lg border border-border overflow-hidden">
          <!-- Header -->
          <button
            class="w-full text-left p-4 hover:bg-bg/50 transition-colors"
            onclick={() => toggleExpand(plugin.name)}
          >
            <div class="flex items-center gap-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <h3 class="font-semibold">{plugin.name}</h3>
                  <span class="px-1.5 py-0.5 rounded bg-bg text-text-muted text-[10px] font-mono">
                    {plugin.package}
                  </span>
                  {#if isCliAvailable()}
                    {#if isPluginInstalled(plugin.name)}
                      <span class="px-1.5 py-0.5 rounded-full bg-green-900/60 text-green-300 text-[10px]">installed</span>
                      {@const pluginUpdate = getPluginUpdate(plugin.name)}
                      {#if pluginUpdate}
                        <button
                          class="px-1.5 py-0.5 rounded-full bg-amber-900/60 text-amber-300 text-[10px] hover:bg-amber-800/60 transition-colors"
                          onclick={(e) => { e.stopPropagation(); open(pluginUpdate.release_url); }}
                          title="{pluginUpdate.current} → {pluginUpdate.latest}"
                        >update available</button>
                      {/if}
                    {:else}
                      <span class="px-1.5 py-0.5 rounded-full bg-bg text-text-muted text-[10px]">not installed</span>
                    {/if}
                  {/if}
                </div>
                <p class="text-sm text-text-muted mt-1">{plugin.description}</p>
              </div>
              <span class="text-text-muted text-xs transition-transform {expanded ? 'rotate-90' : ''}">&#9654;</span>
            </div>

            <!-- Capability badges -->
            <div class="flex flex-wrap gap-1.5 mt-2">
              {#each plugin.capabilities as cap}
                <span class="px-2 py-0.5 rounded-full bg-primary/20 text-text text-[11px] font-mono">
                  {cap.replace("hook:", "")}
                </span>
              {/each}
            </div>
          </button>

          <!-- Expanded detail -->
          {#if expanded}
            <div class="border-t border-border p-4 bg-bg/30">
              <div class="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Details</h4>
                  <dl class="space-y-1.5">
                    <div class="flex gap-2">
                      <dt class="text-text-muted text-xs w-28 shrink-0">Package</dt>
                      <dd class="text-xs font-mono">{plugin.package}</dd>
                    </div>
                    <div class="flex gap-2">
                      <dt class="text-text-muted text-xs w-28 shrink-0">Author</dt>
                      <dd class="text-xs">{plugin.author}</dd>
                    </div>
                    <div class="flex gap-2">
                      <dt class="text-text-muted text-xs w-28 shrink-0">License</dt>
                      <dd class="text-xs">{plugin.license}</dd>
                    </div>
                    <div class="flex gap-2">
                      <dt class="text-text-muted text-xs w-28 shrink-0">Min Version</dt>
                      <dd class="text-xs font-mono">v{plugin.min_reeln_version}</dd>
                    </div>
                    <div class="flex gap-2">
                      <dt class="text-text-muted text-xs w-28 shrink-0">Homepage</dt>
                      <dd class="text-xs">
                        <span class="font-mono text-secondary truncate">{plugin.homepage}</span>
                      </dd>
                    </div>
                  </dl>
                </div>

                <div>
                  <h4 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">Hook Lifecycle</h4>
                  <div class="space-y-1">
                    {#each plugin.capabilities as cap}
                      <div class="flex items-center gap-2">
                        <span class="px-1.5 py-0.5 rounded bg-surface text-[10px] font-mono text-text-muted w-20 text-center shrink-0">
                          {hookPhase(cap)}
                        </span>
                        <span class="text-xs font-mono">{cap.replace("hook:", "")}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              </div>

              <!-- Install / Update action -->
              <div class="mt-4 pt-3 border-t border-border/50 flex items-center gap-3">
                {#if isCliAvailable() && isPluginInstalled(plugin.name)}
                  {@const pUpdate = getPluginUpdate(plugin.name)}
                  {#if pUpdate}
                    <button
                      class="px-3 py-1.5 text-xs font-medium bg-amber-700 hover:bg-amber-600 text-text rounded-lg transition-colors"
                      onclick={() => open(pUpdate.release_url)}
                    >Update {pUpdate.current} &rarr; {pUpdate.latest}</button>
                  {:else}
                    <span class="px-3 py-1.5 text-xs font-medium text-green-300 border border-green-800 rounded-lg">Installed</span>
                  {/if}
                {:else if isCliAvailable()}
                  <button
                    class="px-3 py-1.5 text-xs font-medium bg-primary hover:bg-primary-light text-text rounded-lg transition-colors disabled:opacity-50"
                    disabled={installingPlugin === plugin.name}
                    onclick={() => handleInstall(plugin.name)}
                  >
                    {installingPlugin === plugin.name ? "Installing..." : "Install"}
                  </button>
                  {#if installError && installingPlugin === null && expandedPlugin === plugin.name}
                    <span class="text-xs text-accent">{installError}</span>
                  {/if}
                {:else}
                  <code class="px-3 py-1.5 bg-bg rounded text-xs font-mono text-text select-all">
                    pip install {plugin.package}
                  </code>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
