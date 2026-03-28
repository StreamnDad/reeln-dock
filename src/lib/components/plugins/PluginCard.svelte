<script lang="ts">
  import type { PluginDetail, RegistryPlugin } from "$lib/types/plugin";
  import { togglePlugin, updatePluginSettings } from "$lib/stores/plugins.svelte";

  interface Props {
    name: string;
    status: "enabled" | "disabled" | "available";
    detail?: PluginDetail;
    registryInfo?: RegistryPlugin;
    onAdd: () => void;
    onRemove: () => void;
  }

  let { name, status, detail, registryInfo, onAdd, onRemove }: Props = $props();

  let showSettings = $state(false);
  let editing = $state(false);
  let editValues = $state<Record<string, string>>({});
  let saving = $state(false);
  let confirmRemove = $state(false);
  let adding = $state(false);

  function startEditing() {
    if (!detail) return;
    const vals: Record<string, string> = {};
    for (const [key, val] of Object.entries(detail.settings)) {
      vals[key] = typeof val === "string" ? val : JSON.stringify(val);
    }
    editValues = vals;
    editing = true;
  }

  function cancelEditing() {
    editing = false;
    editValues = {};
  }

  async function handleSave() {
    saving = true;
    const parsed: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(editValues)) {
      try {
        parsed[key] = JSON.parse(val);
      } catch {
        parsed[key] = val;
      }
    }
    await updatePluginSettings(name, parsed);
    editing = false;
    saving = false;
  }

  async function handleToggle() {
    await togglePlugin(name);
  }

  async function handleAdd() {
    adding = true;
    await onAdd();
    adding = false;
  }

  function handleRemoveClick() {
    if (confirmRemove) {
      onRemove();
      confirmRemove = false;
    } else {
      confirmRemove = true;
      setTimeout(() => (confirmRemove = false), 3000);
    }
  }

  function isSecretKey(key: string): boolean {
    const lower = key.toLowerCase();
    return (
      lower.includes("api_key") ||
      lower.includes("secret") ||
      lower.includes("token") ||
      lower.includes("password")
    );
  }

  function formatValue(val: unknown): string {
    if (typeof val === "string") return val;
    if (typeof val === "boolean") return val ? "true" : "false";
    if (typeof val === "number") return String(val);
    return JSON.stringify(val);
  }

  let settingsEntries = $derived(
    detail ? Object.entries(detail.settings) : [],
  );
  let hasSettings = $derived(settingsEntries.length > 0);
  let isConfigured = $derived(status !== "available");

  let borderClass = $derived(
    status === "enabled"
      ? "border-secondary/40"
      : status === "disabled"
        ? "border-border"
        : "border-border/40 border-dashed",
  );
</script>

<div class="bg-surface rounded-lg border overflow-hidden {borderClass}">
  <div class="flex items-center gap-3 p-4">
    <!-- Plugin name + description -->
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-2">
        <h3 class="font-medium">{name}</h3>
        {#if status === "available"}
          <span class="px-2 py-0.5 rounded-full bg-bg text-text-muted text-xs">not configured</span>
        {:else if status === "disabled"}
          <span class="px-2 py-0.5 rounded-full bg-bg text-text-muted text-xs">disabled</span>
        {/if}
      </div>
      {#if registryInfo}
        <p class="text-xs text-text-muted mt-0.5 truncate" title={registryInfo.description}>
          {registryInfo.description}
        </p>
      {/if}
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-3 shrink-0">
      {#if isConfigured}
        <!-- Toggle switch -->
        <button
          class="relative w-10 h-5 rounded-full transition-colors cursor-pointer"
          class:bg-secondary={status === "enabled"}
          class:bg-border={status !== "enabled"}
          onclick={handleToggle}
          title={status === "enabled" ? "Disable" : "Enable"}
        >
          <div
            class="absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform"
            class:translate-x-5={status === "enabled"}
            class:translate-x-0.5={status !== "enabled"}
          ></div>
        </button>

        {#if hasSettings}
          <button
            class="px-3 py-1 text-xs border rounded transition-colors"
            class:border-secondary={showSettings}
            class:text-secondary={showSettings}
            class:border-border={!showSettings}
            class:text-text-muted={!showSettings}
            class:hover:border-text-muted={!showSettings}
            class:hover:text-text={!showSettings}
            onclick={() => (showSettings = !showSettings)}
          >
            Settings
          </button>
        {/if}

        <!-- Remove button -->
        <button
          class="px-2 py-1 text-xs rounded transition-colors {confirmRemove
            ? 'bg-accent text-white'
            : 'text-text-muted hover:text-accent border border-transparent hover:border-accent/30'}"
          onclick={handleRemoveClick}
          title="Remove from profile"
        >
          {confirmRemove ? "Confirm?" : "Remove"}
        </button>
      {:else}
        <!-- Add button -->
        <button
          class="px-3 py-1.5 text-xs font-medium border border-secondary text-secondary hover:bg-secondary hover:text-text rounded-lg transition-colors disabled:opacity-50"
          disabled={adding}
          onclick={handleAdd}
        >
          {adding ? "Adding..." : "+ Add"}
        </button>
      {/if}
    </div>
  </div>

  <!-- Capabilities -->
  {#if registryInfo && registryInfo.capabilities.length > 0}
    <div class="px-4 pb-2 flex flex-wrap gap-1">
      {#each registryInfo.capabilities as cap}
        <span class="px-1.5 py-0.5 rounded bg-bg text-text-muted text-[10px] font-mono">
          {cap.replace("hook:", "")}
        </span>
      {/each}
    </div>
  {/if}

  <!-- Registry meta for available plugins -->
  {#if !isConfigured && registryInfo}
    <div class="px-4 pb-2 flex items-center gap-3 text-[10px] text-text-muted">
      <span>{registryInfo.package}</span>
      <span>{registryInfo.author}</span>
      <span>{registryInfo.license}</span>
      <span>v{registryInfo.min_reeln_version}+</span>
    </div>
  {/if}

  <!-- Settings panel -->
  {#if showSettings && hasSettings && detail}
    <div class="border-t border-border p-4 bg-bg/50">
      {#if editing}
        <div class="space-y-3">
          {#each Object.entries(editValues) as [key, _] (key)}
            <div>
              <label class="block text-xs text-text-muted mb-1" for="plugin-{name}-{key}">
                {key}
              </label>
              <input
                id="plugin-{name}-{key}"
                type={isSecretKey(key) ? "password" : "text"}
                bind:value={editValues[key]}
                class="w-full px-3 py-1.5 bg-bg border border-border rounded text-sm text-text font-mono focus:outline-none focus:border-secondary"
              />
            </div>
          {/each}
          <div class="flex gap-2 pt-1">
            <button
              class="px-4 py-2 bg-primary hover:bg-primary-light text-text rounded-lg text-sm transition-colors disabled:opacity-50"
              disabled={saving}
              onclick={handleSave}
            >
              {saving ? "Saving..." : "Save"}
            </button>
            <button
              class="px-4 py-2 text-text-muted hover:text-text text-sm transition-colors"
              onclick={cancelEditing}
            >
              Cancel
            </button>
          </div>
        </div>
      {:else}
        <div class="space-y-1.5 text-sm">
          {#each settingsEntries as [key, val] (key)}
            <div class="flex items-baseline gap-2 font-mono">
              <span class="text-text-muted text-xs shrink-0">{key}</span>
              <span class="text-xs truncate" title={formatValue(val)}>
                {#if isSecretKey(key) && typeof val === "string" && val.length > 0}
                  ••••••••
                {:else}
                  {formatValue(val)}
                {/if}
              </span>
            </div>
          {/each}
        </div>
        <button
          class="mt-3 px-4 py-2 text-xs border border-border text-text-muted hover:text-text hover:border-secondary rounded-lg transition-colors"
          onclick={startEditing}
        >
          Edit Settings
        </button>
      {/if}
    </div>
  {/if}
</div>
